use crate::common::{MidiMessage, OrderedMidiMessage};
use crate::devices::traits::DeviceTrait;
use crate::primitives::clock::{Clock, ClockSettings};
use crate::synthesizers::{drumkit_sampler, welsh};
use crossbeam::deque::Worker;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

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
        (self.master_mixer.borrow().get_audio_sample(), done)
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
            .insert(String::from("main_mixer"), self.master_mixer.clone());

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
        // Then set up instruments, attaching to sequencers as they're set up.
        for device in self.settings.devices.clone() {
            match device {
                DeviceSettings::Instrument(instrument_settings) => {
                    let instrument = self.instrument_from_type(instrument_settings.instrument_type);
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
            let sequencer_device = self.get_sequencer_by_id(String::from("sequencer"));
            for note in self.settings.notes.clone() {
                sequencer_device.borrow_mut().add_message(note);
            }
        }
    }

    fn instrument_from_type(
        &self,
        instrument_type: InstrumentType,
    ) -> Rc<RefCell<dyn DeviceTrait>> {
        match instrument_type {
            InstrumentType::Welsh => Rc::new(RefCell::new(welsh::Synth::new(
                self.settings.clock.sample_rate(),
                welsh::SynthPreset::by_name(&welsh::PresetName::Piano),
            ))),
            InstrumentType::Drumkit => {
                Rc::new(RefCell::new(drumkit_sampler::Sampler::new()))
            }
        }
        // TODO: maybe create a MIDI bus struct, depending on how far I want to lean into MIDI
    }

    fn plug_in_patch_cables(&self) {
        for (output_id, input_id) in self.settings.patch_cables.clone() {
            let output = self.get_device_by_id(output_id);
            let input = self.get_device_by_id(input_id);

            input.borrow_mut().add_audio_source(output);
        }
    }

    fn get_device_by_id(&self, id: String) -> Rc<RefCell<dyn DeviceTrait>> {
        (*self.id_to_instrument.get(&id).unwrap()).clone()
    }

    fn get_sequencer_by_id(&self, id: String) -> Rc<RefCell<Sequencer>> {
        (*self.id_to_sequencer.get(&id).unwrap()).clone()
    }
}

type DeviceId = String;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum InstrumentType {
    Welsh,
    Drumkit,
}

type MidiChannel = u8;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct InstrumentSettings {
    id: DeviceId,
    #[serde(rename="type")]
    instrument_type: InstrumentType,
    midi_input_channel: MidiChannel,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum DeviceSettings {
    Instrument(InstrumentSettings),
    Sequencer(DeviceId),
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct OrchestratorSettings {
    pub clock: ClockSettings,

    pub devices: Vec<DeviceSettings>,
    pub patch_cables: Vec<(DeviceId, DeviceId)>, // (output, input)
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
                instrument_type: InstrumentType::Welsh,
                midi_input_channel: 0,
            }));

        r.devices
            .push(DeviceSettings::Instrument(InstrumentSettings {
                id: String::from("drum"),
                instrument_type: InstrumentType::Drumkit,
                midi_input_channel: 10,
            }));

        r.devices
            .push(DeviceSettings::Sequencer(String::from("sequencer")));
        r.patch_cables
            .push((String::from("piano"), String::from("main_mixer")));
        r.patch_cables
            .push((String::from("drum"), String::from("main_mixer")));

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
}
