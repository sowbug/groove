use std::{rc::Rc, cell::RefCell};

use crate::common::MonoSample;

use super::{SinksAudio, SourcesAudio, TransformsAudio};

#[derive(Default)]
pub struct MiniLimiter {
    sources: Vec<Rc<RefCell<dyn SourcesAudio>>>,

    min: MonoSample,
    max: MonoSample,
}
impl MiniLimiter {
    pub fn new(min: MonoSample, max: MonoSample) -> Self {
        Self {
            min,
            max,
            ..Default::default()
        }
    }
}

impl SinksAudio for MiniLimiter {
    fn sources(&mut self) -> &mut Vec<Rc<RefCell<dyn SourcesAudio>>> {
        &mut self.sources
    }
}

impl TransformsAudio for MiniLimiter {
    fn transform_audio(&mut self, input_sample: MonoSample) -> MonoSample {
        input_sample.clamp(self.min, self.max)
    }
}

#[cfg(test)]
mod tests {
    use crate::{common::MonoSample, primitives::{tests::TestAlwaysTooLoudDevice, clock::Clock}};

    use super::*;

    #[test]
    fn test_limiter_mainline() {
        const MAX: MonoSample = 0.9;
        let mut limiter = MiniLimiter::new(0.0, MAX);
        limiter.add_audio_source(Rc::new(RefCell::new(TestAlwaysTooLoudDevice::new())));
        assert_eq!(limiter.source_audio(&Clock::new()), MAX);
    }
}
