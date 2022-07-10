#[derive(Default)]
pub struct MiniLimiter {
    min: f32,
    max: f32,
}
impl MiniLimiter {
    pub fn new(min: f32, max: f32) -> Self {
        Self { min, max }
    }
    pub fn process(&self, value: f32) -> f32 {
        value.clamp(self.min, self.max)
    }
}

#[cfg(test)]
mod tests {
    use crate::primitives::tests::TestAlwaysTooLoudDevice;

    use super::*;

    #[test]
    fn test_limiter_mainline() {
        const MAX: f32 = 0.9;
        let too_loud = TestAlwaysTooLoudDevice::new();
        let limiter = MiniLimiter::new(0.0, MAX);
        assert_eq!(limiter.process(too_loud.get_audio_sample()), MAX);
    }
}
