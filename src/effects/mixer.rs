use std::{cell::RefCell, rc::Rc};

use crate::common::MonoSample;

use crate::primitives::clock::Clock;
use crate::traits::{
    IsEffect, SinksAudio, SinksControl, SinksControlParam, SourcesAudio, TransformsAudio,
};

#[derive(Debug, Default)]
pub struct Mixer {
    sources: Vec<Rc<RefCell<dyn SourcesAudio>>>,
}
impl IsEffect for Mixer {}
impl Mixer {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}
impl SinksAudio for Mixer {
    fn sources(&self) -> &[Rc<RefCell<dyn SourcesAudio>>] {
        &self.sources
    }
    fn sources_mut(&mut self) -> &mut Vec<Rc<RefCell<dyn SourcesAudio>>> {
        &mut self.sources
    }
}
impl TransformsAudio for Mixer {
    fn transform_audio(&mut self, input_sample: MonoSample) -> MonoSample {
        input_sample
    }
}
impl SinksControl for Mixer {
    fn handle_control(&mut self, _clock: &Clock, _param: &SinksControlParam) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use crate::{
        common::MONO_SAMPLE_SILENCE,
        primitives::clock::Clock,
        traits::tests::{TestAlwaysLoudDevice, TestAlwaysSameLevelDevice, TestAlwaysSilentDevice},
    };

    use super::*;

    #[test]
    fn test_mixer_mainline() {
        let clock = Clock::new();
        let mut mixer = Mixer::new();

        // Nothing/empty
        assert_eq!(mixer.source_audio(&clock), MONO_SAMPLE_SILENCE);

        // One always-loud
        mixer.add_audio_source(Rc::new(RefCell::new(TestAlwaysLoudDevice::new())));
        assert_eq!(mixer.source_audio(&clock), 1.0);

        // One always-loud and one always-quiet
        mixer.add_audio_source(Rc::new(RefCell::new(TestAlwaysSilentDevice::new())));
        assert_eq!(mixer.source_audio(&clock), 1.0 + 0.0);

        // ... and one in the middle
        mixer.add_audio_source(Rc::new(RefCell::new(TestAlwaysSameLevelDevice::new(0.25))));
        assert_eq!(mixer.source_audio(&clock), 1.0 + 0.0 + 0.25);
    }
}
