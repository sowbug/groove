use std::{cell::RefCell, rc::Rc};

use crate::common::MonoSample;

use super::{IsEffect, SinksAudio, SourcesAudio, TransformsAudio};

#[derive(Default)]
pub struct Mixer {
    sources: Vec<Rc<RefCell<dyn SourcesAudio>>>,
}

impl Mixer {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

impl SinksAudio for Mixer {
    fn sources(&mut self) -> &mut Vec<Rc<RefCell<dyn SourcesAudio>>> {
        &mut self.sources
    }
}

impl TransformsAudio for Mixer {
    fn transform_audio(&mut self, input_sample: MonoSample) -> MonoSample {
        input_sample
    }
}

impl IsEffect for Mixer {}

#[cfg(test)]
mod tests {
    use std::{rc::Rc, cell::RefCell};

    use crate::{
        common::MONO_SAMPLE_SILENCE,
        primitives::tests::{
            TestAlwaysLoudDevice, TestAlwaysSameLevelDevice, TestAlwaysSilentDevice,
        },
    };

    use super::*;

    #[test]
    fn test_mixer_mainline() {
        const TIME_ON_CLOCK: f32 = 0.0;
        let mut mixer = Mixer::new();

        // Nothing/empty
        assert_eq!(mixer.source_audio(TIME_ON_CLOCK), MONO_SAMPLE_SILENCE);

        // One always-loud
        mixer.add_audio_source(Rc::new(RefCell::new(TestAlwaysLoudDevice::new())));
        assert_eq!(mixer.source_audio(TIME_ON_CLOCK), 1.0);

        // One always-loud and one always-quiet
        mixer.add_audio_source(Rc::new(RefCell::new(TestAlwaysSilentDevice::new())));
        assert_eq!(mixer.source_audio(TIME_ON_CLOCK), 1.0 + 0.0);

        // ... and one in the middle
        mixer.add_audio_source(Rc::new(RefCell::new(TestAlwaysSameLevelDevice::new(0.25))));
        assert_eq!(mixer.source_audio(TIME_ON_CLOCK), 1.0 + 0.0 + 0.25);
    }
}
