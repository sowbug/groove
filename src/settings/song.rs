use serde::{Deserialize, Serialize};

use crate::common::DeviceId;

use super::{
    automation::{AutomationPatternSettings, AutomationTrackSettings},
    ClockSettings, DeviceSettings, PatternSettings, TrackSettings,
};

type PatchCable = Vec<DeviceId>; // first is source, last is sink

#[derive(Serialize, Deserialize, Default, Clone)]
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
    pub automation_patterns: Vec<AutomationPatternSettings>,
    #[serde(default)]
    pub automation_tracks: Vec<AutomationTrackSettings>,
}

impl SongSettings {
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

    use crate::{
        common::MonoSample, devices::orchestrator::Orchestrator, settings::InstrumentSettings,
        synthesizers::welsh::PresetName,
    };

    use super::{DeviceSettings, SongSettings};

    impl SongSettings {
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
        let yaml = std::fs::read_to_string("test_data/kitchen-sink.yaml").unwrap();
        let settings = SongSettings::new_from_yaml(yaml.as_str());
        let mut orchestrator = Orchestrator::new(settings);
        let worker = Worker::<MonoSample>::new_fifo();
        assert!(orchestrator.perform_to_queue(&worker).is_ok());
    }
}
