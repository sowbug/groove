use super::{
    control::{ControlPathSettings, ControlTripSettings},
    ClockSettings, DeviceSettings, PatternSettings, TrackSettings,
};
use crate::common::DeviceId;
use serde::{Deserialize, Serialize};

type PatchCable = Vec<DeviceId>; // first is source, last is sink

#[derive(Debug, Clone)]
pub enum LoadError {
    FileError,
    FormatError,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct SongSettings {
    pub clock: ClockSettings,

    pub devices: Vec<DeviceSettings>,
    pub patch_cables: Vec<PatchCable>,
    #[serde(default)]
    pub patterns: Vec<PatternSettings>,
    #[serde(default)]
    pub tracks: Vec<TrackSettings>,
    #[serde(default)]
    pub paths: Vec<ControlPathSettings>,
    #[serde(default)]
    pub trips: Vec<ControlTripSettings>,
}

impl SongSettings {
    pub fn new_defaults() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn new_from_yaml(yaml: &str) -> Result<SongSettings, LoadError> {
        serde_yaml::from_str(yaml).map_err(|e| {
            println!("{}", e);
            LoadError::FormatError
        })
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

    use super::{DeviceSettings, SongSettings};
    use crate::{
        orchestrator::Orchestrator, settings::InstrumentSettings, synthesizers::welsh::PresetName,
    };

    impl SongSettings {
        #[allow(dead_code)]
        pub fn new_dev() -> Self {
            let mut r = Self {
                ..Default::default()
            };
            r.devices.push(DeviceSettings::Instrument(
                String::from("piano-1"),
                InstrumentSettings::Welsh {
                    midi_input_channel: 0,
                    preset_name: PresetName::Piano,
                },
            ));

            r.devices.push(DeviceSettings::Instrument(
                String::from("drum-1"),
                InstrumentSettings::Drumkit {
                    midi_input_channel: 10,
                    preset_name: String::from("707"), // TODO, for now all 707
                },
            ));

            r.patch_cables
                .push(Self::new_patch_cable(vec!["piano", "main-mixer"]));
            r.patch_cables
                .push(Self::new_patch_cable(vec!["drumkit", "main-mixer"]));

            r
        }
    }

    #[test]
    fn test_yaml_loads_and_parses() {
        let yaml = std::fs::read_to_string("test_data/kitchen-sink.yaml").unwrap();
        if let Ok(song_settings) = SongSettings::new_from_yaml(yaml.as_str()) {
            let mut orchestrator = Orchestrator::new_with(song_settings);
            if let Ok(_performance) = orchestrator.perform() {
                // cool
            } else {
                assert!(false, "performance failed");
            }
        } else {
            assert!(false, "loading settings failed");
        }
    }
}
