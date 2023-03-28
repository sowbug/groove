// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    traits::{IsEffect, TransformsAudio},
    Normal, Sample,
};
use groove_proc_macros::{Control, Synchronization, Uid};
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
    serde(rename = "gain", rename_all = "kebab-case")
)]
pub struct GainParams {
    #[sync]
    pub ceiling: Normal,
}

#[derive(Control, Debug, Default, Uid)]
pub struct Gain {
    uid: usize,

    params: GainParams,

    #[controllable]
    ceiling: Normal,
}
impl IsEffect for Gain {}
impl TransformsAudio for Gain {
    fn transform_channel(&mut self, _channel: usize, input_sample: Sample) -> Sample {
        Sample(input_sample.0 * self.ceiling.value())
    }
}
impl Gain {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            ceiling: Normal::new(1.0),
            ..Default::default()
        }
    }

    pub fn new_with(ceiling: Normal) -> Self {
        Self {
            ceiling,
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    pub fn ceiling(&self) -> Normal {
        self.ceiling
    }

    pub fn set_ceiling(&mut self, ceiling: Normal) {
        self.ceiling = ceiling;
    }

    pub fn set_control_ceiling(&mut self, value: groove_core::control::F32ControlValue) {
        self.set_ceiling(Normal::new_from_f32(value.0));
    }

    pub fn params(&self) -> GainParams {
        self.params
    }

    pub fn update(&mut self, message: GainParamsMessage) {
        self.params.update(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use groove_core::{traits::Generates, StereoSample};
    use groove_toys::ToyAudioSource;

    #[test]
    fn test_gain_mainline() {
        let mut gain = Gain::new_with(Normal::new(0.5));
        assert_eq!(
            gain.transform_audio(ToyAudioSource::new_with(ToyAudioSource::LOUD).value()),
            StereoSample::from(0.5)
        );
    }
}
