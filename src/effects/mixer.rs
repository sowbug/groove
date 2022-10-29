use crate::{
    common::{MonoSample, Rrc, Ww, rrc, rrc_downgrade},
    traits::{IsEffect, IsMutable, SinksAudio, SourcesAudio, TransformsAudio},
};


#[derive(Clone, Debug, Default)]
pub struct Mixer {
    pub(crate) me: Ww<Self>,
    sources: Vec<Ww<dyn SourcesAudio>>,
    is_muted: bool,
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

        let wrapped = rrc(Self::new());
        wrapped.borrow_mut().me = rrc_downgrade(&wrapped);
        wrapped
    }

    pub fn mute_source(&mut self, index: usize, is_muted: bool) {
        if let Some(source) = self.sources[index].upgrade() {
            source.borrow_mut().set_muted(is_muted);
        }
    }
}
impl SinksAudio for Mixer {
    fn sources(&self) -> &[Ww<dyn SourcesAudio>] {
        &self.sources
    }
    fn sources_mut(&mut self) -> &mut Vec<Ww<dyn SourcesAudio>> {
        &mut self.sources
    }
}
impl TransformsAudio for Mixer {
    fn transform_audio(&mut self, input_sample: MonoSample) -> MonoSample {
        input_sample
    }
}
impl IsMutable for Mixer {
    fn is_muted(&self) -> bool {
        self.is_muted
    }

    fn set_muted(&mut self, is_muted: bool) {
        self.is_muted = is_muted;
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
    

    #[test]
    fn test_mixer_mainline() {
        let clock = Clock::new();
        let mut mixer = Mixer::new();

        // Nothing/empty
        assert_eq!(mixer.source_audio(&clock), MONO_SAMPLE_SILENCE);

        // One always-loud
        let source = rrc(TestAudioSourceAlwaysLoud::new());
        let source = rrc_downgrade(&source);
        mixer.add_audio_source(source);
        assert_eq!(mixer.source_audio(&clock), 1.0);

        // One always-loud and one always-quiet
        let source = rrc(TestAudioSourceAlwaysSilent::new());
        let source = rrc_downgrade(&source);
        mixer.add_audio_source(source);
        assert_eq!(mixer.source_audio(&clock), 1.0 + 0.0);

        // ... and one in the middle
        let source = rrc(TestAudioSourceAlwaysSameLevel::new(0.25));
        let source = rrc_downgrade(&source);
        mixer.add_audio_source(source);
        assert_eq!(mixer.source_audio(&clock), 1.0 + 0.0 + 0.25);
    }
}
