#[derive(Debug, Default)]
pub struct MiniMixer {}

impl MiniMixer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn process(&self, samples: Vec<f32>) -> f32 {
        if samples.len() > 0 {
            // https://stackoverflow.com/questions/41017140/why-cant-rust-infer-the-resulting-type-of-iteratorsum
            let sum: f32 = samples.iter().sum();
            sum / samples.len() as f32
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
            let sources = vec![TestAlwaysLoudDevice::new().get_audio_sample()];
            assert_eq!(mixer.process(sources), 1.);
        }
        // One always-loud and one always-quiet
        {
            let sources = vec![
                TestAlwaysLoudDevice::new().get_audio_sample(),
                TestAlwaysSilentDevice::new().get_audio_sample(),
            ];
            assert_eq!(mixer.process(sources), 0.5);
        }
    }
}
