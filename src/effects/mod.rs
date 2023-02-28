pub use bitcrusher::Bitcrusher;
pub use chorus::Chorus;
pub use compressor::Compressor;
pub use delay::Delay;
pub use filter::BiQuadFilter;
pub use filter::FilterParams;
pub use gain::Gain;
use groove_core::Sample;
pub use limiter::Limiter;
pub use mixer::Mixer;
pub use reverb::Reverb;

use crate::controllers::F32ControlValue;
use crate::{
    clock::ClockTimeUnit,
    traits::{Controllable, HasUid, IsEffect, TransformsAudio},
};
use groove_macros::Control;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::str::FromStr;
use strum_macros::{Display, EnumString, FromRepr};

pub(crate) mod bitcrusher;
pub(crate) mod chorus;
pub(crate) mod compressor;
pub(crate) mod delay;
pub(crate) mod filter;
pub(crate) mod gain;
pub(crate) mod limiter;
pub(crate) mod mixer;
pub(crate) mod reverb;

#[derive(Control, Debug, Default)]
pub struct TestMixer {
    uid: usize,
}
impl IsEffect for TestMixer {}
impl HasUid for TestMixer {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}
impl TransformsAudio for TestMixer {
    fn transform_channel(&mut self, _channel: usize, input_sample: Sample) -> Sample {
        input_sample
    }
}

#[derive(Control, Debug, Default)]
pub struct TestEffect {
    uid: usize,

    #[controllable]
    my_value: f32,

    pub checkpoint_values: VecDeque<f32>,
    pub checkpoint: f32,
    pub checkpoint_delta: f32,
    pub time_unit: ClockTimeUnit,
}
impl IsEffect for TestEffect {}
impl TransformsAudio for TestEffect {
    fn transform_channel(&mut self, _channel: usize, input_sample: Sample) -> Sample {
        /////////////////////// TODO        self.check_values(clock);
        -input_sample
    }
}
impl HasUid for TestEffect {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}
// impl TestsValues for TestEffect {
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
impl TestEffect {
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

    pub(crate) fn set_control_my_value(&mut self, my_value: F32ControlValue) {
        self.set_my_value(my_value.0);
    }
}
