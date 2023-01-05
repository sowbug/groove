use crate::{
    clock::Clock,
    common::{MonoSample, MONO_SAMPLE_MAX, MONO_SAMPLE_MIN},
    messages::EntityMessage,
    traits::{HasUid, IsEffect, Response, TransformsAudio, Updateable},
};
use groove_macros::Uid;
use std::str::FromStr;
use strum_macros::{Display, EnumString, FromRepr};

#[derive(Display, Debug, EnumString, FromRepr)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum LimiterControlParams {
    Max,
    Min,
}

#[derive(Debug, Default, Uid)]
pub struct Limiter {
    uid: usize,

    min: MonoSample,
    max: MonoSample,
}
impl IsEffect for Limiter {}
impl TransformsAudio for Limiter {
    fn transform_audio(&mut self, _clock: &Clock, input_sample: MonoSample) -> MonoSample {
        input_sample.clamp(self.min, self.max)
    }
}
impl Updateable for Limiter {
    type Message = EntityMessage;

    fn update(&mut self, _clock: &Clock, message: Self::Message) -> Response<Self::Message> {
        match message {
            Self::Message::UpdateF32(param_id, value) => {
                self.set_indexed_param_f32(param_id, value);
            }
            _ => todo!(),
        }
        Response::none()
    }

    fn param_id_for_name(&self, name: &str) -> usize {
        if let Ok(param) = LimiterControlParams::from_str(name) {
            param as usize
        } else {
            usize::MAX
        }
    }

    fn set_indexed_param_f32(&mut self, index: usize, value: f32) {
        if let Some(param) = LimiterControlParams::from_repr(index) {
            match param {
                LimiterControlParams::Max => self.set_max(value),
                LimiterControlParams::Min => self.set_min(value),
            }
        } else {
            todo!()
        }
    }
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

    pub(crate) fn set_min(&mut self, value: f32) {
        self.min = value;
    }

    pub(crate) fn set_max(&mut self, value: f32) {
        self.max = value;
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
    fn test_limiter_mainline() {
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
}
