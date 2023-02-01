use crate::{
    clock::Clock,
    common::{F32ControlValue, MonoSample, MONO_SAMPLE_MAX, MONO_SAMPLE_MIN},
    messages::EntityMessage,
    traits::{Controllable, HasUid, IsEffect, TransformsAudio, Updateable},
};
use groove_macros::{Control, Uid};
use std::str::FromStr;
use strum_macros::{Display, EnumString, FromRepr};

#[derive(Control, Debug, Default, Uid)]
pub struct Limiter {
    uid: usize,

    #[controllable]
    min: MonoSample,
    #[controllable]
    max: MonoSample,
}
impl IsEffect for Limiter {}
impl TransformsAudio for Limiter {
    fn transform_audio(&mut self, _clock: &Clock, input_sample: MonoSample) -> MonoSample {
        let sign = input_sample.signum();
        input_sample.abs().clamp(self.min, self.max) * sign
    }
}
impl Updateable for Limiter {
    type Message = EntityMessage;
}

impl Limiter {
    #[allow(dead_code)]
    fn new() -> Self {
        Self::new_with(MONO_SAMPLE_MIN, MONO_SAMPLE_MAX)
    }
    pub(crate) fn new_with(min: MonoSample, max: MonoSample) -> Self {
        Self {
            min,
            max,
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
        clock::Clock, common::MONO_SAMPLE_SILENCE, messages::tests::TestMessage,
        traits::SourcesAudio, utils::AudioSource,
    };
    use more_asserts::{assert_gt, assert_lt};

    #[test]
    fn limiter_mainline() {
        let clock = Clock::default();

        // audio sources are at or past boundaries
        assert_gt!(
            AudioSource::<TestMessage>::new_with(AudioSource::<TestMessage>::TOO_LOUD)
                .source_audio(&clock),
            MONO_SAMPLE_MAX
        );
        assert_eq!(
            AudioSource::<TestMessage>::new_with(AudioSource::<TestMessage>::LOUD)
                .source_audio(&clock),
            MONO_SAMPLE_MAX
        );
        assert_eq!(
            AudioSource::<TestMessage>::new_with(AudioSource::<TestMessage>::SILENT)
                .source_audio(&clock),
            MONO_SAMPLE_SILENCE
        );
        assert_eq!(
            AudioSource::<TestMessage>::new_with(AudioSource::<TestMessage>::QUIET)
                .source_audio(&clock),
            MONO_SAMPLE_MIN
        );
        assert_lt!(
            AudioSource::<TestMessage>::new_with(AudioSource::<TestMessage>::TOO_QUIET)
                .source_audio(&clock),
            MONO_SAMPLE_MIN
        );

        // Limiter clamps high and low, and doesn't change values inside the range.
        let mut limiter = Limiter::new_with(MONO_SAMPLE_MIN, MONO_SAMPLE_MAX);
        assert_eq!(
            limiter.transform_audio(
                &clock,
                AudioSource::<TestMessage>::new_with(AudioSource::<TestMessage>::TOO_LOUD)
                    .source_audio(&clock)
            ),
            MONO_SAMPLE_MAX
        );
        assert_eq!(
            limiter.transform_audio(
                &clock,
                AudioSource::<TestMessage>::new_with(AudioSource::<TestMessage>::LOUD)
                    .source_audio(&clock)
            ),
            MONO_SAMPLE_MAX
        );
        assert_eq!(
            limiter.transform_audio(
                &clock,
                AudioSource::<TestMessage>::new_with(AudioSource::<TestMessage>::SILENT)
                    .source_audio(&clock)
            ),
            MONO_SAMPLE_SILENCE
        );
        assert_eq!(
            limiter.transform_audio(
                &clock,
                AudioSource::<TestMessage>::new_with(AudioSource::<TestMessage>::QUIET)
                    .source_audio(&clock)
            ),
            MONO_SAMPLE_MIN
        );
        assert_eq!(
            limiter.transform_audio(
                &clock,
                AudioSource::<TestMessage>::new_with(AudioSource::<TestMessage>::TOO_QUIET)
                    .source_audio(&clock)
            ),
            MONO_SAMPLE_MIN
        );
    }

    #[test]
    fn limiter_bias() {
        let clock = Clock::default();

        let mut limiter = Limiter::new_with(0.2, 0.8);
        assert_eq!(
            limiter.transform_audio(&clock, 0.1),
            0.2,
            "Limiter failed to clamp min {}",
            0.2
        );
        assert_eq!(
            limiter.transform_audio(&clock, 0.9),
            0.8,
            "Limiter failed to clamp max {}",
            0.8
        );
        assert_eq!(
            limiter.transform_audio(&clock, -0.1),
            -0.2,
            "Limiter failed to clamp min {} for negative values",
            0.2
        );
        assert_eq!(
            limiter.transform_audio(&clock, -0.9),
            -0.8,
            "Limiter failed to clamp max {} for negative values",
            0.8
        );
    }
}
