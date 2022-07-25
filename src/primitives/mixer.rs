#[derive(Debug, Default)]
pub struct MiniMixer {}

impl MiniMixer {
    pub fn new() -> Self {
        Self {}
    }

    // (sample value, gain)
    pub fn process(&self, samples: Vec<(f32, f32)>) -> f32 {
        if !samples.is_empty() {
            // https://stackoverflow.com/questions/41017140/why-cant-rust-infer-the-resulting-type-of-iteratorsum
            // this was from old code that used iter().sum()

            // Weighted sum
            // TODO: learn how to do this with custom implementation of std:iter:sum
            let mut sum = 0.0f32;
            let mut divisor = 0.0f32;
            for (v, g) in samples {
                sum += v * g;
                divisor += g;
            }
            sum / divisor as f32
        } else {
            0.
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::primitives::tests::{TestAlwaysLoudDevice, TestAlwaysSilentDevice};

    use super::*;

    #[test]
    fn test_mixer_mainline() {
        let mixer = MiniMixer::new();

        // Nothing/empty
        assert_eq!(mixer.process(Vec::new()), 0.);

        // One always-loud
        {
            let sources = vec![(TestAlwaysLoudDevice::new().get_audio_sample(), 42.0)];
            assert_eq!(mixer.process(sources), 1.);
        }
        // One always-loud and one always-quiet
        {
            let sources = vec![
                (TestAlwaysLoudDevice::new().get_audio_sample(), 0.7),
                (TestAlwaysSilentDevice::new().get_audio_sample(), 0.3),
            ];
            assert_eq!(mixer.process(sources), 0.7); // (0.7 * 1.0 + 0.3 * 0.0) / (0.7 + 0.3)
        }
    }
}
