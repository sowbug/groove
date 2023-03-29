// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    time::ClockTimeUnit,
    traits::{IsEffect, TransformsAudio},
    Normal, Sample,
};
use groove_proc_macros::{Nano, Uid};
use std::collections::VecDeque;
use std::fmt::Debug;
use std::str::FromStr;
use strum::EnumCount;
use strum_macros::{Display, EnumCount as EnumCountMacro, EnumString, FromRepr, IntoStaticStr};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

/// An [IsEffect](groove_core::traits::IsEffect) that negates the input signal.
#[derive(Debug, Default, Nano, Uid)]
pub struct ToyEffect {
    uid: usize,

    #[nano]
    my_value: Normal,

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

    pub fn update(&mut self, message: ToyEffectMessage) {
        todo!()
    }

    pub fn my_value(&self) -> Normal {
        self.my_value
    }
}
