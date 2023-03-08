// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    traits::{IsEffect, TransformsAudio},
    BipolarNormal, Sample, SampleType,
};
use groove_macros::{Control, Uid};
use std::str::FromStr;
use strum_macros::{Display, EnumString, FromRepr};

#[derive(Control, Debug, Uid)]
pub struct Limiter {
    uid: usize,

    #[controllable]
    min: f32,
    #[controllable]
    max: f32,
}
impl Default for Limiter {
    fn default() -> Self {
        Self {
            uid: Default::default(),
            min: BipolarNormal::MIN as f32, // TODO: this should be a regular Normal, since we don't have negatives
            max: BipolarNormal::MAX as f32,
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
                .clamp(self.min as SampleType, self.max as SampleType)
                * sign,
        )
    }
}
impl Limiter {
    #[allow(dead_code)]
    fn new() -> Self {
        Self::default()
    }

    pub fn new_with(min: BipolarNormal, max: BipolarNormal) -> Self {
        Self {
            min: min.value() as f32,
            max: max.value() as f32,
            ..Default::default()
        }
    }

    pub fn min(&self) -> f32 {
        self.min
    }

    pub fn max(&self) -> f32 {
        self.max
    }

    pub fn set_min(&mut self, value: f32) {
        self.min = value;
    }

    pub fn set_max(&mut self, value: f32) {
        self.max = value;
    }

    pub fn set_control_min(&mut self, value: groove_core::control::F32ControlValue) {
        self.set_min(value.0);
    }

    pub fn set_control_max(&mut self, value: groove_core::control::F32ControlValue) {
        self.set_max(value.0);
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
        let mut limiter = Limiter::new_with(BipolarNormal::from(0.2), BipolarNormal::from(0.8));
        assert_eq!(
            limiter.transform_channel(0, Sample::from(0.1f32)),
            Sample::from(0.2f32),
            "Limiter failed to clamp min {}",
            0.2
        );
        assert_eq!(
            limiter.transform_channel(0, Sample::from(0.9f32)),
            Sample::from(0.8f32),
            "Limiter failed to clamp max {}",
            0.8
        );
        assert_eq!(
            limiter.transform_channel(0, Sample::from(-0.1f32)),
            Sample::from(-0.2f32),
            "Limiter failed to clamp min {} for negative values",
            0.2
        );
        assert_eq!(
            limiter.transform_channel(0, Sample::from(-0.9f32)),
            Sample::from(-0.8f32),
            "Limiter failed to clamp max {} for negative values",
            0.8
        );
    }
}