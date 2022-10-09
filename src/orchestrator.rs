use crate::common::{DeviceId, MonoSample, MONO_SAMPLE_SILENCE};
use crate::midi::sequencer::MidiSequencer;
use crate::midi::smf_reader::MidiBus;
use crate::midi::MidiChannel;
use crate::midi::MIDI_CHANNEL_RECEIVE_ALL;
use crate::settings::song::SongSettings;
use crate::settings::{DeviceSettings, InstrumentSettings};
use crate::{
    clock::WatchedClock,
    effects::{arpeggiator::Arpeggiator, mixer::Mixer},
};

use crate::synthesizers::{drumkit_sampler, welsh};
use crate::traits::{
    IsEffect, IsMidiEffect, SinksAudio, SinksControl, SinksMidi, SourcesAudio, SourcesMidi,
    WatchesClock,
};
use crossbeam::deque::Worker;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, Write};
use std::rc::{Rc, Weak};

use crate::control::{ControlPath, ControlTrip};
use crate::patterns::{Pattern, PatternSequencer};

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
    midi_sequencer: Rc<RefCell<MidiSequencer>>,
    pattern_sequencer: Rc<RefCell<PatternSequencer>>,

    id_to_clock_watcher: HashMap<DeviceId, Rc<RefCell<dyn WatchesClock>>>,
    id_to_audio_source: HashMap<DeviceId, Rc<RefCell<dyn SourcesAudio>>>,
    id_to_effect: HashMap<DeviceId, Rc<RefCell<dyn IsEffect>>>,
    id_to_midi_effect: HashMap<DeviceId, Rc<RefCell<dyn IsMidiEffect>>>,

    id_to_pattern: HashMap<DeviceId, Rc<RefCell<Pattern>>>,
    id_to_control_path: HashMap<DeviceId, Rc<RefCell<ControlPath>>>,
}

impl Orchestrator {
    pub fn new(settings: SongSettings) -> Self {
        let mut r = Self {
            settings: settings.clone(),
            clock: WatchedClock::new_with(&settings.clock),
            main_mixer: Box::new(Mixer::new()),
            midi_bus: Rc::new(RefCell::new(MidiBus::new())),
            midi_sequencer: Rc::new(RefCell::new(MidiSequencer::new())),
            pattern_sequencer: Rc::new(RefCell::new(PatternSequencer::new(
                &settings.clock.time_signature(),
            ))),
            id_to_clock_watcher: HashMap::new(),
            id_to_audio_source: HashMap::new(),
            id_to_effect: HashMap::new(),
            id_to_midi_effect: HashMap::new(),

            id_to_pattern: HashMap::new(),
            id_to_control_path: HashMap::new(),
        };

        let sink = Rc::downgrade(&r.midi_bus);
        r.pattern_sequencer
            .borrow_mut()
            .add_midi_sink(MIDI_CHANNEL_RECEIVE_ALL, sink);
        let sink = Rc::downgrade(&r.midi_bus);
        r.midi_sequencer
            .borrow_mut()
            .add_midi_sink(MIDI_CHANNEL_RECEIVE_ALL, sink);
        let watcher = Rc::clone(&r.midi_sequencer);
        r.clock.add_watcher(watcher);
        let watcher = Rc::clone(&r.pattern_sequencer);
        r.clock.add_watcher(watcher);
        r.prepare_from_settings();
        r
    }

