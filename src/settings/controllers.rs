use serde::{Deserialize, Serialize};

use crate::{
    clock::BeatValue,
    common::DeviceId,
    controllers::arpeggiator::Arpeggiator,
    traits::{IsController, TestController},
    GrooveMessage,
};

use super::MidiChannel;

/// A ControlTrip contains successive ControlSteps. A ControlStep describes how
/// to get from point A in time to point B in time, while controlling/automating
/// the parameter over that time. For example, one ControlStep might say "go
/// from 0.5 to 0.7 linearly from beat twelve to beat sixteen." The ControlTrip
/// knows which target that 0.5-0.7 applies to.
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

    // curved, but it starts out fast and ends up slow.
    Logarithmic {
        start: f32,
        end: f32,
    },

    // curved, but it starts out slow and ends up fast.
    Exponential {
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
    pub fn new_flat(value: f32) -> crate::settings::controllers::ControlStep {
        ControlStep::Flat { value }
    }
    pub fn new_slope(start: f32, end: f32) -> crate::settings::controllers::ControlStep {
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
}

impl ControllerSettings {
    pub(crate) fn instantiate(
        &self,
        load_only_test_entities: bool,
    ) -> (
        MidiChannel,
        MidiChannel,
        Box<dyn IsController<Message = GrooveMessage, ViewMessage = GrooveMessage>>,
    ) {
        if load_only_test_entities {
            let (midi_input_channel, midi_output_channel) = match self {
                ControllerSettings::Test {
                    midi_input_channel,
                    midi_output_channel,
                } => (midi_input_channel, midi_output_channel),
                ControllerSettings::Arpeggiator {
                    midi_input_channel,
                    midi_output_channel,
                } => (midi_input_channel, midi_output_channel),
            };
            return (
                *midi_input_channel,
                *midi_output_channel,
                Box::new(TestController::<GrooveMessage>::new_with(
                    *midi_output_channel,
                )),
            );
        }
        match *self {
            ControllerSettings::Test {
                midi_input_channel,
                midi_output_channel,
            } => (
                midi_input_channel,
                midi_output_channel,
                Box::new(TestController::<GrooveMessage>::new_with(
                    midi_output_channel,
                )),
            ),
            ControllerSettings::Arpeggiator {
                midi_input_channel,
                midi_output_channel,
            } => (
                midi_input_channel,
                midi_output_channel,
                Box::new(Arpeggiator::new_with(midi_output_channel)),
            ),
        }
    }
}
