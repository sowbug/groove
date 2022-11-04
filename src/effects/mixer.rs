use crate::{
    common::{rrc, rrc_downgrade, MonoSample, Rrc, Ww},
    traits::{HasOverhead, IsEffect, Overhead, SinksAudio, SourcesAudio, TransformsAudio},
};

#[derive(Debug, Default)]
pub struct Mixer {
    pub(crate) me: Ww<Self>,
    overhead: Overhead,

    sources: Vec<Ww<dyn SourcesAudio>>,
}
impl IsEffect for Mixer {}
impl Mixer {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
    pub fn new_wrapped() -> Rrc<Self> {
        let wrapped = rrc(Self::new());
        wrapped.borrow_mut().me = rrc_downgrade(&wrapped);
        wrapped
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
impl HasOverhead for Mixer {
    fn overhead(&self) -> &Overhead {
        &self.overhead
    }

    fn overhead_mut(&mut self) -> &mut Overhead {
        &mut self.overhead
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
