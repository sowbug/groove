use crate::common::{DeviceId, MidiChannel, MonoSample};
use crate::primitives::clock::Clock;
use crate::primitives::{SinksAudio, SourcesAudio, WatchesClock, SinksControl};
use crate::settings::effects::EffectSettings;
use crate::settings::song::SongSettings;
use crate::settings::{DeviceSettings, InstrumentSettings};

use crate::synthesizers::{drumkit_sampler, welsh};
use crossbeam::deque::Worker;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, Write};
use std::rc::Rc;

use super::automation::{AutomationPath, ControlTrip};
use super::effects::{Bitcrusher, Filter, Gain, Limiter};
use super::mixer::Mixer;
use super::patterns::{Pattern, PatternSequencer};
use super::sequencer::MidiSequencer;
use super::traits::{ArpTrait, AutomatorTrait, EffectTrait, InstrumentTrait, MidiSource};
use super::Arpeggiator;

/// Orchestrator takes a description of a song and turns it into an in-memory representation that is ready to render to sound.
#[derive(Default, Clone)]
pub struct Orchestrator {
    settings: SongSettings,

    master_mixer: Rc<RefCell<Mixer>>,
    midi_sequencer: Rc<RefCell<MidiSequencer>>,
    pattern_sequencer: Rc<RefCell<PatternSequencer>>,

    id_to_automator: HashMap<DeviceId, Rc<RefCell<dyn AutomatorTrait>>>,
    id_to_instrument: HashMap<DeviceId, Rc<RefCell<dyn InstrumentTrait>>>,
    id_to_effect: HashMap<DeviceId, Rc<RefCell<dyn EffectTrait>>>,
    id_to_arp: HashMap<DeviceId, Rc<RefCell<dyn ArpTrait>>>, // TODO: learn Rust

    id_to_pattern: HashMap<DeviceId, Rc<RefCell<Pattern>>>,
    id_to_automation_sequence: HashMap<DeviceId, Rc<RefCell<AutomationPath>>>,
}

impl Orchestrator {
    pub fn new(settings: SongSettings) -> Self {
        let mut r = Self {
            settings: settings.clone(),
            master_mixer: Rc::new(RefCell::new(Mixer::new())),
            midi_sequencer: Rc::new(RefCell::new(MidiSequencer::new())),
            pattern_sequencer: Rc::new(RefCell::new(PatternSequencer::new(
                &settings.clock.time_signature(),
            ))),
            id_to_automator: HashMap::new(),
            id_to_instrument: HashMap::new(),
            id_to_effect: HashMap::new(),
            id_to_arp: HashMap::new(),

            id_to_pattern: HashMap::new(),
            id_to_automation_sequence: HashMap::new(),
        };
        r.add_effect_by_id(String::from("main-mixer"), r.master_mixer.clone());

        r.prepare_from_settings();
        r
    }

    pub fn new_defaults() -> Self {
        Self::new(SongSettings::new_defaults())
    }

    pub fn settings(&self) -> &SongSettings {
        &self.settings
    }

    fn tick(&mut self, clock: &mut Clock) -> (MonoSample, bool) {
        let mut done = true;
        for d in self.id_to_automator.values() {
            done = d.borrow_mut().tick(clock) && done;
        }
        done = self.midi_sequencer.borrow_mut().tick(clock) && done;
        done = self.pattern_sequencer.borrow_mut().tick(clock) && done;
        for d in self.id_to_arp.values() {
            done = d.borrow_mut().tick(clock) && done;
        }
        for d in self.id_to_instrument.values() {
            done = d.borrow_mut().tick(clock) && done;
        }
        for d in self.id_to_effect.values() {
            done = d.borrow_mut().tick(clock) && done;
        }
        (self.master_mixer.borrow_mut().source_audio(&clock), done)
    }

