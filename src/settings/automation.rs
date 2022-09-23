use serde::{Deserialize, Serialize};

use crate::{common::DeviceId, primitives::clock::BeatValue};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum AutomationStepType {
    Flat {value: f32},
    // Linear {start: f32, end: f32},
    // Logarithmic {start: f32, end: f32},
    // Trigger {id: String, value: f32}, // TODO: this might mean Automators are also AutomationSinks
    //          // and maybe MidiSinks.
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct AutomationSequenceSettings {
    pub id: DeviceId,
    pub note_value: Option<BeatValue>,
    pub steps: Vec<AutomationStepType>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct AutomationTargetSettings {
    pub id: DeviceId,
    pub param: String,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct AutomationTrackSettings {
    pub id: DeviceId,
    pub target: AutomationTargetSettings,

    #[serde(rename = "patterns")]
    pub pattern_ids: Vec<DeviceId>,
}
