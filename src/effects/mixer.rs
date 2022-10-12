use crate::{
    common::{MonoSample, Rrc, Ww},
    traits::{IsEffect, SinksAudio, SourcesAudio, TransformsAudio},
};
use std::{cell::RefCell, rc::Rc};

#[derive(Debug, Default)]
pub struct Mixer {
    pub(crate) me: Ww<Self>,
    sources: Vec<Rrc<dyn SourcesAudio>>,
}
impl IsEffect for Mixer {}
impl Mixer {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
    pub fn new_wrapped() -> Rrc<Self> {
        // TODO: Rc::new_cyclic() should make this easier, but I couldn't get the syntax right.
        // https://doc.rust-lang.org/std/rc/struct.Rc.html#method.new_cyclic

        let wrapped = Rc::new(RefCell::new(Self::new()));
        wrapped.borrow_mut().me = Rc::downgrade(&wrapped);
        wrapped
    }
}
impl SinksAudio for Mixer {
    fn sources(&self) -> &[Rrc<dyn SourcesAudio>] {
        &self.sources
    }
    fn sources_mut(&mut self) -> &mut Vec<Rrc<dyn SourcesAudio>> {
        &mut self.sources
    }
}
impl TransformsAudio for Mixer {
    fn transform_audio(&mut self, input_sample: MonoSample) -> MonoSample {
        input_sample
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        clock::Clock,
        common::MONO_SAMPLE_SILENCE,
        utils::tests::{
            TestAudioSourceAlwaysLoud, TestAudioSourceAlwaysSameLevel, TestAudioSourceAlwaysSilent,
        },
    };
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn test_mixer_mainline() {
        let clock = Clock::new();
        let mut mixer = Mixer::new();

        // Nothing/empty
        assert_eq!(mixer.source_audio(&clock), MONO_SAMPLE_SILENCE);

        // One always-loud
        mixer.add_audio_source(Rc::new(RefCell::new(TestAudioSourceAlwaysLoud::new())));
        assert_eq!(mixer.source_audio(&clock), 1.0);

        // One always-loud and one always-quiet
        mixer.add_audio_source(Rc::new(RefCell::new(TestAudioSourceAlwaysSilent::new())));
        assert_eq!(mixer.source_audio(&clock), 1.0 + 0.0);

        // ... and one in the middle
        mixer.add_audio_source(Rc::new(RefCell::new(TestAudioSourceAlwaysSameLevel::new(
            0.25,
        ))));
        assert_eq!(mixer.source_audio(&clock), 1.0 + 0.0 + 0.25);
    }
}
