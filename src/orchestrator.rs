use crate::common::{rrc, DeviceId, MonoSample, Rrc, Ww, MONO_SAMPLE_SILENCE};
use crate::control::{ControlPath, ControlTrip};
use crate::midi::smf_reader::MidiSmfReader;
use crate::midi::{
    sequencer::MidiSequencer, smf_reader::MidiBus, MidiChannel, MIDI_CHANNEL_RECEIVE_ALL,
};
use crate::patterns::{Pattern, PatternSequencer};
use crate::settings::{song::SongSettings, DeviceSettings};
use crate::traits::{
    IsEffect, IsMidiEffect, MakesControlSink, SinksAudio, SinksMidi, SourcesAudio, SourcesMidi,
    WatchesClock,
};
use crate::{clock::WatchedClock, effects::mixer::Mixer};
use crossbeam::deque::Worker;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, Write};
use std::rc::{Rc, Weak};

#[derive(Debug)]
pub struct Performance {
    pub sample_rate: usize,
    pub worker: Worker<MonoSample>,
}

impl Performance {
    pub fn new_with(sample_rate: usize) -> Self {
        Self {
            sample_rate,
            worker: Worker::<MonoSample>::new_fifo(),
        }
    }
}

/// Orchestrator takes a description of a song and turns it into an in-memory representation that is ready to render to sound.
#[derive(Debug, Default)]
pub struct Orchestrator {
    settings: SongSettings,

    clock: WatchedClock,
    main_mixer: Box<Mixer>, /////////////// // Not Box because get_effect("main-mixer") - can eliminate?
    midi_bus: Rc<RefCell<MidiBus>>, // Not Box because we need Weaks
    midi_sequencer: Ww<MidiSequencer>, // owned by WatchedClock
    pattern_sequencer: Ww<PatternSequencer>, // owned by WatchedClock

    // We don't have owning Vecs for WatchesClock or IsMidiEffect because
    // both of those are owned by WatchedClock.
    audio_sources: Vec<Rrc<dyn SourcesAudio>>,
    effects: Vec<Rrc<dyn IsEffect>>,
    patterns: Vec<Rrc<Pattern>>,
    control_paths: Vec<Rrc<ControlPath>>,

    // These are all Weaks. That means someone else owns them.
    // That someone else might be us (see Rc<RefCell<>> above).
    id_to_clock_watcher: HashMap<DeviceId, Ww<dyn WatchesClock>>,
    id_to_audio_source: HashMap<DeviceId, Ww<dyn SourcesAudio>>,
    id_to_effect: HashMap<DeviceId, Ww<dyn IsEffect>>,
    id_to_midi_effect: HashMap<DeviceId, Ww<dyn IsMidiEffect>>,
    id_to_pattern: HashMap<DeviceId, Ww<Pattern>>,
    id_to_control_path: HashMap<DeviceId, Ww<ControlPath>>,
}

impl Orchestrator {
    pub fn new_with(settings: &SongSettings) -> Self {
        let midi_bus = rrc(MidiBus::new());

        let midi_sequencer = rrc(MidiSequencer::new());
        let sink = Rc::downgrade(&midi_bus);
        midi_sequencer
            .borrow_mut()
            .add_midi_sink(MIDI_CHANNEL_RECEIVE_ALL, sink);

        let pattern_sequencer = rrc(PatternSequencer::new(&settings.clock.time_signature()));
        let sink = Rc::downgrade(&midi_bus);
        pattern_sequencer
            .borrow_mut()
            .add_midi_sink(MIDI_CHANNEL_RECEIVE_ALL, sink);

        let mut r = Self {
            settings: settings.clone(),
            clock: WatchedClock::new_with(&settings.clock),
            main_mixer: Box::new(Mixer::new()),
            midi_bus,
            midi_sequencer: Rc::downgrade(&midi_sequencer),
            pattern_sequencer: Rc::downgrade(&pattern_sequencer),

            audio_sources: Vec::new(),
            effects: Vec::new(),
            patterns: Vec::new(),
            control_paths: Vec::new(),

            id_to_clock_watcher: HashMap::new(),
            id_to_audio_source: HashMap::new(),
            id_to_effect: HashMap::new(),
            id_to_midi_effect: HashMap::new(),

            id_to_pattern: HashMap::new(),
            id_to_control_path: HashMap::new(),
        };

        r.clock.add_watcher(midi_sequencer);
        r.clock.add_watcher(pattern_sequencer);
        r.prepare_from_settings();
        r
    }

    pub fn new() -> Self {
        Self::new_with(&SongSettings::new_defaults())
    }

    pub fn settings(&self) -> &SongSettings {
        &self.settings
    }

    pub fn settings_mut(&mut self) -> &mut SongSettings {
        &mut self.settings
    }

    fn tick(&mut self) -> (MonoSample, bool) {
        if self.clock.visit_watchers() {
            return (MONO_SAMPLE_SILENCE, true);
        }
        let sample = self.main_mixer.source_audio(self.clock.inner_clock());
        self.clock.tick();
        (sample, false)
    }

