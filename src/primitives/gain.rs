use crate::common::MonoSample;

use super::{SinksAudio, SourcesAudio, TransformsAudio};

#[derive(Default)]
pub struct MiniGain {
    sources: Vec<Box<dyn SourcesAudio>>,
    amount: f32,
}

impl MiniGain {
    pub fn new() -> Self {
        Self {
            amount: 1.0,
            ..Default::default()
        }
    }

    pub fn new_with(amount: f32) -> Self {
        Self {
            amount,
            ..Default::default()
        }
    }
}

impl SinksAudio for MiniGain {
    fn sources(&mut self) -> &mut Vec<Box<dyn SourcesAudio>> {
        &mut self.sources
    }
}

impl TransformsAudio for MiniGain {
    fn transform_audio(&mut self, input_sample: MonoSample) -> MonoSample {
        input_sample * self.amount
    }
}

#[cfg(test)]
mod tests {
    use crate::primitives::tests::{TestAlwaysLoudDevice, TestAlwaysSameLevelDevice};

    use super::*;

    #[test]
    fn test_gain_mainline() {
        let mut gain = MiniGain::new_with(1.1);
        gain.add_audio_source(Box::new(TestAlwaysLoudDevice::new()));
        assert_eq!(gain.source_audio(0.0), 1.1);
    }

    #[test]
    fn test_gain_pola() { // principle of least astonishment: does a default instance adhere?
        let mut gain = MiniGain::new();
        gain.add_audio_source(Box::new(TestAlwaysSameLevelDevice::new(0.888)));
        assert_eq!(gain.source_audio(0.0), 0.888);
    }
}
