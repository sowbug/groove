use crate::{
    common::{rrc, rrc_downgrade, MonoSample, Rrc, Ww, MONO_SAMPLE_MAX, MONO_SAMPLE_MIN},
    traits::{HasOverhead, IsEffect, Overhead, SinksAudio, SourcesAudio, TransformsAudio},
};

#[derive(Debug, Default)]
pub struct Limiter {
    pub(crate) me: Ww<Self>,
    overhead: Overhead,

    sources: Vec<Ww<dyn SourcesAudio>>,

    min: MonoSample,
    max: MonoSample,
}
impl IsEffect for Limiter {}

impl Limiter {
    #[allow(dead_code)]
    fn new() -> Self {
        Self::new_with(MONO_SAMPLE_MIN, MONO_SAMPLE_MAX)
    }
    fn new_with(min: MonoSample, max: MonoSample) -> Self {
        Self {
            min,
            max,
            ..Default::default()
        }
    }

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
impl SinksAudio for Limiter {
    fn sources(&self) -> &[Ww<dyn SourcesAudio>] {
        &self.sources
    }
    fn sources_mut(&mut self) -> &mut Vec<Ww<dyn SourcesAudio>> {
        &mut self.sources
    }
}
impl TransformsAudio for Limiter {
    fn transform_audio(&mut self, input_sample: MonoSample) -> MonoSample {
        input_sample.clamp(self.min, self.max)
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
        common::MonoSample,
        utils::tests::{TestAudioSourceAlwaysSameLevel, TestAudioSourceAlwaysTooLoud},
    };
    use assert_approx_eq::assert_approx_eq;

    #[test]
    fn test_limiter_mainline() {
        const MAX: MonoSample = 0.9;
        let mut limiter = Limiter::new_with(0.0, MAX);
        let source = rrc(TestAudioSourceAlwaysTooLoud::new());
        limiter.add_audio_source(rrc_downgrade::<TestAudioSourceAlwaysTooLoud>(&source));
        assert_eq!(limiter.source_audio(&Clock::new()), MAX);
    }

    #[test]
    fn test_limiter() {
        const MIN: MonoSample = -0.75;
        const MAX: MonoSample = -MIN;
        let clock = Clock::new_test();
        {
            let mut limiter = Limiter::new_with(MIN, MAX);
            let source = rrc(TestAudioSourceAlwaysSameLevel::new_with(0.5));
            limiter.add_audio_source(rrc_downgrade::<TestAudioSourceAlwaysSameLevel>(&source));
            assert_eq!(limiter.source_audio(&clock), 0.5);
        }
        {
            let mut limiter = Limiter::new_with(MIN, MAX);
            let source = rrc(TestAudioSourceAlwaysSameLevel::new_with(-0.8));
            limiter.add_audio_source(rrc_downgrade::<TestAudioSourceAlwaysSameLevel>(&source));
            assert_eq!(limiter.source_audio(&clock), MIN);
        }
        {
            let mut limiter = Limiter::new_with(MIN, MAX);
            let source = rrc(TestAudioSourceAlwaysSameLevel::new_with(0.8));
            limiter.add_audio_source(rrc_downgrade::<TestAudioSourceAlwaysSameLevel>(&source));
            assert_eq!(limiter.source_audio(&clock), MAX);
        }

        // multiple sources
        {
            let mut limiter = Limiter::new_with(MIN, MAX);
            let source = rrc(TestAudioSourceAlwaysSameLevel::new_with(0.2));
            limiter.add_audio_source(rrc_downgrade::<TestAudioSourceAlwaysSameLevel>(&source));
            let source = rrc(TestAudioSourceAlwaysSameLevel::new_with(0.6));
            limiter.add_audio_source(rrc_downgrade::<TestAudioSourceAlwaysSameLevel>(&source));
            assert_eq!(limiter.source_audio(&clock), MAX);
            let source = rrc(TestAudioSourceAlwaysSameLevel::new_with(-1.0));
            limiter.add_audio_source(rrc_downgrade::<TestAudioSourceAlwaysSameLevel>(&source));
            assert_approx_eq!(limiter.source_audio(&clock), -0.2);
            let source = rrc(TestAudioSourceAlwaysSameLevel::new_with(-1.0));
            limiter.add_audio_source(rrc_downgrade::<TestAudioSourceAlwaysSameLevel>(&source));
            assert_eq!(limiter.source_audio(&clock), MIN);
        }
    }
}
