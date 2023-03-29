// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    traits::{IsEffect, TransformsAudio},
    BipolarNormal, Sample,
};
use groove_proc_macros::{Nano, Uid};
use std::str::FromStr;
use strum::EnumCount;
use strum_macros::{Display, EnumCount as EnumCountMacro, EnumString, FromRepr, IntoStaticStr};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Nano, Uid)]
pub struct Limiter {
    uid: usize,

    #[nano]
    max: BipolarNormal,
    #[nano]
    min: BipolarNormal,
}
impl Default for Limiter {
    fn default() -> Self {
        Self {
            uid: Default::default(),
            min: BipolarNormal::minimum(), // TODO: this should be a regular Normal, since we don't have negatives
            max: BipolarNormal::maximum(),
        }
    }
}
impl IsEffect for Limiter {}
impl TransformsAudio for Limiter {
    fn transform_channel(&mut self, _channel: usize, input_sample: Sample) -> Sample {
        let sign = input_sample.0.signum();
        Sample::from(
            input_sample
                .0
                .abs()
                .clamp(self.min.value(), self.max.value())
                * sign,
        )
    }
}
impl Limiter {
    pub fn new_with_params(params: NanoLimiter) -> Self {
        Self {
            min: params.min(),
            max: params.max(),
            ..Default::default()
        }
    }

    pub fn update(&mut self, message: LimiterMessage) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use groove_core::{traits::Generates, StereoSample};
    use groove_toys::ToyAudioSource;
    use more_asserts::{assert_gt, assert_lt};

    #[test]
    fn limiter_mainline() {
        // audio sources are at or past boundaries
        assert_gt!(
            ToyAudioSource::new_with(ToyAudioSource::TOO_LOUD).value(),
            StereoSample::MAX
        );
        assert_eq!(
            ToyAudioSource::new_with(ToyAudioSource::LOUD).value(),
            StereoSample::MAX
        );
        assert_eq!(
            ToyAudioSource::new_with(ToyAudioSource::SILENT).value(),
            StereoSample::SILENCE
        );
        assert_eq!(
            ToyAudioSource::new_with(ToyAudioSource::QUIET).value(),
            StereoSample::MIN
        );
        assert_lt!(
            ToyAudioSource::new_with(ToyAudioSource::TOO_QUIET).value(),
            StereoSample::MIN
        );

        // Limiter clamps high and low, and doesn't change values inside the range.
        let mut limiter = Limiter::default();
        assert_eq!(
            limiter.transform_audio(ToyAudioSource::new_with(ToyAudioSource::TOO_LOUD).value()),
            StereoSample::MAX
        );
        assert_eq!(
            limiter.transform_audio(ToyAudioSource::new_with(ToyAudioSource::LOUD).value()),
            StereoSample::MAX
        );
        assert_eq!(
            limiter.transform_audio(ToyAudioSource::new_with(ToyAudioSource::SILENT).value()),
            StereoSample::SILENCE
        );
        assert_eq!(
            limiter.transform_audio(ToyAudioSource::new_with(ToyAudioSource::QUIET).value()),
            StereoSample::MIN
        );
        assert_eq!(
            limiter.transform_audio(ToyAudioSource::new_with(ToyAudioSource::TOO_QUIET).value()),
            StereoSample::MIN
        );
    }

    #[test]
    fn limiter_bias() {
        let mut limiter = Limiter::new_with_params(NanoLimiter {
            min: BipolarNormal::from(0.2),
            max: BipolarNormal::from(0.8),
        });
        assert_eq!(
            limiter.transform_channel(0, Sample::from(0.1)),
            Sample::from(0.2),
            "Limiter failed to clamp min {}",
            0.2
        );
        assert_eq!(
            limiter.transform_channel(0, Sample::from(0.9)),
            Sample::from(0.8),
            "Limiter failed to clamp max {}",
            0.8
        );
        assert_eq!(
            limiter.transform_channel(0, Sample::from(-0.1)),
            Sample::from(-0.2),
            "Limiter failed to clamp min {} for negative values",
            0.2
        );
        assert_eq!(
            limiter.transform_channel(0, Sample::from(-0.9)),
            Sample::from(-0.8),
            "Limiter failed to clamp max {} for negative values",
            0.8
        );
    }
}
