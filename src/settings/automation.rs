use serde::{Deserialize, Serialize};

use crate::{common::DeviceId, primitives::clock::BeatValue};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Debug, Default)]
#[serde(rename_all = "kebab-case")]
pub enum InterpolationType {
    #[default]
    Stairstep,
    Linear,
    Logarithmic,
    Trigger, // TODO: this might mean Automators are also AutomationSinks
             // and maybe MidiSinks.
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct AutomationPatternSettings {
    pub id: DeviceId,
    pub note_value: Option<BeatValue>,
    pub interpolation: Option<InterpolationType>,
    pub points: Vec<f32>,
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
