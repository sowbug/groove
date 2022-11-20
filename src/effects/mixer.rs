use crate::{
    clock::Clock,
    common::{rrc, rrc_downgrade, MonoSample, Rrc, Ww},
    traits::{
        HasOverhead, HasUid, IsEffect, MessageBounds, NewIsEffect, NewUpdateable, Overhead, SinksAudio,
        SourcesAudio, TransformsAudio,
    },
};

#[derive(Clone, Debug, Default)]
pub struct Mixer<M: MessageBounds> {
    uid: usize,
    pub(crate) me: Ww<Self>,
    overhead: Overhead,

    sources: Vec<Ww<dyn SourcesAudio>>,
}
impl<M: MessageBounds> IsEffect for Mixer<M> {}
impl<M: MessageBounds> NewIsEffect for Mixer<M> {}
impl<M: MessageBounds> TransformsAudio for Mixer<M> {
    fn transform_audio(&mut self, _clock: &Clock, input_sample: MonoSample) -> MonoSample {
        input_sample
    }
}
impl<M: MessageBounds> NewUpdateable for Mixer<M> {
    type Message = M;
}
impl<M: MessageBounds> HasUid for Mixer<M> {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}
impl<M: MessageBounds> Mixer<M> {
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
impl<M: MessageBounds> SinksAudio for Mixer<M> {
    fn sources(&self) -> &[Ww<dyn SourcesAudio>] {
        &self.sources
    }
    fn sources_mut(&mut self) -> &mut Vec<Ww<dyn SourcesAudio>> {
        &mut self.sources
    }
}
impl<M: MessageBounds> HasOverhead for Mixer<M> {
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
        messages::tests::TestMessage,
        utils::tests::{
            TestAudioSourceAlwaysLoud, TestAudioSourceAlwaysSilent, TestAudioSourceOneLevel,
        },
    };

    #[test]
    fn test_mixer_mainline() {
        let clock = Clock::new();
        let mut mixer = Mixer::<TestMessage>::new();

        // Nothing/empty
        assert_eq!(mixer.source_audio(&clock), MONO_SAMPLE_SILENCE);

        // One always-loud
        let source = rrc(TestAudioSourceAlwaysLoud::new());
        mixer.add_audio_source(rrc_downgrade::<TestAudioSourceAlwaysLoud<TestMessage>>(
            &source,
        ));
        assert_eq!(mixer.source_audio(&clock), 1.0);

        // One always-loud and one always-quiet
        let source = rrc(TestAudioSourceAlwaysSilent::new());
        mixer.add_audio_source(rrc_downgrade::<TestAudioSourceAlwaysSilent<TestMessage>>(
            &source,
        ));
        assert_eq!(mixer.source_audio(&clock), 1.0 + 0.0);

        // ... and one in the middle
        let source = rrc(TestAudioSourceOneLevel::new_with(0.25));
        mixer.add_audio_source(rrc_downgrade::<TestAudioSourceOneLevel<TestMessage>>(
            &source,
        ));
        assert_eq!(mixer.source_audio(&clock), 1.0 + 0.0 + 0.25);
    }
}
