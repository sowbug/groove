use crate::common::{DeviceId, MonoSample};
use crate::devices::traits::DeviceTrait;
use crate::primitives::clock::{Clock, TimeSignature};
use crate::settings::{DeviceSettings, EffectSettings, InstrumentSettings, OrchestratorSettings};
use crate::synthesizers::{drumkit_sampler, welsh};
use crossbeam::deque::Worker;
use sorted_vec::SortedVec;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::rc::Rc;

use super::effects::{Bitcrusher, Filter, Gain, Limiter};
use super::mixer::Mixer;
use super::sequencer::{Pattern, Sequencer};

#[derive(Default, Clone)]
pub struct Orchestrator {
    settings: OrchestratorSettings,

    pub clock: Clock,
    master_mixer: Rc<RefCell<Mixer>>,

    id_to_instrument: HashMap<DeviceId, Rc<RefCell<dyn DeviceTrait>>>,
    id_to_sequencer: HashMap<DeviceId, Rc<RefCell<Sequencer>>>,
    id_to_pattern: HashMap<DeviceId, Rc<RefCell<Pattern>>>,
    id_to_automation_pattern: HashMap<DeviceId, Rc<RefCell<AutomationPattern>>>,

    // legacy
    devices: Vec<Rc<RefCell<dyn DeviceTrait>>>,
}

impl Orchestrator {
    pub fn new(settings: OrchestratorSettings) -> Self {
        let mut r = Self {
            settings: settings.clone(),
            clock: Clock::new(settings.clock),
            master_mixer: Rc::new(RefCell::new(Mixer::new())),
            id_to_instrument: HashMap::new(),
            id_to_sequencer: HashMap::new(),
            id_to_pattern: HashMap::new(),
            id_to_automation_pattern: HashMap::new(),

            devices: Vec::new(),
        };
        r.set_up_from_settings();
        r
    }

    pub fn new_defaults() -> Self {
        let settings = OrchestratorSettings::new_defaults();
        Self::new(settings)
    }

    pub fn settings(&self) -> &OrchestratorSettings {
        &self.settings
    }

    pub fn add_device(&mut self, device: Rc<RefCell<dyn DeviceTrait>>) {
        self.devices.push(device);
    }

    fn tick(&mut self) -> (MonoSample, bool) {
        let mut done = true;
        for d in self.devices.clone() {
            if d.borrow().sources_automation() && d.borrow().needs_tick() {
                done = d.borrow_mut().tick(&self.clock) && done;
            }
        }
        for d in self.devices.clone() {
            if d.borrow().sources_midi() && d.borrow().needs_tick() {
                done = d.borrow_mut().tick(&self.clock) && done;
            }
        }
        for d in self.devices.clone() {
            if d.borrow().sources_audio() && d.borrow().needs_tick() {
                done = d.borrow_mut().tick(&self.clock) && done;
            }
        }
        self.clock.tick();
        for d in self.devices.clone() {
            d.borrow_mut().reset_needs_tick();
        }
        (self.master_mixer.borrow_mut().get_audio_sample(), done)
    }

    pub fn perform_to_queue(&mut self, worker: &Worker<MonoSample>) -> anyhow::Result<()> {
        loop {
            let (sample, done) = self.tick();
            worker.push(sample);
            if done {
                break;
            }
        }
        Ok(())
    }

    pub(crate) fn add_master_mixer_source(&self, device: Rc<RefCell<dyn DeviceTrait>>) {
        self.master_mixer.borrow_mut().add_audio_source(device);
    }

    // TODO: this is a temp hack while I figure out how to map tracks to specific sequencers
    fn get_hack_sequencer(&self) -> Rc<RefCell<Sequencer>> {
        self.get_sequencer_by_id(&String::from("sequencer"))
    }

    pub fn set_up_from_settings(&mut self) {
        self.create_required_entities();
        self.create_sequencers();
        self.create_effects();
        self.create_instruments();
        self.plug_in_patch_cables();
        self.create_tracks();
        self.create_automations();
    }