    pub fn new_defaults() -> Self {
        Self::new(SongSettings::new_defaults())
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

    pub fn add_main_mixer_source(&mut self, device: Rc<RefCell<dyn SourcesAudio>>) {
        self.main_mixer.add_audio_source(device);
    }

    fn prepare_from_settings(&mut self) {
        self.create_effects_from_settings();
        self.create_instruments_from_settings();
        self.create_patch_cables_from_settings();
        self.create_tracks_from_settings();
        self.create_control_trips_from_settings();
    }

    pub fn add_instrument_by_id(&mut self, id: String, instrument: Rc<RefCell<dyn SourcesAudio>>) {
        self.id_to_audio_source.insert(id, instrument);
    }

    pub fn connect_to_downstream_midi_bus(
        &mut self,
        channel: MidiChannel,
        instrument: Weak<RefCell<dyn SinksMidi>>,
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

    pub fn add_midi_effect_by_id(
        &mut self,
        id: String,
        midi_effect: Rc<RefCell<dyn IsMidiEffect>>,
        channel: MidiChannel,
    ) {
        let watcher = Rc::clone(&midi_effect);
        self.clock.add_watcher(watcher);
        self.id_to_midi_effect.insert(id, Rc::clone(&midi_effect));
        let instrument = Rc::downgrade(&midi_effect);
        self.connect_to_upstream_midi_bus(midi_effect);
        self.connect_to_downstream_midi_bus(channel, instrument);
    }

    fn add_effect_by_id(&mut self, id: String, instrument: Rc<RefCell<dyn IsEffect>>) {
        self.id_to_effect.insert(id, instrument);
        // TODO: and connect to main mixer? Are other things supposed to be chained to these?
    }

    fn add_clock_watcher_by_id(&mut self, id: String, automator: Rc<RefCell<dyn WatchesClock>>) {
        self.id_to_clock_watcher.insert(id, Rc::clone(&automator));
        self.clock.add_watcher(automator);
        // TODO: assert that it wasn't added twice.
    }

    fn create_instruments_from_settings(&mut self) {
        // Then set up instruments, attaching to sequencers as they're set up.
        for device in self.settings.devices.clone() {
            // TODO: grrr, want this to be iter_mut()
            match device {
                DeviceSettings::Instrument(id, settings) => match settings {
                    InstrumentSettings::Welsh {
                        midi_input_channel,
                        preset_name,
                    } => {
                        let instrument = Rc::new(RefCell::new(welsh::Synth::new(
                            midi_input_channel,
                            self.settings.clock.sample_rate(),
                            welsh::SynthPreset::by_name(&preset_name),
                        )));
                        let instrument_weak = Rc::downgrade(&instrument);
                        self.connect_to_downstream_midi_bus(midi_input_channel, instrument_weak);
                        self.add_instrument_by_id(id, instrument);
                    }
                    InstrumentSettings::Drumkit {
                        midi_input_channel,
                        preset_name: _preset,
                    } => {
                        let instrument = Rc::new(RefCell::new(
                            drumkit_sampler::Sampler::new_from_files(midi_input_channel),
                        ));
                        let instrument_weak = Rc::downgrade(&instrument);
                        self.connect_to_downstream_midi_bus(midi_input_channel, instrument_weak);
                        self.add_instrument_by_id(id, instrument);
                    }
                    InstrumentSettings::Arpeggiator {
                        midi_input_channel,
                        midi_output_channel,
                    } => self.add_midi_effect_by_id(
                        // TODO
                        id,
                        Rc::new(RefCell::new(Arpeggiator::new_with(
                            midi_input_channel,
                            midi_output_channel,
                        ))),
                        midi_input_channel,
                    ),
                },
                DeviceSettings::Effect(_id, _settings) => { // skip
                }
            }
        }
    }

    fn create_effects_from_settings(&mut self) {
        for device in self.settings.devices.clone() {
            if let DeviceSettings::Effect(id, effect_settings) = device {
                self.add_effect_by_id(
                    id,
                    effect_settings.instantiate(self.settings().clock.sample_rate()),
                );
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
                    let output: Rc<RefCell<dyn SourcesAudio>> = self.get_audio_source_by_id(&ldi);
                    if device_id == "main-mixer" {
                        self.add_main_mixer_source(output);
                    } else {
                        let input: Rc<RefCell<dyn SinksAudio>> =
                            self.get_audio_sink_by_id(&device_id);
                        input.borrow_mut().add_audio_source(output);
                    }
                }
                last_device_id = Some(device_id);
            }
        }
    }

    #[allow(dead_code)]
    fn get_instrument_by_id(&self, id: &str) -> Rc<RefCell<dyn SourcesAudio>> {
        if self.id_to_audio_source.contains_key(id) {
            return Rc::clone(self.id_to_audio_source.get(id).unwrap());
        }
        panic!("yo {}", id);
    }

    fn get_audio_source_by_id(&self, id: &str) -> Rc<RefCell<dyn SourcesAudio>> {
        if self.id_to_audio_source.contains_key(id) {
            return Rc::clone(self.id_to_audio_source.get(id).unwrap());
        } else if self.id_to_effect.contains_key(id) {
            let clone = Rc::clone(self.id_to_effect.get(id).unwrap());
            return clone;
        }
        panic!("yo {}", id);
    }

    fn get_audio_sink_by_id(&self, id: &str) -> Rc<RefCell<dyn SinksAudio>> {
        if id == "main-mixer" {
            panic!("special case this");
        }
        if self.id_to_effect.contains_key(id) {
            let clone = Rc::clone(self.id_to_effect.get(id).unwrap());
            return clone;
        }
        panic!("yo {}", id);
    }

    fn get_control_sink_by_id(&self, id: &str) -> Rc<RefCell<dyn SinksControl>> {
        if self.id_to_effect.contains_key(id) {
            let clone = Rc::clone(self.id_to_effect.get(id).unwrap());
            return clone;
        }
        panic!("yo {}", id);
    }

    fn create_tracks_from_settings(&mut self) {
        if self.settings.tracks.is_empty() {
            return;
        }

        for pattern in self.settings.patterns.clone() {
            self.id_to_pattern.insert(
                pattern.id.clone(),
                Rc::new(RefCell::new(Pattern::from_settings(&pattern))),
            );
        }
        // TODO: for now, a track has a single time signature. Each pattern can have its
        // own to override the track's, but that's unwieldy compared to a single signature
        // change as part of the overall track sequence. Maybe a pattern can be either
        // a pattern or a TS change...
        //
        // TODO - should PatternSequencers be able to change their base time signature? Probably

        for track in self.settings.tracks.clone() {
            let channel = track.midi_channel;
            self.pattern_sequencer.borrow_mut().reset_cursor();
            for pattern_id in track.pattern_ids {
                let pattern = self.get_pattern_by_id(&pattern_id);
                self.pattern_sequencer
                    .borrow_mut()
                    .insert_pattern(pattern, channel);
            }
        }
    }

    fn create_control_trips_from_settings(&mut self) {
        if self.settings.trips.is_empty() {
            return;
        }

        for path_settings in self.settings.paths.clone() {
            let id_copy = path_settings.id.clone();
            self.id_to_control_path.insert(
                id_copy,
                Rc::new(RefCell::new(ControlPath::from_settings(&path_settings))),
            );
        }
        for control_trip_settings in self.settings.trips.clone() {
            let target = self.get_control_sink_by_id(&control_trip_settings.target.id);
            let control_trip = Rc::new(RefCell::new(ControlTrip::new(
                target,
                control_trip_settings.target.param,
            )));
            control_trip.borrow_mut().reset_cursor();
            for path_id in control_trip_settings.path_ids {
                let control_path_opt = self.id_to_control_path.get(&path_id);
                // TODO: not sure this clone() is right
                if let Some(control_path) = control_path_opt {
                    control_trip.borrow_mut().add_path(Rc::clone(control_path));
                } else {
                    panic!(
                        "automation track {} needs missing sequence {}",
                        control_trip_settings.id, path_id
                    );
                }
            }
            self.add_clock_watcher_by_id(control_trip_settings.id, control_trip);
        }
    }

    fn get_pattern_by_id(&self, pattern_id: &str) -> Rc<RefCell<Pattern>> {
        Rc::clone(self.id_to_pattern.get(pattern_id).unwrap())
    }

    pub fn midi_sequencer(&self) -> Rc<RefCell<MidiSequencer>> {
        Rc::clone(&self.midi_sequencer)
    }

    pub fn main_mixer(&self) -> &Mixer {
        &self.main_mixer
    }
}
