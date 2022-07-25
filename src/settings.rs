use serde::{Deserialize, Serialize};

use crate::{common::DeviceId, primitives::clock::ClockSettings, synthesizers::welsh::PresetName};

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

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct PatternSettings {
    pub id: DeviceId,
    pub division: u8,
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
    use crate::synthesizers::welsh::PresetName;

    use super::{DeviceSettings, InstrumentSettings, OrchestratorSettings};

    impl OrchestratorSettings {
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
}