    fn create_instruments(&mut self) {
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
                            self.settings.clock.sample_rate(),
                            welsh::SynthPreset::by_name(&preset_name),
                        )));
                        self.id_to_instrument.insert(id, instrument.clone());
                        self.add_device(instrument.clone());
                        for sequencer in self.id_to_sequencer.values_mut() {
                            sequencer.borrow_mut().connect_midi_sink_for_channel(
                                instrument.clone(),
                                midi_input_channel,
                            );
                        }
                    }
                    InstrumentSettings::Drumkit {
                        id,
                        midi_input_channel,
                        preset_name: _preset,
                    } => {
                        let instrument =
                            Rc::new(RefCell::new(drumkit_sampler::Sampler::new_from_files()));
                        self.id_to_instrument.insert(id, instrument.clone());
                        self.add_device(instrument.clone());
                        for sequencer in self.id_to_sequencer.values_mut() {
                            sequencer.borrow_mut().connect_midi_sink_for_channel(
                                instrument.clone(),
                                midi_input_channel,
                            );
                        }
                    }
                },
                DeviceSettings::Sequencer(_settings) => { // skip
                }
                DeviceSettings::Effect(_settings) => { // skip
                }
            }
        }
    }

    fn create_effects(&mut self) {
        // Then set up effects.

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
                        self.id_to_instrument.insert(id, device.clone());
                        self.add_device(device.clone());
                    }
                    EffectSettings::Gain { id, amount } => {
                        let device = Rc::new(RefCell::new(Gain::new_with_params(amount)));
                        self.id_to_instrument.insert(id, device.clone());
                        self.add_device(device.clone());
                    }
                    EffectSettings::Bitcrusher { id, bits_to_crush } => {
                        let device =
                            Rc::new(RefCell::new(Bitcrusher::new_with_params(bits_to_crush)));
                        self.id_to_instrument.insert(id, device.clone());
                        self.add_device(device.clone());
                    }
                    EffectSettings::FilterLowPass12db { id, cutoff, q } => {
                        let device = Rc::new(RefCell::new(Filter::new_low_pass_12db(
                            self.settings().clock.sample_rate(),
                            cutoff,
                            q,
                        )));
                        self.id_to_instrument.insert(id, device.clone());
                        self.add_device(device.clone());
                    }
                    EffectSettings::FilterHighPass12db { id, cutoff, q } => {
                        let device = Rc::new(RefCell::new(Filter::new_high_pass_12db(
                            self.settings().clock.sample_rate(),
                            cutoff,
                            q,
                        )));
                        self.id_to_instrument.insert(id, device.clone());
                        self.add_device(device.clone());
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
                        self.id_to_instrument.insert(id, device.clone());
                        self.add_device(device.clone());
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
                        self.id_to_instrument.insert(id, device.clone());
                        self.add_device(device.clone());
                    }
                    EffectSettings::FilterAllPass12db { id, cutoff, q } => {
                        let device = Rc::new(RefCell::new(Filter::new_all_pass_12db(
                            self.settings().clock.sample_rate(),
                            cutoff,
                            q,
                        )));
                        self.id_to_instrument.insert(id, device.clone());
                        self.add_device(device.clone());
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
                        self.id_to_instrument.insert(id, device.clone());
                        self.add_device(device.clone());
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
                        self.id_to_instrument.insert(id, device.clone());
                        self.add_device(device.clone());
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
                        self.id_to_instrument.insert(id, device.clone());
                        self.add_device(device.clone());
                    }
                };
            }
        }
    }

    fn create_sequencers(&mut self) {
        // First set up sequencers.
        for device in self.settings.devices.clone() {
            if let DeviceSettings::Sequencer(id) = device {
                let sequencer = Rc::new(RefCell::new(Sequencer::new()));
                self.id_to_sequencer.insert(id, sequencer.clone());
                self.add_device(sequencer.clone());
                sequencer
                    .borrow_mut()
                    .set_tempo(self.clock.settings().bpm());
                sequencer
                    .borrow_mut()
                    .set_time_signature(self.clock.settings().time_signature());
            }
        }
    }

    fn create_required_entities(&mut self) {
        self.id_to_instrument
            .insert(String::from("main-mixer"), self.master_mixer.clone());
    }

    fn plug_in_patch_cables(&self) {
        for patch_cable in self.settings.patch_cables.clone() {
            if patch_cable.len() < 2 {
                dbg!("ignoring patch cable of length < 2");
                continue;
            }
            let mut last_device_id: Option<DeviceId> = None;
            for device_id in patch_cable {
                if let Some(ldi) = last_device_id {
                    let output = self.get_device_by_id(&ldi);
                    let input = self.get_device_by_id(&device_id);
                    input.borrow_mut().add_audio_source(output);
                }
                last_device_id = Some(device_id);
            }
        }
    }

    fn get_device_by_id(&self, id: &String) -> Rc<RefCell<dyn DeviceTrait>> {
        if !self.id_to_instrument.contains_key(id) {
            panic!("yo {}", id);
        }
        (self.id_to_instrument.get(id).unwrap()).clone()
    }

    fn get_sequencer_by_id(&self, id: &String) -> Rc<RefCell<Sequencer>> {
        (self.id_to_sequencer.get(id).unwrap()).clone()
    }

    fn create_tracks(&mut self) {
        if self.settings.tracks.is_empty() {
            return;
        }

        for pattern in self.settings.patterns.clone() {
            self.id_to_pattern.insert(
                pattern.id.clone(),
                Rc::new(RefCell::new(Pattern::from_settings(&pattern))),
            );
        }
        let sequencer = self.get_hack_sequencer();
        for track in self.settings.tracks.clone() {
            let channel = track.midi_channel;
            let mut insertion_point = 0u32; // TODO: which unit?
            for pattern_id in track.pattern_ids {
                let pattern = self.get_pattern_by_id(&pattern_id);
                sequencer.borrow_mut().insert_pattern(
                    pattern.clone(),
                    channel,
                    &mut insertion_point,
                );
            }
        }
    }

    fn create_automations(&mut self) {
        if self.settings.automation_tracks.is_empty() {
            return;
        }

        for pattern in self.settings.automation_patterns.clone() {
            self.id_to_automation_pattern.insert(
                pattern.id.clone(),
                Rc::new(RefCell::new(AutomationPattern::from_settings(&pattern))),
            );
        }
        for track_settings in self.settings.automation_tracks.clone() {
            let target = self
                .id_to_instrument
                .get(&track_settings.target.id)
                .unwrap();
            let automation_track = Rc::new(RefCell::new(AutomationTrack::new(
                target.clone(),
                track_settings.target.param,
            )));
            let mut insertion_point = 0u32; // TODO: this is probably wrong
            for pattern_id in track_settings.pattern_ids {
                let pattern_opt = self.id_to_automation_pattern.get(&pattern_id);
                if let Some(pattern) = pattern_opt {
                    automation_track.borrow_mut().add_pattern(
                        pattern.clone(),
                        &mut insertion_point,
                        &self.clock,
                    );
                }
            }
            self.add_device(automation_track.clone());
        }
    }

    fn get_pattern_by_id(&self, pattern_id: &str) -> Rc<RefCell<Pattern>> {
        (self.id_to_pattern.get(pattern_id).unwrap()).clone()
    }
}

