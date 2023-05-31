// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::{BeatValueSettings, DeviceId, MidiChannel};
use groove_core::{ParameterType, SignalType};
use groove_entities::controllers::{
    Arpeggiator, ArpeggiatorParams, Calculator, CalculatorParams, ControlPath, ControlStep,
    LfoController, LfoControllerParams, MidiChannelParams, SignalPassthroughController,
    ToyController, ToyControllerParams,
};
use groove_orchestration::Entity;
use serde::{Deserialize, Serialize};

#[cfg(feature = "iced-framework")]
use groove_entities::ToyMessageMaker;

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
    pub fn derive_control_step(&self) -> ControlStep {
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

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
#[serde(rename_all = "kebab-case")]
pub struct ControlPathSettings {
    pub id: DeviceId,
    pub note_value: Option<BeatValueSettings>,
    pub steps: Vec<ControlStepSettings>,
}
impl ControlPathSettings {
    pub fn derive_control_path(&self) -> ControlPath {
        let note_value = self
            .note_value
            .as_ref()
            .map(|note_value| note_value.into_beat_value());
        let mut r = ControlPath {
            note_value,
            steps: Default::default(),
        };
        for step in self.steps.iter() {
            r.steps.push(step.derive_control_step());
        }
        r
    }
}

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
#[serde(rename_all = "kebab-case")]
pub struct ControlTargetSettings {
    pub id: DeviceId,
    pub param: String,
}

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
#[serde(rename_all = "kebab-case")]
pub struct ControlTripSettings {
    pub id: DeviceId,
    pub target: ControlTargetSettings,

    #[serde(rename = "paths")]
    pub path_ids: Vec<DeviceId>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
#[serde(rename_all = "kebab-case")]
pub enum ControllerSettings {
    #[serde(rename_all = "kebab-case")]
    Test(MidiChannelParams),
    #[serde(rename_all = "kebab-case")]
    Arpeggiator(MidiChannelParams, ArpeggiatorParams),
    #[serde(rename_all = "kebab-case", rename = "lfo")]
    LfoController(MidiChannelParams, LfoControllerParams),
    #[serde(rename_all = "kebab-case", rename = "signal-passthrough-controller")]
    SignalPassthroughController(MidiChannelParams),
    #[serde(rename_all = "kebab-case", rename = "calculator")]
    Calculator(MidiChannelParams, CalculatorParams),
}

impl ControllerSettings {
    pub(crate) fn instantiate(
        &self,
        bpm: ParameterType,
        load_only_test_entities: bool,
    ) -> (MidiChannel, MidiChannel, Entity) {
        if load_only_test_entities {
            let (midi_input_channel, midi_output_channel) = match self {
                ControllerSettings::Test(
                    MidiChannelParams {
                        midi_in: midi_input_channel,
                        midi_out: midi_output_channel,
                    },
                    ..,
                )
                | ControllerSettings::Arpeggiator(
                    MidiChannelParams {
                        midi_in: midi_input_channel,
                        midi_out: midi_output_channel,
                    },
                    ..,
                )
                | ControllerSettings::LfoController(
                    MidiChannelParams {
                        midi_in: midi_input_channel,
                        midi_out: midi_output_channel,
                    },
                    ..,
                )
                | ControllerSettings::SignalPassthroughController(
                    MidiChannelParams {
                        midi_in: midi_input_channel,
                        midi_out: midi_output_channel,
                    },
                    ..,
                ) => (midi_input_channel, midi_output_channel),
                ControllerSettings::Calculator(
                    MidiChannelParams {
                        midi_in: midi_input_channel,
                        midi_out: midi_output_channel,
                    },
                    ..,
                ) => (midi_input_channel, midi_output_channel),
            };
            return (
                *midi_input_channel,
                *midi_output_channel,
                Entity::ToyController(Box::new(ToyController::new_with(
                    ToyControllerParams {},
                    *midi_output_channel,
                ))),
            );
        }
        match self {
            ControllerSettings::Test(midi) => (
                midi.midi_in,
                midi.midi_out,
                Entity::ToyController(Box::new(ToyController::new_with(
                    ToyControllerParams {},
                    midi.midi_out,
                ))),
            ),
            ControllerSettings::Arpeggiator(midi, params) => (
                midi.midi_in,
                midi.midi_out,
                Entity::Arpeggiator(Box::new(Arpeggiator::new_with(&params, midi.midi_out))),
            ),
            ControllerSettings::LfoController(midi, params) => (
                midi.midi_in,
                midi.midi_out,
                Entity::LfoController(Box::new(LfoController::new_with(&params))),
            ),
            ControllerSettings::SignalPassthroughController(midi) => (
                midi.midi_in,
                midi.midi_out,
                Entity::SignalPassthroughController(Box::new(SignalPassthroughController::new())),
            ),
            ControllerSettings::Calculator(midi, _params) => (
                midi.midi_in,
                midi.midi_out,
                Entity::Integrated(Box::new(Calculator::default())),
            ),
        }
    }
}
