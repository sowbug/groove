use crate::{
    clock::Clock,
    common::{F32ControlValue, Sample, SampleType},
    messages::EntityMessage,
    traits::{Controllable, HasUid, IsEffect, TransformsAudio, Updateable},
    BipolarNormal,
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
            min: BipolarNormal::MIN as f32,
            max: BipolarNormal::MAX as f32,
        }
    }
}
impl IsEffect for Limiter {}
impl TransformsAudio for Limiter {
    fn transform_channel(
        &mut self,
        _clock: &Clock,
        _channel: usize,
        input_sample: crate::common::Sample,
    ) -> crate::common::Sample {
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
impl Updateable for Limiter {
    type Message = EntityMessage;
}

impl Limiter {
    #[allow(dead_code)]
    fn new() -> Self {
        Self::default()
    }

    pub(crate) fn new_with(min: BipolarNormal, max: BipolarNormal) -> Self {
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

    pub(crate) fn set_control_min(&mut self, value: F32ControlValue) {
        self.set_min(value.0);
    }

    pub(crate) fn set_control_max(&mut self, value: F32ControlValue) {
        self.set_max(value.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        clock::Clock, common::Sample, messages::tests::TestMessage, traits::SourcesAudio,
        utils::AudioSource, StereoSample,
    };
    use more_asserts::{assert_gt, assert_lt};

    #[test]
    fn limiter_mainline() {
        let clock = Clock::default();

        // audio sources are at or past boundaries
        assert_gt!(
            AudioSource::<TestMessage>::new_with(AudioSource::<TestMessage>::TOO_LOUD)
                .source_audio(&clock),
            StereoSample::MAX
        );
        assert_eq!(
            AudioSource::<TestMessage>::new_with(AudioSource::<TestMessage>::LOUD)
                .source_audio(&clock),
            StereoSample::MAX
        );
        assert_eq!(
            AudioSource::<TestMessage>::new_with(AudioSource::<TestMessage>::SILENT)
                .source_audio(&clock),
            StereoSample::SILENCE
        );
        assert_eq!(
            AudioSource::<TestMessage>::new_with(AudioSource::<TestMessage>::QUIET)
                .source_audio(&clock),
            StereoSample::MIN
        );
        assert_lt!(
            AudioSource::<TestMessage>::new_with(AudioSource::<TestMessage>::TOO_QUIET)
                .source_audio(&clock),
            StereoSample::MIN
        );

        // Limiter clamps high and low, and doesn't change values inside the range.
        let mut limiter = Limiter::default();
        assert_eq!(
            limiter.transform_audio(
                &clock,
                AudioSource::<TestMessage>::new_with(AudioSource::<TestMessage>::TOO_LOUD)
                    .source_audio(&clock)
            ),
            StereoSample::MAX
        );
        assert_eq!(
            limiter.transform_audio(
                &clock,
                AudioSource::<TestMessage>::new_with(AudioSource::<TestMessage>::LOUD)
                    .source_audio(&clock)
            ),
            StereoSample::MAX
        );
        assert_eq!(
            limiter.transform_audio(
                &clock,
                AudioSource::<TestMessage>::new_with(AudioSource::<TestMessage>::SILENT)
                    .source_audio(&clock)
            ),
            StereoSample::SILENCE
        );
        assert_eq!(
            limiter.transform_audio(
                &clock,
                AudioSource::<TestMessage>::new_with(AudioSource::<TestMessage>::QUIET)
                    .source_audio(&clock)
            ),
            StereoSample::MIN
        );
        assert_eq!(
            limiter.transform_audio(
                &clock,
                AudioSource::<TestMessage>::new_with(AudioSource::<TestMessage>::TOO_QUIET)
                    .source_audio(&clock)
            ),
            StereoSample::MIN
        );
    }

    #[test]
    fn limiter_bias() {
        let clock = Clock::default();

        let mut limiter = Limiter::new_with(BipolarNormal::from(0.2), BipolarNormal::from(0.8));
        assert_eq!(
            limiter.transform_channel(&clock, 0, Sample::from(0.1f32)),
            Sample::from(0.2f32),
            "Limiter failed to clamp min {}",
            0.2
        );
        assert_eq!(
            limiter.transform_channel(&clock, 0, Sample::from(0.9f32)),
            Sample::from(0.8f32),
            "Limiter failed to clamp max {}",
            0.8
        );
        assert_eq!(
            limiter.transform_channel(&clock, 0, Sample::from(-0.1f32)),
            Sample::from(-0.2f32),
            "Limiter failed to clamp min {} for negative values",
            0.2
        );
        assert_eq!(
            limiter.transform_channel(&clock, 0, Sample::from(-0.9f32)),
            Sample::from(-0.8f32),
            "Limiter failed to clamp max {} for negative values",
            0.8
        );
    }
}
