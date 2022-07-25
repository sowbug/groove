use crate::common::{MidiMessage, OrderedMidiMessage};
use crate::devices::traits::DeviceTrait;
use crate::primitives::clock::{Clock, ClockSettings};
use crate::synthesizers::{drumkit_sampler, welsh};
use crossbeam::deque::Worker;
use serde::{Deserialize, Serialize};
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
    id_to_instrument: HashMap<DeviceId, Rc<RefCell<dyn DeviceTrait>>>,
    id_to_sequencer: HashMap<DeviceId, Rc<RefCell<Sequencer>>>,
    master_mixer: Rc<RefCell<Mixer>>,

    // legacy
    devices: Vec<Rc<RefCell<dyn DeviceTrait>>>,
}

impl Orchestrator {
    pub fn new(settings: OrchestratorSettings) -> Self {
        let mut r = Self {
            settings: settings.clone(),
            clock: Clock::new(settings.clock),
            id_to_instrument: HashMap::new(),
            id_to_sequencer: HashMap::new(),
            master_mixer: Rc::new(RefCell::new(Mixer::new())),
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

    pub fn set_up_from_settings(&mut self) {
        self.id_to_instrument
            .insert(String::from("main-mixer"), self.master_mixer.clone());

        // First set up sequencers.
        for instrument in self.settings.devices.clone() {
            match instrument {
                DeviceSettings::Sequencer(id) => {
                    let sequencer = Rc::new(RefCell::new(Sequencer::new()));
                    self.id_to_sequencer.insert(id, sequencer.clone());
                    self.add_device(sequencer.clone());
                }
                _ => {}
            }
        }

        // Then set up effects.
        for device_settings in self.settings.devices.clone() {
            match device_settings {
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

        // Then set up instruments, attaching to sequencers as they're set up.
        for device in self.settings.devices.clone() {
            match device {
                DeviceSettings::Instrument(instrument_settings) => {
                    let instrument = self.instrument_from_type(instrument_settings.device_type);
                    self.id_to_instrument
                        .insert(instrument_settings.id, instrument.clone());
                    self.add_device(instrument.clone());
                    for sequencer in self.id_to_sequencer.values_mut() {
                        sequencer.borrow_mut().connect_midi_sink_for_channel(
                            instrument.clone(),
                            instrument_settings.midi_input_channel,
                        );
                    }
                }
                _ => {}
            }
        }
        self.plug_in_patch_cables();

        // TODO: this is a temp hack while I figure out how to map tracks to specific sequencers
        if !self.settings.notes.is_empty() {
            let sequencer_device = self.get_sequencer_by_id(&String::from("sequencer"));
            for note in self.settings.notes.clone() {
                sequencer_device.borrow_mut().add_message(note);
            }
        }
    }

    fn instrument_from_type(&self, device_type: InstrumentType) -> Rc<RefCell<dyn DeviceTrait>> {
        match device_type {
            InstrumentType::Welsh => Rc::new(RefCell::new(welsh::Synth::new(
                self.settings.clock.sample_rate(),
                welsh::SynthPreset::by_name(&welsh::PresetName::Piano),
            ))),
            InstrumentType::Drumkit => Rc::new(RefCell::new(drumkit_sampler::Sampler::new())),
        }
        // TODO: maybe create a MIDI bus struct, depending on how far I want to lean into MIDI
    }

    fn effect_from_type(&self, device_type: EffectType) -> Rc<RefCell<dyn DeviceTrait>> {
        match device_type {
            EffectType::Gain => Rc::new(RefCell::new(Gain::new())),
            EffectType::Limiter => Rc::new(RefCell::new(Limiter::new())),
            EffectType::Bitcrusher => Rc::new(RefCell::new(Bitcrusher::new())),
        }
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
        (*self.id_to_instrument.get(id).unwrap()).clone()
    }

    fn get_sequencer_by_id(&self, id: &String) -> Rc<RefCell<Sequencer>> {
        (*self.id_to_sequencer.get(id).unwrap()).clone()
    }
}

type DeviceId = String;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum InstrumentType {
    Welsh,
    Drumkit,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum EffectType {
    Gain,
    Limiter,
    Bitcrusher,
}

type MidiChannel = u8;

type PatchCable = Vec<DeviceId>; // first is source, last is sink

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct InstrumentSettings {
    id: DeviceId,
    #[serde(rename = "type")]
    device_type: InstrumentType,
    midi_input_channel: MidiChannel,
}

// #[derive(Serialize, Deserialize, Clone)]
// #[serde(rename_all = "kebab-case")]
// pub struct EffectSettings {
//     id: DeviceId,
//     #[serde(rename = "type")]
//     device_type: EffectType,
// }

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum EffectSettings {
    Gain { id: DeviceId, amount: f32 },
    Limiter { id: DeviceId, min: f32, max: f32 },
    Bitcrusher { id: DeviceId, bits_to_crush: u8 },
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum DeviceSettings {
    Instrument(InstrumentSettings),
    Sequencer(DeviceId),
    Effect(EffectSettings),
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct OrchestratorSettings {
    pub clock: ClockSettings,

    pub devices: Vec<DeviceSettings>,
    pub patch_cables: Vec<PatchCable>,
    pub notes: Vec<OrderedMidiMessage>,
}

impl OrchestratorSettings {
    pub fn new_defaults() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn new_dev() -> Self {
        let mut r = Self {
            ..Default::default()
        };
        r.devices
            .push(DeviceSettings::Instrument(InstrumentSettings {
                id: String::from("piano"),
                device_type: InstrumentType::Welsh,
                midi_input_channel: 0,
            }));

        r.devices
            .push(DeviceSettings::Instrument(InstrumentSettings {
                id: String::from("drum"),
                device_type: InstrumentType::Drumkit,
                midi_input_channel: 10,
            }));

        r.devices
            .push(DeviceSettings::Sequencer(String::from("sequencer")));
        r.patch_cables
            .push(Self::new_patch_cable(vec!["piano", "main-mixer"]));
        r.patch_cables
            .push(Self::new_patch_cable(vec!["drumkit", "main-mixer"]));

        let mut i: u32 = 0;
        for note in vec![60, 64, 67, 72, 67, 64, 60] {
            r.notes.push(OrderedMidiMessage {
                when: i * 960 * 2,
                message: MidiMessage::new_note_on(0, note as u8, 100),
            });
            r.notes.push(OrderedMidiMessage {
                when: i * 960 * 2 + 960,
                message: MidiMessage::new_note_off(0, note as u8, 100),
            });
            i += 1;
        }
        r
    }

    pub fn new_from_yaml(yaml: &str) -> Self {
        serde_yaml::from_str(yaml).unwrap()
    }

    fn new_patch_cable(devices_to_connect: Vec<&str>) -> PatchCable {
        if devices_to_connect.len() < 2 {
            panic!("need vector of at least two devices to create PatchCable");
        }
        let mut patch_cable: Vec<DeviceId> = Vec::new();

        for device in devices_to_connect {
            patch_cable.push(String::from(device));
        }
        patch_cable
    }
}