struct AutomationTrack {
    patterns: Vec<Rc<RefCell<AutomationPattern>>>,
    target_instrument: Rc<RefCell<dyn DeviceTrait>>,
    target_param_name: String,

    automation_events: SortedVec<OrderedAutomationEvent>,

    // for DeviceTrait
    needs_tick: bool,
}

impl AutomationTrack {
    pub fn new(target: Rc<RefCell<dyn DeviceTrait>>, target_param_name: String) -> Self {
        Self {
            patterns: Vec::new(),
            target_instrument: target,
            target_param_name,
            automation_events: SortedVec::new(),
            needs_tick: true,
        }
    }

    pub fn add_pattern(
        &mut self,
        pattern: Rc<RefCell<AutomationPattern>>,
        insertion_point: &mut u32,
        clock: &Clock,
    ) {
        self.patterns.push(pattern.clone()); // TODO: is this necessary if we're flattening right away?
        let beat_value = (clock.settings().sample_rate() as f32 / (clock.settings().bpm() / 60.0)) as u32;
        // TODO: beat_value accumulates integer error
        for point in pattern.borrow().points.clone() {
            *insertion_point += beat_value;
            self.automation_events.insert(OrderedAutomationEvent {
                when: *insertion_point,
                target_param_value: point as u32,
            });
        }
    }
}

impl DeviceTrait for AutomationTrack {
    fn sources_automation(&self) -> bool {
        true
    }

    fn needs_tick(&self) -> bool {
        self.needs_tick
    }

    fn reset_needs_tick(&mut self) {
        self.needs_tick = true;
    }

    fn tick(&mut self, clock: &Clock) -> bool {
        self.needs_tick = false;

        // TODO: handle self.automation_events
        
        // TODO: be smarter about whether we're all done
        true
    }
}

use crate::primitives::clock::BeatValue;

#[derive(Clone)]
pub struct AutomationPattern {
    pub beat_value: Option<BeatValue>,
    pub points: Vec<f32>,
}

impl AutomationPattern {
    pub(crate) fn from_settings(settings: &crate::settings::AutomationPatternSettings) -> Self {
        Self {
            beat_value: settings.beat_value.clone(),
            points: settings.points.clone(),
        }
    }
}

#[derive(PartialEq, PartialOrd, Clone)]
pub struct OrderedAutomationEvent {
    when: u32,
    target_param_value: u32,
}

impl Ord for OrderedAutomationEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        self.when.cmp(&other.when)
    }
}

impl Eq for OrderedAutomationEvent {}