    pub fn perform(&mut self) -> anyhow::Result<Performance> {
        let sample_rate = self.clock.inner_clock().settings().sample_rate();
        let performance = Performance::new_with(sample_rate);
        let progress_indicator_quantum: usize = sample_rate / 2;
        let mut next_progress_indicator: usize = progress_indicator_quantum;
        self.clock.reset();
        loop {
            let (sample, done) = self.tick();
            performance.worker.push(sample);
            if next_progress_indicator <= self.clock.inner_clock().samples() {
                print!(".");
                io::stdout().flush().unwrap();
                next_progress_indicator += progress_indicator_quantum;
            }
            if done {
                break;
            }
        }
        println!();
        Ok(performance)
    }

    pub fn add_main_mixer_source(&mut self, device: Ww<dyn SourcesAudio>) {
        self.main_mixer.add_audio_source(device);
    }

    fn prepare_from_settings(&mut self) {
        self.create_devices_from_settings();
        self.create_patch_cables_from_settings();
        self.create_tracks_from_settings();
        self.create_control_trips_from_settings();
    }

    pub fn add_audio_source_by_id(
        &mut self,
        id: String,
        audio_source: Rc<RefCell<dyn SourcesAudio>>,
    ) {
        self.id_to_audio_source
            .insert(id, Rc::downgrade(&audio_source));
        self.audio_sources.push(audio_source);
    }

    fn add_effect_by_id(&mut self, id: String, effect: Rc<RefCell<dyn IsEffect>>) {
        self.id_to_effect.insert(id, Rc::downgrade(&effect));
        self.effects.push(effect);
    }

    pub fn add_midi_effect_by_id(
        &mut self,
        id: String,
        midi_effect: Rc<RefCell<dyn IsMidiEffect>>,
        channel: MidiChannel,
    ) {
        let instrument = Rc::downgrade(&midi_effect);
        self.connect_to_downstream_midi_bus(channel, instrument);
        let instrument = Rc::clone(&midi_effect);
        self.connect_to_upstream_midi_bus(instrument);

        self.id_to_midi_effect
            .insert(id, Rc::downgrade(&midi_effect));
        self.clock.add_watcher(midi_effect);
    }

    pub fn connect_to_downstream_midi_bus(
        &mut self,
        channel: MidiChannel,
        instrument: Ww<dyn SinksMidi>,
    ) {
        self.midi_bus
            .borrow_mut()
            .add_midi_sink(channel, instrument);
    }

    pub fn connect_to_upstream_midi_bus(&mut self, instrument: Rc<RefCell<dyn SourcesMidi>>) {
        let sink = Rc::downgrade(&self.midi_bus);
        instrument
            .borrow_mut()
            .add_midi_sink(MIDI_CHANNEL_RECEIVE_ALL, sink);
    }

    fn add_clock_watcher_by_id(
        &mut self,
        id: String,
        clock_watcher: Rc<RefCell<dyn WatchesClock>>,
    ) {
        // We don't maintain self.clock_watchers because self.clock owns them all.
        let watcher_weak = Rc::downgrade(&clock_watcher);
        self.id_to_clock_watcher.insert(id, watcher_weak);
        self.clock.add_watcher(clock_watcher);
    }

    fn create_devices_from_settings(&mut self) {
        // Then set up instruments, attaching to sequencers as they're set up.
        let sample_rate = self.settings().clock.sample_rate();

        // TODO: grrr, want this to be iter_mut()
        for device in self.settings.devices.clone() {
            match device {
                DeviceSettings::Instrument(id, instrument_settings) => {
                    let instrument = instrument_settings.instantiate(sample_rate);
                    let midi_channel = instrument.borrow().midi_channel();
                    let instrument_weak = Rc::downgrade(&instrument);
                    self.connect_to_downstream_midi_bus(midi_channel, instrument_weak);
                    self.add_audio_source_by_id(id, instrument);
                }
                DeviceSettings::MidiInstrument(id, midi_instrument_settings) => {
                    let midi_instrument = midi_instrument_settings.instantiate(sample_rate);
                    let midi_channel = midi_instrument.borrow().midi_channel();
                    self.add_midi_effect_by_id(id, midi_instrument, midi_channel);
                }
                DeviceSettings::Effect(id, effect_settings) => {
                    self.add_effect_by_id(id, effect_settings.instantiate(sample_rate));
                }
            }
        }
    }

    fn create_patch_cables_from_settings(&mut self) {
        for patch_cable in self.settings.patch_cables.clone() {
            if patch_cable.len() < 2 {
                dbg!("ignoring patch cable of length < 2");
                continue;
            }
            let mut last_device_id: Option<DeviceId> = None;
            for device_id in patch_cable {
                if let Some(ldi) = last_device_id {
                    let output = self.get_audio_source_by_id(&ldi);
                    if device_id == "main-mixer" {
                        self.add_main_mixer_source(output);
                    } else {
                        let input = self.get_audio_sink_by_id(&device_id);
                        if let Some(input) = input.upgrade() {
                            input.borrow_mut().add_audio_source(output);
                        }
                    }
                }
                last_device_id = Some(device_id);
            }
        }
    }

