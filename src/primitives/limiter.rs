use crate::common::MonoSample;

#[derive(Default)]
pub struct MiniLimiter {
    min: MonoSample,
    max: MonoSample,
}
impl MiniLimiter {
    pub fn new(min: MonoSample, max: MonoSample) -> Self {
        Self { min, max }
    }
    pub fn process(&self, value: MonoSample) -> MonoSample {
        value.clamp(self.min, self.max)
    }
}

#[cfg(test)]
mod tests {
    use crate::{primitives::tests::TestAlwaysTooLoudDevice, common::MonoSample};

    use super::*;

    #[test]
    fn test_limiter_mainline() {
        const MAX: MonoSample = 0.9;
        let too_loud = TestAlwaysTooLoudDevice::new();
        let limiter = MiniLimiter::new(0.0, MAX);
        assert_eq!(limiter.process(too_loud.get_audio_sample()), MAX);
    }
}
