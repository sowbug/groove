use std::{cell::RefCell, rc::Rc};

use crate::common::MonoSample;

use super::{IsEffect, SinksAudio, SinksControl, SourcesAudio, TransformsAudio};

#[derive(Debug, Default)]
pub struct Limiter {
    sources: Vec<Rc<RefCell<dyn SourcesAudio>>>,

    min: MonoSample,
    max: MonoSample,
}
impl IsEffect for Limiter {}
impl Limiter {
    pub fn new_with(min: MonoSample, max: MonoSample) -> Self {
        Self {
            min,
            max,
            ..Default::default()
        }
    }
}
impl SinksAudio for Limiter {
    fn sources(&mut self) -> &mut Vec<Rc<RefCell<dyn SourcesAudio>>> {
        &mut self.sources
    }
}
impl TransformsAudio for Limiter {
    fn transform_audio(&mut self, input_sample: MonoSample) -> MonoSample {
        input_sample.clamp(self.min, self.max)
    }
}
impl SinksControl for Limiter {
    fn handle_control(&mut self, _clock: &super::clock::Clock, param: &super::SinksControlParam) {
        match param {
            super::SinksControlParam::Primary { value } => self.min = *value,
            super::SinksControlParam::Secondary { value } => self.max = *value,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        common::MonoSample,
        primitives::{
            clock::Clock,
            tests::{SingleLevelDevice, TestAlwaysTooLoudDevice},
        },
    };
    use assert_approx_eq::assert_approx_eq;

    use super::*;

    #[test]
    fn test_limiter_mainline() {
        const MAX: MonoSample = 0.9;
        let mut limiter = Limiter::new_with(0.0, MAX);
        limiter.add_audio_source(Rc::new(RefCell::new(TestAlwaysTooLoudDevice::new())));
        assert_eq!(limiter.source_audio(&Clock::new()), MAX);
    }

    #[test]
    fn test_limiter() {
        const MIN: MonoSample = -0.75;
        const MAX: MonoSample = -MIN;
        let clock = Clock::new_test();
        {
            let mut limiter = Limiter::new_with(MIN, MAX);
            limiter.add_audio_source(Rc::new(RefCell::new(SingleLevelDevice::new(0.5))));
            assert_eq!(limiter.source_audio(&clock), 0.5);
        }
        {
            let mut limiter = Limiter::new_with(MIN, MAX);
            limiter.add_audio_source(Rc::new(RefCell::new(SingleLevelDevice::new(-0.8))));
            assert_eq!(limiter.source_audio(&clock), MIN);
        }
        {
            let mut limiter = Limiter::new_with(MIN, MAX);
            limiter.add_audio_source(Rc::new(RefCell::new(SingleLevelDevice::new(0.8))));
            assert_eq!(limiter.source_audio(&clock), MAX);
        }

        // multiple sources
        {
            let mut limiter = Limiter::new_with(MIN, MAX);
            limiter.add_audio_source(Rc::new(RefCell::new(SingleLevelDevice::new(0.2))));
            limiter.add_audio_source(Rc::new(RefCell::new(SingleLevelDevice::new(0.6))));
            assert_eq!(limiter.source_audio(&clock), MAX);
            limiter.add_audio_source(Rc::new(RefCell::new(SingleLevelDevice::new(-1.0))));
            assert_approx_eq!(limiter.source_audio(&clock), -0.2);
            limiter.add_audio_source(Rc::new(RefCell::new(SingleLevelDevice::new(-1.0))));
            assert_eq!(limiter.source_audio(&clock), MIN);
        }
    }
}