    fn get_audio_source_by_id(&self, id: &str) -> Ww<dyn SourcesAudio> {
        if let Some(item) = self.id_to_audio_source.get(id) {
            let clone = Weak::clone(item);
            return clone;
        }
        if let Some(item) = self.id_to_effect.get(id) {
            let clone = Weak::clone(item);
            return clone;
        }
        panic!("SourcesAudio id {} not found", id);
    }

    fn get_audio_sink_by_id(&self, id: &str) -> Ww<dyn SinksAudio> {
        if id == "main-mixer" {
            panic!("special case this");
        }
        if let Some(item) = self.id_to_effect.get(id) {
            let clone = Weak::clone(item);
            return clone;
        }
        panic!("SinksAudio id {} not found", id);
    }

    fn get_is_controllable_by_id(&self, id: &str) -> Ww<dyn MakesControlSink> {
        if let Some(item) = self.id_to_effect.get(id) {
            let clone = Weak::clone(item);
            return clone;
        }
        panic!("MakesControlSink id {} not found", id);
    }

    fn get_pattern_by_id(&self, id: &str) -> Ww<Pattern> {
        if let Some(item) = self.id_to_pattern.get(id) {
            let clone = Weak::clone(item);
            return clone;
        }
        panic!("Pattern id {} not found", id);
    }

    fn get_control_path_by_id(&self, id: &str) -> Ww<ControlPath> {
        if let Some(item) = self.id_to_control_path.get(id) {
            let clone = Weak::clone(item);
            return clone;
        }
        panic!("ControlPath id {} not found", id);
    }

    fn create_tracks_from_settings(&mut self) {
        if self.settings.tracks.is_empty() {
            return;
        }

        for pattern_settings in self.settings.patterns.clone() {
            let pattern = rrc(Pattern::from_settings(&pattern_settings));
            self.id_to_pattern
                .insert(pattern_settings.id.clone(), Rc::downgrade(&pattern));
            self.patterns.push(pattern);
        }
        // TODO: for now, a track has a single time signature. Each pattern can have its
        // own to override the track's, but that's unwieldy compared to a single signature
        // change as part of the overall track sequence. Maybe a pattern can be either
        // a pattern or a TS change...
        //
        // TODO - should PatternSequencers be able to change their base time signature? Probably

        if let Some(pattern_sequencer) = self.pattern_sequencer.upgrade() {
            for track in self.settings.tracks.clone() {
                let channel = track.midi_channel;
                pattern_sequencer.borrow_mut().reset_cursor();
                for pattern_id in track.pattern_ids {
                    let pattern = self.get_pattern_by_id(&pattern_id);
                    if let Some(pattern) = pattern.upgrade() {
                        pattern_sequencer
                            .borrow_mut()
                            .add_pattern(&pattern.borrow(), channel);
                    }
                }
            }
        }
    }

    fn create_control_trips_from_settings(&mut self) {
        if self.settings.trips.is_empty() {
            // There's no need to instantiate the paths if there are no trips to use them.
            return;
        }

        for path_settings in self.settings.paths.clone() {
            let id_copy = path_settings.id.clone();
            let v = Rc::new(RefCell::new(ControlPath::from_settings(&path_settings)));
            self.id_to_control_path.insert(id_copy, Rc::downgrade(&v));
            self.control_paths.push(v);
        }
        for control_trip_settings in self.settings.trips.clone() {
            let target = self.get_is_controllable_by_id(&control_trip_settings.target.id);
            if let Some(target) = target.upgrade() {
                if let Some(controller) = target
                    .borrow()
                    .make_control_sink(&control_trip_settings.target.param)
                {
                    let control_trip = Rc::new(RefCell::new(ControlTrip::new(controller)));
                    control_trip.borrow_mut().reset_cursor();
                    for path_id in control_trip_settings.path_ids {
                        let control_path = self.get_control_path_by_id(&path_id);
                        if let Some(control_path) = control_path.upgrade() {
                            control_trip.borrow_mut().add_path(&control_path.borrow());
                        }
                    }
                    self.add_clock_watcher_by_id(control_trip_settings.id, control_trip);
                } else {
                    panic!(
                        "someone instantiated a MakesControlSink without proper wrapping: {:?}.",
                        target
                    );
                };
            } else {
                panic!("an upgrade failed. YOU HAD ONE JOB");
            }
        }
    }

    fn main_mixer(&self) -> &Mixer {
        &self.main_mixer
    }

    // TODO: this is kind of a mess. We have an IOHelper method calling back into us.
    // I wanted IOHelper to get the IO garbage out of Orchestrator.
    pub(crate) fn read_midi_data(&mut self, data: &[u8]) {
        if let Some(midi_sequencer) = self.midi_sequencer.upgrade() {
            MidiSmfReader::load_sequencer(&data, &mut midi_sequencer.borrow_mut());
        } else {
            panic!("this shouldn't ever happen");
        }
    }
}
