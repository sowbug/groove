// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    time::ClockTimeUnit,
    traits::{IsEffect, TransformsAudio},
    Normal, Sample,
};
use groove_proc_macros::{Control, Synchronization, Uid};
use std::collections::VecDeque;
use std::fmt::Debug;
use std::str::FromStr;
use strum::EnumCount;
use strum_macros::{
    Display, EnumCount as EnumCountMacro, EnumIter, EnumString, FromRepr, IntoStaticStr,
};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Synchronization)]
#[cfg_attr(
    feature = "serialization",
    derive(Serialize, Deserialize),
    serde(rename = "toy-effect", rename_all = "kebab-case")
)]
pub struct ToyEffectParams {
    #[sync]
    pub my_value: Normal,
}
impl ToyEffectParams {
    pub fn my_value(&self) -> Normal {
        self.my_value
    }

    pub fn set_my_value(&mut self, my_value: Normal) {
        self.my_value = my_value;
    }
}

/// An [IsEffect](groove_core::traits::IsEffect) that negates the input signal.
#[derive(Control, Debug, Default, Uid)]
pub struct ToyEffect {
    uid: usize,
    params: ToyEffectParams,

    #[controllable]
    my_value: f32,

    pub checkpoint_values: VecDeque<f32>,
    pub checkpoint: f32,
    pub checkpoint_delta: f32,
    pub time_unit: ClockTimeUnit,
}
impl IsEffect for ToyEffect {}
impl TransformsAudio for ToyEffect {
    fn transform_channel(&mut self, _channel: usize, input_sample: Sample) -> Sample {
        /////////////////////// TODO        self.check_values(clock);
        -input_sample
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

    pub fn set_my_value(&mut self, my_value: f32) {
        self.my_value = my_value;
    }

    pub fn my_value(&self) -> f32 {
        self.my_value
    }

    pub fn set_control_my_value(&mut self, my_value: groove_core::control::F32ControlValue) {
        self.set_my_value(my_value.0);
    }

    pub fn params(&self) -> ToyEffectParams {
        self.params
    }

    pub fn update(&mut self, message: ToyEffectParamsMessage) {
        self.params.update(message)
    }
}
