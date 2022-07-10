#[derive(Default)]
pub struct MiniGain {
    amount: f32,
}

impl MiniGain {
    pub fn new(amount: f32) -> Self {
        Self { amount }
    }

    pub fn process(&self, input: f32) -> f32 {
        self.amount * input
    }
}

#[cfg(test)]
mod tests {
    use crate::primitives::tests::TestAlwaysLoudDevice;

    use super::*;

    #[test]
    fn test_gain_mainline() {
        let loud = TestAlwaysLoudDevice::new();
        let gain = MiniGain::new(1.1);
        assert_eq!(gain.process(loud.get_audio_sample()), 1.1);
    }
}
