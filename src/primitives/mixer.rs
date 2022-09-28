use crate::common::MonoSample;

use super::{SinksAudio, SourcesAudio};

#[derive(Default)]
pub struct Mixer {
    sources: Vec<Box<dyn SourcesAudio>>,
}

impl Mixer {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

impl SourcesAudio for Mixer {
    fn source_audio(&mut self, time_seconds: f32) -> MonoSample {
        self.gather_source_audio(time_seconds)
    }
}

impl SinksAudio for Mixer {
    fn sources(&mut self) -> &mut Vec<Box<dyn SourcesAudio>> {
        &mut self.sources
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        common::MONO_SAMPLE_SILENCE,
        primitives::tests::{TestAlwaysLoudDevice, TestAlwaysSilentDevice},
    };

    use super::*;

    #[test]
    fn test_mixer_mainline() {
        const TIME_ON_CLOCK: f32 = 0.0;
        let mut mixer = Mixer::new();

        // Nothing/empty
        assert_eq!(mixer.source_audio(TIME_ON_CLOCK), MONO_SAMPLE_SILENCE);

        // One always-loud
        mixer.add_audio_source(Box::new(TestAlwaysLoudDevice::new()));
        assert_eq!(mixer.source_audio(TIME_ON_CLOCK), 1.);

        // One always-loud and one always-quiet
        mixer.add_audio_source(Box::new(TestAlwaysSilentDevice::new()));
        assert_eq!(mixer.source_audio(TIME_ON_CLOCK), 0.5);
    }
}
