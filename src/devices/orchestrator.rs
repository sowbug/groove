use crate::common::{
    DeviceId, MidiChannel, MonoSample, MIDI_CHANNEL_RECEIVE_ALL, MONO_SAMPLE_SILENCE,
};
use crate::primitives::bitcrusher::Bitcrusher;
use crate::primitives::clock::WatchedClock;
use crate::primitives::filter::MiniFilter2;
use crate::primitives::gain::MiniGain;
use crate::primitives::limiter::MiniLimiter;
use crate::primitives::mixer::Mixer;
use crate::primitives::{
    IsEffect, IsMidiEffect, SinksAudio, SinksControl, SinksMidi, SourcesAudio, SourcesMidi,
    WatchesClock,
};
use crate::settings::effects::EffectSettings;
use crate::settings::song::SongSettings;
use crate::settings::{DeviceSettings, InstrumentSettings};

use crate::synthesizers::{drumkit_sampler, welsh};
use crossbeam::deque::Worker;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, Write};
use std::rc::{Rc, Weak};

use super::control::{ControlPath, ControlTrip};
use super::midi::MidiBus;
use super::patterns::{Pattern, PatternSequencer};
use super::sequencer::MidiSequencer;
use super::Arpeggiator;

/// Orchestrator takes a description of a song and turns it into an in-memory representation that is ready to render to sound.
#[derive(Default)]
pub struct Orchestrator {
    settings: SongSettings,

    clock: WatchedClock,
    main_mixer: Box<Mixer>, /////////////// // Not Box because get_effect("main-mixer") - can eliminate?
    midi_bus: Rc<RefCell<MidiBus>>, // Not Box because we need Weaks
    midi_sequencer: Rc<RefCell<MidiSequencer>>,
    pattern_sequencer: Rc<RefCell<PatternSequencer>>,

    id_to_controller: HashMap<DeviceId, Rc<RefCell<dyn WatchesClock>>>,
    id_to_instrument: HashMap<DeviceId, Rc<RefCell<dyn SourcesAudio>>>,
    id_to_effect: HashMap<DeviceId, Rc<RefCell<dyn IsEffect>>>,
    id_to_arp: HashMap<DeviceId, Rc<RefCell<dyn IsMidiEffect>>>,

    id_to_pattern: HashMap<DeviceId, Rc<RefCell<Pattern>>>,
    id_to_automation_sequence: HashMap<DeviceId, Rc<RefCell<ControlPath>>>,
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
            id_to_controller: HashMap::new(),
            id_to_instrument: HashMap::new(),
            id_to_effect: HashMap::new(),
            id_to_arp: HashMap::new(),

