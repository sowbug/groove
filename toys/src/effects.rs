// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    time::ClockTimeUnit,
    traits::{IsEffect, Resets, TransformsAudio},
    Normal, Sample,
};
use groove_proc_macros::{Control, Params, Uid};
use std::collections::VecDeque;
use std::fmt::Debug;

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

/// An [IsEffect](groove_core::traits::IsEffect) that negates the input signal.
#[derive(Debug, Default, Control, Params, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct ToyEffect {
    #[cfg_attr(feature = "serialization", serde(skip))]
    uid: usize,

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
}
impl IsEffect for ToyEffect {}
impl TransformsAudio for ToyEffect {
    fn transform_channel(&mut self, _channel: usize, input_sample: Sample) -> Sample {
        /////////////////////// TODO        self.check_values(clock);
        -input_sample
    }
}
impl Resets for ToyEffect {}
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
        }
    }

    #[cfg(feature = "iced-framework")]
    pub fn update(&mut self, message: ToyEffectMessage) {
        match message {
            ToyEffectMessage::ToyEffect(s) => *self = Self::new_with(s),
            _ => self.derived_update(message),
        }
    }

    pub fn my_value(&self) -> Normal {
        self.my_value
    }

    pub fn set_my_value(&mut self, my_value: Normal) {
        self.my_value = my_value;
    }
}
