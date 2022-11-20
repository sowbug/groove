use crate::{
    clock::Clock,
    common::{rrc, rrc_downgrade, MonoSample, Rrc, Ww, MONO_SAMPLE_MAX, MONO_SAMPLE_MIN},
    messages::GrooveMessage,
    traits::{
        HasOverhead, HasUid, NewIsEffect, NewUpdateable, Overhead, SourcesAudio, TransformsAudio,
    },
};

#[derive(Debug, Default)]
pub struct Limiter {
    uid: usize,
    pub(crate) me: Ww<Self>,
    overhead: Overhead,

    min: MonoSample,
    max: MonoSample,
}
impl NewIsEffect for Limiter {}
impl TransformsAudio for Limiter {
    fn transform_audio(&mut self, _clock: &Clock, input_sample: MonoSample) -> MonoSample {
        input_sample.clamp(self.min, self.max)
    }
}
impl NewUpdateable for Limiter {
    type Message = GrooveMessage;
}
impl HasUid for Limiter {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
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

    #[deprecated]
    pub(crate) fn new_wrapped_with(min: MonoSample, max: MonoSample) -> Rrc<Self> {
        let wrapped = rrc(Self::new_with(min, max));
        wrapped.borrow_mut().me = rrc_downgrade(&wrapped);
        wrapped
    }

    pub(crate) fn min(&self) -> f32 {
        self.min
    }

    pub(crate) fn max(&self) -> f32 {
        self.max
    }

    pub(crate) fn set_min(&mut self, value: f32) {
        self.min = value;
    }

    pub(crate) fn set_max(&mut self, value: f32) {
        self.max = value;
    }
}
impl HasOverhead for Limiter {
    fn overhead(&self) -> &Overhead {
        &self.overhead
    }

    fn overhead_mut(&mut self) -> &mut Overhead {
        &mut self.overhead
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        clock::Clock,
        common::MONO_SAMPLE_SILENCE,
        messages::tests::TestMessage,
        utils::tests::{
            TestAudioSourceAlwaysLoud, TestAudioSourceAlwaysSilent, TestAudioSourceAlwaysTooLoud,
            TestAudioSourceAlwaysVeryQuiet,
        },
    };
    use more_asserts::{assert_gt, assert_lt};

    #[test]
    fn test_limiter_mainline() {
        let clock = Clock::default();

        // audio sources are at or past boundaries
        assert_gt!(
            TestAudioSourceAlwaysTooLoud::<TestMessage>::default().source_audio(&clock),
            MONO_SAMPLE_MAX
        );
        assert_eq!(
            TestAudioSourceAlwaysLoud::<TestMessage>::default().source_audio(&clock),
            MONO_SAMPLE_MAX
        );
        assert_eq!(
            TestAudioSourceAlwaysSilent::<TestMessage>::default().source_audio(&clock),
            MONO_SAMPLE_SILENCE
        );
        assert_lt!(
            TestAudioSourceAlwaysVeryQuiet::<TestMessage>::default().source_audio(&clock),
            MONO_SAMPLE_SILENCE
        );

        // Limiter clamps high and low, and doesn't change values inside the range.
        let mut limiter = Limiter::new_with(MONO_SAMPLE_SILENCE, MONO_SAMPLE_MAX);
        assert_eq!(
            limiter.transform_audio(
                &clock,
                TestAudioSourceAlwaysLoud::<TestMessage>::default().source_audio(&clock)
            ),
            MONO_SAMPLE_MAX
        );
        assert_eq!(
            limiter.transform_audio(
                &clock,
                TestAudioSourceAlwaysTooLoud::<TestMessage>::default().source_audio(&clock)
            ),
            MONO_SAMPLE_MAX
        );
        assert_eq!(
            limiter.transform_audio(
                &clock,
                TestAudioSourceAlwaysVeryQuiet::<TestMessage>::default().source_audio(&clock)
            ),
            MONO_SAMPLE_SILENCE
        );
        assert_eq!(
            limiter.transform_audio(
                &clock,
                TestAudioSourceAlwaysSilent::<TestMessage>::default().source_audio(&clock)
            ),
            MONO_SAMPLE_SILENCE
        );
    }
}
