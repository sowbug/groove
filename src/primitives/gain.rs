use crate::common::MonoSample;

use super::{SinksAudio, SourcesAudio, TransformsAudio};

#[derive(Default)]
pub struct MiniGain {
    sources: Vec<Box<dyn SourcesAudio>>,
    amount: f32,
}

impl MiniGain {
    pub fn new(amount: f32) -> Self {
        Self {
            amount,
            ..Default::default()
        }
    }
}

impl SourcesAudio for MiniGain {
    fn source_audio(&mut self, time_seconds: f32) -> MonoSample {
        let input = self.gather_source_audio(time_seconds);
        self.transform_audio(input)
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
    use crate::primitives::tests::TestAlwaysLoudDevice;

    use super::*;

    #[test]
    fn test_gain_mainline() {
        let mut gain = MiniGain::new(1.1);
        gain.add_audio_source(Box::new(TestAlwaysLoudDevice::new()));
        assert_eq!(gain.source_audio(0.0), 1.1);
    }
}
