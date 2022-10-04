use std::{cell::RefCell, rc::Rc};

use crate::common::MonoSample;

use crate::primitives::clock::Clock;
use crate::traits::{IsEffect, SinksAudio, SinksControl, SourcesAudio, TransformsAudio, SinksControlParam};

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
    fn handle_control(&mut self, _clock: &Clock, param: &SinksControlParam) {
        match param {
            SinksControlParam::Primary { value } => self.min = *value,
            SinksControlParam::Secondary { value } => self.max = *value,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        common::MonoSample,
        primitives::{
            clock::Clock,
        }, traits::tests::{TestAlwaysTooLoudDevice, TestAlwaysSameLevelDevice},
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
            limiter.add_audio_source(Rc::new(RefCell::new(TestAlwaysSameLevelDevice::new(0.5))));
            assert_eq!(limiter.source_audio(&clock), 0.5);
        }
        {
            let mut limiter = Limiter::new_with(MIN, MAX);
            limiter.add_audio_source(Rc::new(RefCell::new(TestAlwaysSameLevelDevice::new(-0.8))));
            assert_eq!(limiter.source_audio(&clock), MIN);
        }
        {
            let mut limiter = Limiter::new_with(MIN, MAX);
            limiter.add_audio_source(Rc::new(RefCell::new(TestAlwaysSameLevelDevice::new(0.8))));
            assert_eq!(limiter.source_audio(&clock), MAX);
        }

        // multiple sources
        {
            let mut limiter = Limiter::new_with(MIN, MAX);
            limiter.add_audio_source(Rc::new(RefCell::new(TestAlwaysSameLevelDevice::new(0.2))));
            limiter.add_audio_source(Rc::new(RefCell::new(TestAlwaysSameLevelDevice::new(0.6))));
            assert_eq!(limiter.source_audio(&clock), MAX);
            limiter.add_audio_source(Rc::new(RefCell::new(TestAlwaysSameLevelDevice::new(-1.0))));
            assert_approx_eq!(limiter.source_audio(&clock), -0.2);
            limiter.add_audio_source(Rc::new(RefCell::new(TestAlwaysSameLevelDevice::new(-1.0))));
            assert_eq!(limiter.source_audio(&clock), MIN);
        }
    }
}
