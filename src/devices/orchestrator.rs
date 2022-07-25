use crate::common::DeviceId;
use crate::devices::traits::DeviceTrait;
use crate::primitives::clock::Clock;
use crate::settings::{DeviceSettings, EffectSettings, InstrumentSettings, OrchestratorSettings};
use crate::synthesizers::{drumkit_sampler, welsh};
use crossbeam::deque::Worker;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::effects::{Bitcrusher, Gain, Limiter};
use super::mixer::Mixer;
use super::sequencer::Sequencer;

#[derive(Default, Clone)]
pub struct Orchestrator {
    settings: OrchestratorSettings,

    pub clock: Clock,
    master_mixer: Rc<RefCell<Mixer>>,

    id_to_instrument: HashMap<DeviceId, Rc<RefCell<dyn DeviceTrait>>>,
    id_to_sequencer: HashMap<DeviceId, Rc<RefCell<Sequencer>>>,
    id_to_pattern: HashMap<DeviceId, Rc<RefCell<Pattern>>>,

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

    fn tick(&mut self) -> (f32, bool) {
        let mut done = true;
        for d in self.devices.clone() {
            if d.borrow().sources_midi() {
                done = d.borrow_mut().tick(&self.clock) && done;
            }
        }
        for d in self.devices.clone() {
            if d.borrow().sources_audio() {
                done = d.borrow_mut().tick(&self.clock) && done;
            }
        }
        self.clock.tick();
        (self.master_mixer.borrow_mut().get_audio_sample(), done)
    }

    pub fn perform_to_queue(&mut self, worker: &Worker<f32>) -> anyhow::Result<()> {
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
            match device {
                DeviceSettings::Effect(effect_settings) => {
                    match effect_settings {
                        // This has more repetition than we'd expect because of
                        // https://stackoverflow.com/questions/26378842/how-do-i-overcome-match-arms-with-incompatible-types-for-structs-implementing-sa
                        //
                        // Match arms have to return the same types, and returning a Rc<RefCell<dyn some trait>> doesn't count
                        // as the same type.
                        EffectSettings::Limiter { id, min, max } => {
                            let device = Rc::new(RefCell::new(Limiter::new_with_params(min, max)));
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
                    };
                }
                // this is OK because we are looking only for effects, and want to ignore other instruments.
                _ => {}
            }
        }
    }

    fn create_sequencers(&mut self) {
        // First set up sequencers.
        for device in self.settings.devices.clone() {
            match device {
                DeviceSettings::Sequencer(id) => {
                    let sequencer = Rc::new(RefCell::new(Sequencer::new()));
                    self.id_to_sequencer.insert(id, sequencer.clone());
                    self.add_device(sequencer.clone());
                }
                _ => {}
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
                if last_device_id.is_some() {
                    let output = self.get_device_by_id(&last_device_id.unwrap());
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

    fn get_pattern_by_id(&self, pattern_id: &str) -> Rc<RefCell<Pattern>> {
        (self.id_to_pattern.get(pattern_id).unwrap()).clone()
    }
}

#[derive(Clone)]
pub struct Pattern {
    pub division: u8,
    pub notes: Vec<Vec<u8>>,
}

impl Pattern {
    pub(crate) fn from_settings(settings: &crate::settings::PatternSettings) -> Self {
        let mut r = Self {
            division: settings.division,
            notes: Vec::new(),
        };
        for note_sequence in settings.notes.clone() {
            let mut note_vec = Vec::new();
            for note in note_sequence.clone() {
                note_vec.push(note);
            }
            r.notes.push(note_vec);
        }
        r
    }
}
