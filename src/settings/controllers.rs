use super::{patches::WaveformType, BeatValueSettings, DeviceId, MidiChannel};
use crate::{entities::Entity, utils::ToyMessageMaker};
use groove_core::{ParameterType, SignalType};
use groove_entities::controllers::{
    Arpeggiator, ControlPath, ControlStep, LfoController, SignalPassthroughController,
};
use groove_toys::ToyController;
use serde::{Deserialize, Serialize};

/// A ControlTrip contains successive ControlSteps. A ControlStep describes how
/// to get from point A in time to point B in time, while controlling/automating
/// the parameter over that time. For example, one ControlStep might say "go
/// from 0.5 to 0.7 linearly from beat twelve to beat sixteen." The ControlTrip
/// knows which target that 0.5-0.7 applies to.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename = "control-step", rename_all = "kebab-case")]
pub enum ControlStepSettings {
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
impl ControlStepSettings {
    pub fn into_control_step(&self) -> ControlStep {
        match self {
            ControlStepSettings::Flat { value } => ControlStep::Flat { value: *value },
            ControlStepSettings::Slope { start, end } => ControlStep::Slope {
                start: *start,
                end: *end,
            },
            ControlStepSettings::Logarithmic { start, end } => ControlStep::Logarithmic {
                start: *start,
                end: *end,
            },
            ControlStepSettings::Exponential { start, end } => ControlStep::Exponential {
                start: *start,
                end: *end,
            },
            ControlStepSettings::Triggered {} => ControlStep::Triggered {},
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ControlPathSettings {
    pub id: DeviceId,
    pub note_value: Option<BeatValueSettings>,
    pub steps: Vec<ControlStepSettings>,
}
impl ControlPathSettings {
    pub fn into_control_path(&self) -> ControlPath {
        let note_value = if let Some(note_value) = &self.note_value {
            Some(note_value.into_beat_value())
        } else {
            None
        };
        let mut r = ControlPath {
            note_value,
            steps: Default::default(),
        };
        for step in self.steps.iter() {
            r.steps.push(step.into_control_step());
        }
        r
    }
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
        sample_rate: usize,
        bpm: ParameterType,
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
                Entity::ToyController(Box::new(ToyController::new_with(
                    sample_rate,
                    bpm,
                    *midi_output_channel,
                    Box::new(ToyMessageMaker {}),
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
                Entity::ToyController(Box::new(ToyController::new_with(
                    sample_rate,
                    bpm,
                    midi_output_channel,
                    Box::new(ToyMessageMaker {}),
                ))),
            ),
            ControllerSettings::Arpeggiator {
                midi_input_channel,
                midi_output_channel,
            } => (
                midi_input_channel,
                midi_output_channel,
                Entity::Arpeggiator(Box::new(Arpeggiator::new_with(
                    sample_rate,
                    bpm,
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
                    sample_rate,
                    waveform.into(),
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
