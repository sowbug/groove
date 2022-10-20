use crate::{
    common::{rrc, MonoSample, Rrc, Ww, MONO_SAMPLE_MAX, MONO_SAMPLE_MIN},
    traits::{IsEffect, IsMutable, SinksAudio, SourcesAudio, TransformsAudio},
};
use std::rc::Rc;

#[derive(Debug, Default)]
pub struct Limiter {
    pub(crate) me: Ww<Self>,
    sources: Vec<Ww<dyn SourcesAudio>>,
    is_muted: bool,

    min: MonoSample,
    max: MonoSample,
}
impl IsEffect for Limiter {}
impl Limiter {
    pub(crate) const CONTROL_PARAM_MIN: &str = "min";
    pub(crate) const CONTROL_PARAM_MAX: &str = "max";

    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::new_with(MONO_SAMPLE_MIN, MONO_SAMPLE_MAX)
    }
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
impl IsMutable for Limiter {
    fn is_muted(&self) -> bool {
        self.is_muted
    }

    fn set_muted(&mut self, is_muted: bool) {
        self.is_muted = is_muted;
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
        let source = Rc::new(RefCell::new(TestAudioSourceAlwaysTooLoud::new()));
        let source = Rc::downgrade(&source);
        limiter.add_audio_source(source);
        assert_eq!(limiter.source_audio(&Clock::new()), MAX);
    }

    #[test]
    fn test_limiter() {
        const MIN: MonoSample = -0.75;
        const MAX: MonoSample = -MIN;
        let clock = Clock::new_test();
        {
            let mut limiter = Limiter::new_with(MIN, MAX);
            let source = Rc::new(RefCell::new(TestAudioSourceAlwaysSameLevel::new(0.5)));
            let source = Rc::downgrade(&source);
            limiter.add_audio_source(source);
            assert_eq!(limiter.source_audio(&clock), 0.5);
        }
        {
            let mut limiter = Limiter::new_with(MIN, MAX);
            let source = Rc::new(RefCell::new(TestAudioSourceAlwaysSameLevel::new(-0.8)));
            let source = Rc::downgrade(&source);
            limiter.add_audio_source(source);
            assert_eq!(limiter.source_audio(&clock), MIN);
        }
        {
            let mut limiter = Limiter::new_with(MIN, MAX);
            let source = Rc::new(RefCell::new(TestAudioSourceAlwaysSameLevel::new(0.8)));
            let source = Rc::downgrade(&source);
            limiter.add_audio_source(source);
            assert_eq!(limiter.source_audio(&clock), MAX);
        }

        // multiple sources
        {
            let mut limiter = Limiter::new_with(MIN, MAX);
            let source = Rc::new(RefCell::new(TestAudioSourceAlwaysSameLevel::new(0.2)));
            let source = Rc::downgrade(&source);
            limiter.add_audio_source(source);
            let source = Rc::new(RefCell::new(TestAudioSourceAlwaysSameLevel::new(0.6)));
            let source = Rc::downgrade(&source);
            limiter.add_audio_source(source);
            assert_eq!(limiter.source_audio(&clock), MAX);
            let source = Rc::new(RefCell::new(TestAudioSourceAlwaysSameLevel::new(-1.0)));
            let source = Rc::downgrade(&source);
            limiter.add_audio_source(source);
            assert_approx_eq!(limiter.source_audio(&clock), -0.2);
            let source = Rc::new(RefCell::new(TestAudioSourceAlwaysSameLevel::new(-1.0)));
            let source = Rc::downgrade(&source);
            limiter.add_audio_source(source);
            assert_eq!(limiter.source_audio(&clock), MIN);
        }
    }
}
