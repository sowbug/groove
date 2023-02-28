use super::{patches::WaveformType, ClockSettings, DeviceId, MidiChannel};
use crate::{
    clock::BeatValue,
    common::SignalType,
    controllers::{Arpeggiator, LfoController, SignalPassthroughController, TestController},
    entities::Entity,
};
use serde::{Deserialize, Serialize};

/// A ControlTrip contains successive ControlSteps. A ControlStep describes how
/// to get from point A in time to point B in time, while controlling/automating
/// the parameter over that time. For example, one ControlStep might say "go
/// from 0.5 to 0.7 linearly from beat twelve to beat sixteen." The ControlTrip
/// knows which target that 0.5-0.7 applies to.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum ControlStep {
    /// Stairstep: one value per step.
    Flat { value: SignalType },
    /// Linear: start at one value and end at another.
    Slope { start: SignalType, end: SignalType },

    /// Curved; starts out changing quickly and ends up changing slowly.
    Logarithmic { start: SignalType, end: SignalType },

    /// Curved; starts out changing slowly and ends up changing quickly.
    Exponential { start: SignalType, end: SignalType },

    /// Event-driven (TODO)
    #[allow(dead_code)]
    Triggered {
        // TODO: if we implement this, then ControlTrips themselves
        // controllable.
    },
}

impl ControlStep {
    pub fn new_flat(value: SignalType) -> crate::settings::controllers::ControlStep {
        ControlStep::Flat { value }
    }
    pub fn new_slope(
        start: SignalType,
        end: SignalType,
    ) -> crate::settings::controllers::ControlStep {
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ControllerSettings {
    #[serde(rename_all = "kebab-case")]
    Test {
        #[serde(rename = "midi-in")]
        midi_input_channel: MidiChannel,
        #[serde(rename = "midi-out")]
        midi_output_channel: MidiChannel,
    },
    #[serde(rename_all = "kebab-case")]
    Arpeggiator {
        #[serde(rename = "midi-in")]
        midi_input_channel: MidiChannel,
        #[serde(rename = "midi-out")]
        midi_output_channel: MidiChannel,
    },
    #[serde(rename_all = "kebab-case", rename = "lfo")]
    LfoController {
        #[serde(rename = "midi-in")]
        midi_input_channel: MidiChannel,
        #[serde(rename = "midi-out")]
        midi_output_channel: MidiChannel,
        waveform: WaveformType,
        frequency: f32,
    },
    #[serde(rename_all = "kebab-case", rename = "signal-passthrough-controller")]
    SignalPassthroughController {
        #[serde(rename = "midi-in")]
        midi_input_channel: MidiChannel,
        #[serde(rename = "midi-out")]
        midi_output_channel: MidiChannel,
    },
}

impl ControllerSettings {
    pub(crate) fn instantiate(
        &self,
        clock_settings: &ClockSettings,
        load_only_test_entities: bool,
    ) -> (MidiChannel, MidiChannel, Entity) {
        if load_only_test_entities {
            let (midi_input_channel, midi_output_channel) = match self {
                ControllerSettings::Test {
                    midi_input_channel,
                    midi_output_channel,
                }
                | ControllerSettings::Arpeggiator {
                    midi_input_channel,
                    midi_output_channel,
                }
                | ControllerSettings::LfoController {
                    midi_input_channel,
                    midi_output_channel,
                    ..
                }
                | ControllerSettings::SignalPassthroughController {
                    midi_input_channel,
                    midi_output_channel,
                    ..
                } => (midi_input_channel, midi_output_channel),
            };
            return (
                *midi_input_channel,
                *midi_output_channel,
                Entity::TestController(Box::new(TestController::new_with(
                    clock_settings,
                    *midi_output_channel,
                ))),
            );
        }
        match *self {
            ControllerSettings::Test {
                midi_input_channel,
                midi_output_channel,
            } => (
                midi_input_channel,
                midi_output_channel,
                Entity::TestController(Box::new(TestController::new_with(
                    clock_settings,
                    midi_output_channel,
                ))),
            ),
            ControllerSettings::Arpeggiator {
                midi_input_channel,
                midi_output_channel,
            } => (
                midi_input_channel,
                midi_output_channel,
                Entity::Arpeggiator(Box::new(Arpeggiator::new_with(
                    clock_settings,
                    midi_output_channel,
                ))),
            ),
            ControllerSettings::LfoController {
                midi_input_channel,
                midi_output_channel,
                waveform,
                frequency,
            } => (
                midi_input_channel,
                midi_output_channel,
                Entity::LfoController(Box::new(LfoController::new_with(
                    clock_settings,
                    waveform,
                    frequency as f64,
                ))),
            ),
            ControllerSettings::SignalPassthroughController {
                midi_input_channel,
                midi_output_channel,
            } => (
                midi_input_channel,
                midi_output_channel,
                Entity::SignalPassthroughController(Box::new(SignalPassthroughController::new())),
            ),
        }
    }
}
