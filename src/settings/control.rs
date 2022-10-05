use serde::{Deserialize, Serialize};

use crate::{common::DeviceId, clock::BeatValue};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum ControlStepType {
    Flat { value: f32 }, // stairstep
    Slope { start: f32, end: f32 }, // linear

                         // Logarithmic {start: f32, end: f32},
                         // Trigger {id: String, value: f32}, // TODO: this might mean Automators are also AutomationSinks
                         //          // and maybe MidiSinks.
}

impl ControlStepType {
    pub fn new_flat(value: f32) -> crate::settings::control::ControlStepType {
        ControlStepType::Flat { value }
    }
    pub fn new_slope(start: f32, end: f32) -> crate::settings::control::ControlStepType {
        ControlStepType::Slope { start, end }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ControlPathSettings {
    pub id: DeviceId,
    pub note_value: Option<BeatValue>,
    pub steps: Vec<ControlStepType>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ControlTargetSettings {
    pub id: DeviceId,
    pub param: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ControlTripSettings {
    pub id: DeviceId,
    pub target: ControlTargetSettings,

    #[serde(rename = "paths")]
    pub path_ids: Vec<DeviceId>,
}
