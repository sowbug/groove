use serde::{Deserialize, Serialize};

use crate::{
    common::DeviceId, devices::sequencer::BeatValue, primitives::clock::ClockSettings,
    synthesizers::welsh::PresetName,
};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum InstrumentType {
    Welsh,
    Drumkit,
}

type MidiChannel = u8;

type PatchCable = Vec<DeviceId>; // first is source, last is sink

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum InstrumentSettings {
    #[serde(rename_all = "kebab-case")]
    Welsh {
        id: DeviceId,
        #[serde(rename = "midi-in")]
        midi_input_channel: MidiChannel,
        #[serde(rename = "preset")]
        preset_name: PresetName,
    },
    #[serde(rename_all = "kebab-case")]
    Drumkit {
        id: DeviceId,
        #[serde(rename = "midi-in")]
        midi_input_channel: MidiChannel,
        #[serde(rename = "preset")]
        preset_name: String,
    },
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum EffectSettings {
    Gain {
        id: DeviceId,
        amount: f32,
    },
    Limiter {
        id: DeviceId,
        min: f32,
        max: f32,
    },
    Bitcrusher {
        id: DeviceId,
        bits_to_crush: u8,
    },
    #[serde(rename = "filter-low-pass-12db")]
    FilterLowPass12db {
        id: DeviceId,
        cutoff: f32,
        q: f32,
    },
    #[serde(rename = "filter-high-pass-12db")]
    FilterHighPass12db {
        id: DeviceId,
        cutoff: f32,
        q: f32,
    },
    FilterBandPass12db {
        id: DeviceId,
        cutoff: f32,
        bandwidth: f32,
    },
    FilterBandStop12db {
        id: DeviceId,
        cutoff: f32,
        bandwidth: f32,
    },
    FilterAllPass12db {
        id: DeviceId,
        cutoff: f32,
        q: f32,
    },
    FilterPeakingEq12db {
        id: DeviceId,
        cutoff: f32,
        db_gain: f32,
    },
    FilterLowShelf12db {
        id: DeviceId,
        cutoff: f32,
        db_gain: f32,
    },
    FilterHighShelf12db {
        id: DeviceId,
        cutoff: f32,
        db_gain: f32,
    },
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum DeviceSettings {
    Instrument(InstrumentSettings),
    Sequencer(DeviceId),
    Effect(EffectSettings),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct PatternSettings {
    pub id: DeviceId,
    pub beat_value: Option<BeatValue>,
    pub notes: Vec<Vec<u8>>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct TrackSettings {
    pub id: DeviceId,
    pub midi_channel: MidiChannel,

    #[serde(rename = "patterns")]
    pub pattern_ids: Vec<DeviceId>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct OrchestratorSettings {
    pub clock: ClockSettings,

    pub devices: Vec<DeviceSettings>,
    pub patch_cables: Vec<PatchCable>,
    #[serde(default)]
    pub patterns: Vec<PatternSettings>,
    #[serde(default)]
    pub tracks: Vec<TrackSettings>,
}

impl OrchestratorSettings {
    pub fn new_defaults() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn new_from_yaml(yaml: &str) -> Self {
        serde_yaml::from_str(yaml).unwrap()
    }

    #[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use crossbeam::deque::Worker;

    use crate::{devices::orchestrator::Orchestrator, synthesizers::welsh::PresetName, common::MonoSample};

    use super::{DeviceSettings, InstrumentSettings, OrchestratorSettings};

    impl OrchestratorSettings {
        #[allow(dead_code)]
        pub fn new_dev() -> Self {
            let mut r = Self {
                ..Default::default()
            };
            r.devices
                .push(DeviceSettings::Instrument(InstrumentSettings::Welsh {
                    id: String::from("piano-1"),
                    midi_input_channel: 0,
                    preset_name: PresetName::Piano,
                }));

            r.devices
                .push(DeviceSettings::Instrument(InstrumentSettings::Drumkit {
                    id: String::from("drum-1"),
                    midi_input_channel: 10,
                    preset_name: String::from("707"), // TODO, for now all 707
                }));

            r.devices
                .push(DeviceSettings::Sequencer(String::from("sequencer")));
            r.patch_cables
                .push(Self::new_patch_cable(vec!["piano", "main-mixer"]));
            r.patch_cables
                .push(Self::new_patch_cable(vec!["drumkit", "main-mixer"]));

            r
        }
    }

    #[test]
    fn test_yaml_loads_and_parses() {
        let yaml = std::fs::read_to_string("scripts/exercise-everything.yaml").unwrap();
        let settings = OrchestratorSettings::new_from_yaml(yaml.as_str());
        let mut orchestrator = Orchestrator::new(settings);
        let worker = Worker::<MonoSample>::new_fifo();
        assert!(orchestrator.perform_to_queue(&worker).is_ok());
    }
}
