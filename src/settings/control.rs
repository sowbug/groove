use serde::{Deserialize, Serialize};

use crate::{clock::BeatValue, common::DeviceId};

/// A ControlTrip contains successive ControlSteps. A ControlStep
/// describes how to get from point A in time to point B in time,
/// while controlling/automating the parameter over that time.
/// For example, one ControlStep might say "go from 0.5 to 0.7
/// linearly from beat twelve to beat sixteen." The ControlTrip knows
/// which target that 0.5-0.7 applies to.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum ControlStep {
    // stairstep
    Flat {
        value: f32,
    },
    // linear
    Slope {
        start: f32,
        end: f32,
    },

    // logarithmic
    #[allow(dead_code)]
    Logarithmic {
        start: f32,
        end: f32,
    },

    // event-driven
    #[allow(dead_code)]
    Triggered {
        // TODO: if we implement this, then ControlTrips are also ControlSinks.
    },
}

impl ControlStep {
    pub fn new_flat(value: f32) -> crate::settings::control::ControlStep {
        ControlStep::Flat { value }
    }
    pub fn new_slope(start: f32, end: f32) -> crate::settings::control::ControlStep {
        ControlStep::Slope { start, end }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ControlPathSettings {
    pub id: DeviceId,
    pub note_value: Option<BeatValue>,
    pub steps: Vec<ControlStep>,
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