    pub fn perform_to_queue(&mut self, worker: &Worker<MonoSample>) -> anyhow::Result<()> {
        let mut clock = Clock::new_with(&self.settings.clock);

        let progress_indicator_quantum: usize = clock.settings().sample_rate() / 2;
        let mut next_progress_indicator: usize = progress_indicator_quantum;
        loop {
            let (sample, done) = self.tick(&mut clock);
            worker.push(sample);
            clock.tick();
            if next_progress_indicator <= clock.samples {
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

    pub fn add_master_mixer_source(&self, device: Rc<RefCell<dyn SourcesAudio>>) {
        self.master_mixer.borrow_mut().add_audio_source(device);
    }

    fn prepare_from_settings(&mut self) {
        self.create_effects_from_settings();
        self.create_instruments_from_settings();
        self.create_patch_cables_from_settings();
        self.create_tracks_from_settings();
        self.create_automation_tracks_from_settings();
    }

    pub fn add_instrument_by_id(
        &mut self,
        id: String,
        instrument: Rc<RefCell<dyn InstrumentTrait>>,
        channel: MidiChannel,
    ) {
        self.id_to_instrument.insert(id, instrument.clone());
        self.midi_sequencer
            .borrow_mut()
            .add_midi_sink(instrument.clone(), channel);
        self.pattern_sequencer
            .borrow_mut()
            .add_midi_sink(instrument.clone(), channel);
        for arp in self.id_to_arp.values() {
            arp.borrow_mut().add_midi_sink(instrument.clone(), channel);
        }
    }

    pub fn add_arp_by_id(
        &mut self,
        id: String,
        arp: Rc<RefCell<dyn ArpTrait>>,
        channel: MidiChannel,
    ) {
        self.id_to_arp.insert(id, arp.clone());
        self.midi_sequencer
            .borrow_mut()
            .add_midi_sink(arp.clone(), channel);
        self.pattern_sequencer
            .borrow_mut()
            .add_midi_sink(arp.clone(), channel);
    }

    fn add_effect_by_id(&mut self, id: String, instrument: Rc<RefCell<dyn EffectTrait>>) {
        self.id_to_effect.insert(id, instrument.clone());
    }

    fn add_automator_by_id(&mut self, id: String, automator: Rc<RefCell<dyn AutomatorTrait>>) {
        self.id_to_automator.insert(id, automator.clone());
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
                        self.add_instrument_by_id(id, instrument, midi_input_channel);
                    }
                    InstrumentSettings::Drumkit {
                        id,
                        midi_input_channel,
                        preset_name: _preset,
                    } => {
                        let instrument = Rc::new(RefCell::new(
                            drumkit_sampler::Sampler::new_from_files(midi_input_channel),
                        ));
                        self.add_instrument_by_id(id, instrument, midi_input_channel);
                    }
                    InstrumentSettings::Arpeggiator {
                        id,
                        midi_input_channel,
                        midi_output_channel,
                    } => self.add_arp_by_id(
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
                        let device = Rc::new(RefCell::new(Limiter::new_with_params(
                            min as MonoSample,
                            max as MonoSample,
                        )));
                        self.add_effect_by_id(id, device);
                    }
                    EffectSettings::Gain { id, amount } => {
                        let device = Rc::new(RefCell::new(Gain::new_with(amount)));
                        self.add_effect_by_id(id, device);
                    }
                    EffectSettings::Bitcrusher { id, bits_to_crush } => {
                        let device =
                            Rc::new(RefCell::new(Bitcrusher::new_with_params(bits_to_crush)));
                        self.add_effect_by_id(id, device);
                    }
                    EffectSettings::FilterLowPass12db { id, cutoff, q } => {
                        let device = Rc::new(RefCell::new(Filter::new_low_pass_12db(
                            self.settings().clock.sample_rate(),
                            cutoff,
                            q,
                        )));
                        self.add_effect_by_id(id, device);
                    }
                    EffectSettings::FilterHighPass12db { id, cutoff, q } => {
                        let device = Rc::new(RefCell::new(Filter::new_high_pass_12db(
                            self.settings().clock.sample_rate(),
                            cutoff,
                            q,
                        )));
                        self.add_effect_by_id(id, device);
                    }
                    EffectSettings::FilterBandPass12db {
                        id,
                        cutoff,
                        bandwidth,
                    } => {
                        let device = Rc::new(RefCell::new(Filter::new_band_pass_12db(
                            self.settings().clock.sample_rate(),
                            cutoff,
                            bandwidth,
                        )));
                        self.add_effect_by_id(id, device);
                    }
                    EffectSettings::FilterBandStop12db {
                        id,
                        cutoff,
                        bandwidth,
                    } => {
                        let device = Rc::new(RefCell::new(Filter::new_band_stop_12db(
                            self.settings().clock.sample_rate(),
                            cutoff,
                            bandwidth,
                        )));
                        self.add_effect_by_id(id, device);
                    }
                    EffectSettings::FilterAllPass12db { id, cutoff, q } => {
                        let device = Rc::new(RefCell::new(Filter::new_all_pass_12db(
                            self.settings().clock.sample_rate(),
                            cutoff,
                            q,
                        )));
                        self.add_effect_by_id(id, device);
                    }
                    EffectSettings::FilterPeakingEq12db {
                        id,
                        cutoff,
                        db_gain,
                    } => {
                        let device = Rc::new(RefCell::new(Filter::new_peaking_eq_12db(
                            self.settings().clock.sample_rate(),
                            cutoff,
                            db_gain,
                        )));
                        self.add_effect_by_id(id, device);
                    }
                    EffectSettings::FilterLowShelf12db {
                        id,
                        cutoff,
                        db_gain,
                    } => {
                        let device = Rc::new(RefCell::new(Filter::new_low_shelf_12db(
                            self.settings().clock.sample_rate(),
                            cutoff,
                            db_gain,
                        )));
                        self.add_effect_by_id(id, device);
                    }
                    EffectSettings::FilterHighShelf12db {
                        id,
                        cutoff,
                        db_gain,
                    } => {
                        let device = Rc::new(RefCell::new(Filter::new_high_shelf_12db(
                            self.settings().clock.sample_rate(),
                            cutoff,
                            db_gain,
                        )));
                        self.add_effect_by_id(id, device);
                    }
                };
            }
        }
    }

    fn create_patch_cables_from_settings(&self) {
        for patch_cable in self.settings.patch_cables.clone() {
            if patch_cable.len() < 2 {
                dbg!("ignoring patch cable of length < 2");
                continue;
            }
            let mut last_device_id: Option<DeviceId> = None;
            for device_id in patch_cable {
                if let Some(ldi) = last_device_id {
                    let output: Rc<RefCell<dyn SourcesAudio>> = self.get_audio_source_by_id(&ldi);
                    let input: Rc<RefCell<dyn SinksAudio>> = self.get_audio_sink_by_id(&device_id);
                    input.borrow_mut().add_audio_source(output);
                }
                last_device_id = Some(device_id);
            }
        }
    }

    #[allow(dead_code)]
    fn get_instrument_by_id(&self, id: &str) -> Rc<RefCell<dyn InstrumentTrait>> {
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
        if self.id_to_effect.contains_key(id) {
            return (self.id_to_effect.get(id).unwrap()).clone();
        }
        panic!("yo {}", id);
    }

    fn get_automation_sink_by_id(&self, id: &str) -> Rc<RefCell<dyn SinksControl>> {
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
                Rc::new(RefCell::new(AutomationPath::from_settings(&sequence))),
            );
        }
        for track_settings in self.settings.trips.clone() {
            let target = self.get_automation_sink_by_id(&track_settings.target.id);
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
            self.add_automator_by_id(track_settings.id, automation_track.clone());
        }
    }

    fn get_pattern_by_id(&self, pattern_id: &str) -> Rc<RefCell<Pattern>> {
        (self.id_to_pattern.get(pattern_id).unwrap()).clone()
    }

    pub fn midi_sequencer(&self) -> Rc<RefCell<MidiSequencer>> {
        self.midi_sequencer.clone()
    }
}
