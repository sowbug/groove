use crate::{
    common::{rrc, MonoSample, Rrc, Ww},
    traits::{IsEffect, SinksAudio, SourcesAudio, TransformsAudio},
};
use std::rc::Rc;

#[derive(Debug, Default)]
pub struct Limiter {
    pub(crate) me: Ww<Self>,
    sources: Vec<Rrc<dyn SourcesAudio>>,

    min: MonoSample,
    max: MonoSample,
}
impl IsEffect for Limiter {}
impl Limiter {
    pub(crate) const CONTROL_PARAM_MIN: &str = "min";
    pub(crate) const CONTROL_PARAM_MAX: &str = "max";

    pub fn new_with(min: MonoSample, max: MonoSample) -> Self {
        Self {
            min,
            max,
            ..Default::default()
        }
    }
    pub fn new_wrapped_with(min: MonoSample, max: MonoSample) -> Rrc<Self> {
        let wrapped = rrc(Self::new_with(min, max));
        wrapped.borrow_mut().me = Rc::downgrade(&wrapped);
        wrapped
    }

    pub fn set_min(&mut self, value: f32) {
        self.min = value;
    }

    pub fn set_max(&mut self, value: f32) {
        self.max = value;
    }
}
impl SinksAudio for Limiter {
    fn sources(&self) -> &[Rrc<dyn SourcesAudio>] {
        &self.sources
    }
    fn sources_mut(&mut self) -> &mut Vec<Rrc<dyn SourcesAudio>> {
        &mut self.sources
    }
}
impl TransformsAudio for Limiter {
    fn transform_audio(&mut self, input_sample: MonoSample) -> MonoSample {
        input_sample.clamp(self.min, self.max)
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
    use std::cell::RefCell;

    #[test]
    fn test_limiter_mainline() {
        const MAX: MonoSample = 0.9;
        let mut limiter = Limiter::new_with(0.0, MAX);
        limiter.add_audio_source(Rc::new(RefCell::new(TestAudioSourceAlwaysTooLoud::new())));
        assert_eq!(limiter.source_audio(&Clock::new()), MAX);
    }

    #[test]
    fn test_limiter() {
        const MIN: MonoSample = -0.75;
        const MAX: MonoSample = -MIN;
        let clock = Clock::new_test();
        {
            let mut limiter = Limiter::new_with(MIN, MAX);
            limiter.add_audio_source(Rc::new(RefCell::new(TestAudioSourceAlwaysSameLevel::new(
                0.5,
            ))));
            assert_eq!(limiter.source_audio(&clock), 0.5);
        }
        {
            let mut limiter = Limiter::new_with(MIN, MAX);
            limiter.add_audio_source(Rc::new(RefCell::new(TestAudioSourceAlwaysSameLevel::new(
                -0.8,
            ))));
            assert_eq!(limiter.source_audio(&clock), MIN);
        }
        {
            let mut limiter = Limiter::new_with(MIN, MAX);
            limiter.add_audio_source(Rc::new(RefCell::new(TestAudioSourceAlwaysSameLevel::new(
                0.8,
            ))));
            assert_eq!(limiter.source_audio(&clock), MAX);
        }

        // multiple sources
        {
            let mut limiter = Limiter::new_with(MIN, MAX);
            limiter.add_audio_source(Rc::new(RefCell::new(TestAudioSourceAlwaysSameLevel::new(
                0.2,
            ))));
            limiter.add_audio_source(Rc::new(RefCell::new(TestAudioSourceAlwaysSameLevel::new(
                0.6,
            ))));
            assert_eq!(limiter.source_audio(&clock), MAX);
            limiter.add_audio_source(Rc::new(RefCell::new(TestAudioSourceAlwaysSameLevel::new(
                -1.0,
            ))));
            assert_approx_eq!(limiter.source_audio(&clock), -0.2);
            limiter.add_audio_source(Rc::new(RefCell::new(TestAudioSourceAlwaysSameLevel::new(
                -1.0,
            ))));
            assert_eq!(limiter.source_audio(&clock), MIN);
        }
    }
}