            id_to_pattern: HashMap::new(),
            id_to_automation_sequence: HashMap::new(),
        };

        let sink = Rc::downgrade(&r.midi_bus);
        r.pattern_sequencer
            .borrow_mut()
            .add_midi_sink(MIDI_CHANNEL_RECEIVE_ALL, sink);
        let sink = Rc::downgrade(&r.midi_bus);
        r.midi_sequencer
            .borrow_mut()
            .add_midi_sink(MIDI_CHANNEL_RECEIVE_ALL, sink);
        r.clock.add_watcher(r.midi_sequencer.clone());
        r.clock.add_watcher(r.pattern_sequencer.clone());
        r.prepare_from_settings();
        r
    }

    pub fn new_defaults() -> Self {
        Self::new(SongSettings::new_defaults())
    }

    pub fn settings(&self) -> &SongSettings {
        &self.settings
    }

    fn tick(&mut self) -> (MonoSample, bool) {
        if self.clock.visit_watchers() {
            return (MONO_SAMPLE_SILENCE, true);
        }
        let sample = self.main_mixer.source_audio(self.clock.inner_clock());
        self.clock.tick();
        (sample, false)
    }

    pub fn perform_to_queue(&mut self, worker: &Worker<MonoSample>) -> anyhow::Result<()> {
        let progress_indicator_quantum: usize =
            self.clock.inner_clock().settings().sample_rate() / 2;
        let mut next_progress_indicator: usize = progress_indicator_quantum;
        loop {
            let (sample, done) = self.tick();
            worker.push(sample);
            if next_progress_indicator <= self.clock.inner_clock().samples {
                print!(".");
                io::stdout().flush().unwrap();
                next_progress_indicator += progress_indicator_quantum;
            }
            if done {
                break;
            }
        }
        println!();
        Ok(())
    }

    pub fn add_master_mixer_source(&mut self, device: Rc<RefCell<dyn SourcesAudio>>) {
        self.main_mixer.add_audio_source(device);
    }

    fn prepare_from_settings(&mut self) {
        self.create_effects_from_settings();
        self.create_instruments_from_settings();
        self.create_patch_cables_from_settings();
        self.create_tracks_from_settings();
        self.create_automation_tracks_from_settings();
    }

    pub fn add_instrument_by_id(&mut self, id: String, instrument: Rc<RefCell<dyn SourcesAudio>>) {
        self.id_to_instrument.insert(id, instrument.clone());
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
        arp: Rc<RefCell<dyn IsMidiEffect>>,
        channel: MidiChannel,
    ) {
        self.clock.add_watcher(arp.clone());
        self.id_to_arp.insert(id, arp.clone());
        let instrument = Rc::downgrade(&arp);
        self.connect_to_downstream_midi_bus(channel, instrument);
        self.connect_to_upstream_midi_bus(arp);
    }

    fn add_effect_by_id(&mut self, id: String, instrument: Rc<RefCell<dyn IsEffect>>) {
        self.id_to_effect.insert(id, instrument.clone());
        // TODO: and connect to main mixer? Are other things supposed to be chained to these?
    }

    fn add_clock_watcher_by_id(&mut self, id: String, automator: Rc<RefCell<dyn WatchesClock>>) {
        self.id_to_controller.insert(id, automator.clone());
        self.clock.add_watcher(automator);
        // TODO: assert that it wasn't added twice.
    }

    fn create_instruments_from_settings(&mut self) {
        // Then set up instruments, attaching to sequencers as they're set up.
        for device in self.settings.devices.clone() {
            match device {
                DeviceSettings::Instrument(settings) => match settings {
                    InstrumentSettings::Welsh {
                        id,
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
                        id,
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
                        id,
                        midi_input_channel,
                        midi_output_channel,
                    } => self.add_midi_effect_by_id(
                        // TODO
                        id,
                        Rc::new(RefCell::new(Arpeggiator::new(
                            midi_input_channel,
                            midi_output_channel,
                        ))),
                        midi_input_channel,
                    ),
                },
                DeviceSettings::Effect(_settings) => { // skip
                }
            }
        }
    }

    fn create_effects_from_settings(&mut self) {
        for device in self.settings.devices.clone() {
            if let DeviceSettings::Effect(effect_settings) = device {
                match effect_settings {
                    // This has more repetition than we'd expect because of
                    // https://stackoverflow.com/questions/26378842/how-do-i-overcome-match-arms-with-incompatible-types-for-structs-implementing-sa
                    //
                    // Match arms have to return the same types, and returning a Rc<RefCell<dyn some trait>> doesn't count
                    // as the same type.
                    EffectSettings::Limiter { id, min, max } => {
                        let device = Rc::new(RefCell::new(MiniLimiter::new_with(
                            min as MonoSample,
                            max as MonoSample,
                        )));
                        self.add_effect_by_id(id, device);
                    }
                    EffectSettings::Gain { id, amount } => {
                        let device = Rc::new(RefCell::new(MiniGain::new_with(amount)));
                        self.add_effect_by_id(id, device);
                    }
                    EffectSettings::Bitcrusher { id, bits_to_crush } => {
                        let device = Rc::new(RefCell::new(Bitcrusher::new_with(bits_to_crush)));
                        self.add_effect_by_id(id, device);
                    }
                    EffectSettings::FilterLowPass12db { id, cutoff, q } => {
                        let device = Rc::new(RefCell::new(MiniFilter2::new(
                            &crate::primitives::filter::MiniFilter2Type::LowPass {
                                sample_rate: self.settings().clock.sample_rate(),
                                cutoff,
                                q,
                            },
                        )));
                        self.add_effect_by_id(id, device);
                    }
                    EffectSettings::FilterHighPass12db { id, cutoff, q } => {
                        let device = Rc::new(RefCell::new(MiniFilter2::new(
                            &crate::primitives::filter::MiniFilter2Type::HighPass {
                                sample_rate: self.settings().clock.sample_rate(),
                                cutoff,
                                q,
                            },
                        )));
                        self.add_effect_by_id(id, device);
                    }
                    EffectSettings::FilterBandPass12db {
                        id,
                        cutoff,
                        bandwidth,
                    } => {
                        let device = Rc::new(RefCell::new(MiniFilter2::new(
                            &crate::primitives::filter::MiniFilter2Type::BandPass {
                                sample_rate: self.settings().clock.sample_rate(),
                                cutoff,
                                bandwidth,
                            },
                        )));
                        self.add_effect_by_id(id, device);
                    }
                    EffectSettings::FilterBandStop12db {
                        id,
                        cutoff,
                        bandwidth,
                    } => {
                        let device = Rc::new(RefCell::new(MiniFilter2::new(
                            &crate::primitives::filter::MiniFilter2Type::BandStop {
                                sample_rate: self.settings().clock.sample_rate(),
                                cutoff,
                                bandwidth,
                            },
                        )));
                        self.add_effect_by_id(id, device);
                    }
                    EffectSettings::FilterAllPass12db { id, cutoff, q } => {
                        let device = Rc::new(RefCell::new(MiniFilter2::new(
                            &crate::primitives::filter::MiniFilter2Type::AllPass {
                                sample_rate: self.settings().clock.sample_rate(),
                                cutoff,
                                q,
                            },
                        )));
                        self.add_effect_by_id(id, device);
                    }
                    EffectSettings::FilterPeakingEq12db {
                        id,
                        cutoff,
                        db_gain,
                    } => {
                        let device = Rc::new(RefCell::new(MiniFilter2::new(
                            &crate::primitives::filter::MiniFilter2Type::PeakingEq {
                                sample_rate: self.settings().clock.sample_rate(),
                                cutoff,
                                db_gain,
                            },
                        )));
                        self.add_effect_by_id(id, device);
                    }
                    EffectSettings::FilterLowShelf12db {
                        id,
                        cutoff,
                        db_gain,
                    } => {
                        let device = Rc::new(RefCell::new(MiniFilter2::new(
                            &crate::primitives::filter::MiniFilter2Type::LowShelf {
                                sample_rate: self.settings().clock.sample_rate(),
                                cutoff,
                                db_gain,
                            },
                        )));
                        self.add_effect_by_id(id, device);
                    }
                    EffectSettings::FilterHighShelf12db {
                        id,
                        cutoff,
                        db_gain,
                    } => {
                        let device = Rc::new(RefCell::new(MiniFilter2::new(
                            &crate::primitives::filter::MiniFilter2Type::HighShelf {
                                sample_rate: self.settings().clock.sample_rate(),
                                cutoff,
                                db_gain,
                            },
                        )));
                        self.add_effect_by_id(id, device);
                    }
                };
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
                        self.add_master_mixer_source(output);
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
        if self.id_to_instrument.contains_key(id) {
            return (self.id_to_instrument.get(id).unwrap()).clone();
        }
        panic!("yo {}", id);
    }

    fn get_audio_source_by_id(&self, id: &str) -> Rc<RefCell<dyn SourcesAudio>> {
        if self.id_to_instrument.contains_key(id) {
            return (self.id_to_instrument.get(id).unwrap()).clone();
        } else if self.id_to_effect.contains_key(id) {
            return (self.id_to_effect.get(id).unwrap()).clone();
        }
        panic!("yo {}", id);
    }

    fn get_audio_sink_by_id(&self, id: &str) -> Rc<RefCell<dyn SinksAudio>> {
        if id == "main-mixer" {
            panic!("special case this");
        }
        if self.id_to_effect.contains_key(id) {
            return (self.id_to_effect.get(id).unwrap()).clone();
        }
        panic!("yo {}", id);
    }

    fn get_control_sink_by_id(&self, id: &str) -> Rc<RefCell<dyn SinksControl>> {
        if self.id_to_effect.contains_key(id) {
            return (self.id_to_effect.get(id).unwrap()).clone();
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
                    .insert_pattern(pattern.clone(), channel);
            }
        }
    }

    fn create_automation_tracks_from_settings(&mut self) {
        if self.settings.trips.is_empty() {
            return;
        }

        for sequence in self.settings.paths.clone() {
            self.id_to_automation_sequence.insert(
                sequence.id.clone(),
                Rc::new(RefCell::new(ControlPath::from_settings(&sequence))),
            );
        }
        for track_settings in self.settings.trips.clone() {
            let target = self.get_control_sink_by_id(&track_settings.target.id);
            let automation_track = Rc::new(RefCell::new(ControlTrip::new(
                target.clone(),
                track_settings.target.param,
            )));
            automation_track.borrow_mut().reset_cursor();
            for pattern_id in track_settings.path_ids {
                let pattern_opt = self.id_to_automation_sequence.get(&pattern_id);
                if let Some(pattern) = pattern_opt {
                    automation_track.borrow_mut().add_path(pattern.clone());
                } else {
                    panic!(
                        "automation track {} needs missing sequence {}",
                        track_settings.id, pattern_id
                    );
                }
            }
            automation_track.borrow_mut().freeze_trip_envelopes();
            self.add_clock_watcher_by_id(track_settings.id, automation_track.clone());
        }
    }

    fn get_pattern_by_id(&self, pattern_id: &str) -> Rc<RefCell<Pattern>> {
        (self.id_to_pattern.get(pattern_id).unwrap()).clone()
    }
}
