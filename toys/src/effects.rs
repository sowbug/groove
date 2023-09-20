// Copyright (c) 2023 Mike Tsao. All rights reserved.

use ensnare::core::{Normal, Sample};
use groove_core::{
    time::{ClockTimeUnit, SampleRate},
    traits::{Configurable, Serializable, TransformsAudio},
};
use groove_proc_macros::{Control, IsEffect, Params, Uid};
use std::collections::VecDeque;
use std::fmt::Debug;

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

/// An [IsEffect](groove_core::traits::IsEffect) that negates the input signal.
#[derive(Debug, Default, Control, IsEffect, Params, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct ToyEffect {
    #[cfg_attr(feature = "serialization", serde(skip))]
    uid: groove_core::Uid,

    #[control]
    #[params]
    my_value: Normal,

    #[cfg_attr(feature = "serialization", serde(skip))]
    pub checkpoint_values: VecDeque<f32>,
    #[cfg_attr(feature = "serialization", serde(skip))]
    pub checkpoint: f32,
    #[cfg_attr(feature = "serialization", serde(skip))]
    pub checkpoint_delta: f32,
    #[cfg_attr(feature = "serialization", serde(skip))]
    pub time_unit: ClockTimeUnit,
    #[cfg_attr(feature = "serialization", serde(skip))]
    sample_rate: SampleRate,
}
impl TransformsAudio for ToyEffect {
    fn transform_channel(&mut self, _channel: usize, input_sample: Sample) -> Sample {
        /////////////////////// TODO        self.check_values(clock);
        -input_sample
    }
}
impl Configurable for ToyEffect {
    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }
}
impl Serializable for ToyEffect {}

#[cfg(feature = "egui-framework")]
mod gui {
    use super::ToyEffect;
    use eframe::egui;
    use groove_core::traits::{gui::Displays, HasUid};

    impl Displays for ToyEffect {
        fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
            ui.label(self.name())
        }
    }
}

// impl TestsValues for ToyEffect {
//     fn has_checkpoint_values(&self) -> bool {
//         !self.checkpoint_values.is_empty()
//     }

//     fn time_unit(&self) -> &ClockTimeUnit {
//         &self.time_unit
//     }

//     fn checkpoint_time(&self) -> f32 {
//         self.checkpoint
//     }

//     fn advance_checkpoint_time(&mut self) {
//         self.checkpoint += self.checkpoint_delta;
//     }

//     fn value_to_check(&self) -> f32 {
//         self.my_value()
//     }

//     fn pop_checkpoint_value(&mut self) -> Option<f32> {
//         self.checkpoint_values.pop_front()
//     }
// }
impl ToyEffect {
    pub fn new_with_test_values(
        values: &[f32],
        checkpoint: f32,
        checkpoint_delta: f32,
        time_unit: ClockTimeUnit,
    ) -> Self {
        Self {
            checkpoint_values: VecDeque::from(Vec::from(values)),
            checkpoint,
            checkpoint_delta,
            time_unit,
            ..Default::default()
        }
    }

    pub fn new_with(params: &ToyEffectParams) -> Self {
        Self {
            uid: Default::default(),
            my_value: params.my_value(),
            checkpoint_values: Default::default(),
            checkpoint: Default::default(),
            checkpoint_delta: Default::default(),
            time_unit: Default::default(),
            sample_rate: Default::default(),
        }
    }

    pub fn my_value(&self) -> Normal {
        self.my_value
    }

    pub fn set_my_value(&mut self, my_value: Normal) {
        self.my_value = my_value;
    }
}
