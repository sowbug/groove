use crate::{
    clock::Clock,
    common::{rrc, rrc_downgrade, MonoSample, Rrc, Ww},
    traits::{HasOverhead, IsEffect, Overhead, SinksAudio, SourcesAudio, TransformsAudio},
};

#[derive(Debug, Default)]
pub(crate) struct Gain {
    pub(crate) me: Ww<Self>,
    overhead: Overhead,

    sources: Vec<Ww<dyn SourcesAudio>>,
    ceiling: f32,
}
impl IsEffect for Gain {}

impl Gain {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            ceiling: 1.0,
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    pub fn new_wrapped() -> Rrc<Self> {
        let wrapped = rrc(Self::new());
        wrapped.borrow_mut().me = rrc_downgrade(&wrapped);
        wrapped
    }

    pub fn new_with(ceiling: f32) -> Self {
        Self {
            ceiling,
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    pub fn new_wrapped_with(ceiling: f32) -> Rrc<Self> {
        let wrapped = rrc(Self::new_with(ceiling));
        wrapped.borrow_mut().me = rrc_downgrade(&wrapped);
        wrapped
    }

    #[allow(dead_code)]
    pub fn ceiling(&self) -> f32 {
        self.ceiling
    }

    #[allow(dead_code)]
    pub fn set_ceiling(&mut self, pct: f32) {
        self.ceiling = pct;
    }
}
impl SinksAudio for Gain {
    fn sources(&self) -> &[Ww<dyn SourcesAudio>] {
        &self.sources
    }
    fn sources_mut(&mut self) -> &mut Vec<Ww<dyn SourcesAudio>> {
        &mut self.sources
    }
}
impl TransformsAudio for Gain {
    fn transform_audio(&mut self, _clock: &Clock, input_sample: MonoSample) -> MonoSample {
        input_sample * self.ceiling
    }
}
impl HasOverhead for Gain {
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
        messages::tests::TestMessage,
        utils::tests::{TestAudioSourceAlwaysLoud, TestAudioSourceAlwaysSameLevel},
    };

    #[test]
    fn test_gain_mainline() {
        let mut gain = Gain::new_with(1.1);
        let source = rrc(TestAudioSourceAlwaysLoud::new());
        gain.add_audio_source(rrc_downgrade::<TestAudioSourceAlwaysLoud<TestMessage>>(
            &source,
        ));
        assert_eq!(gain.source_audio(&Clock::new()), 1.1);
    }

    #[test]
    fn test_gain_pola() {
        // principle of least astonishment: does a default instance adhere?
        let mut gain = Gain::new();
        let source = rrc(TestAudioSourceAlwaysSameLevel::new_with(0.888));
        gain.add_audio_source(
            rrc_downgrade::<TestAudioSourceAlwaysSameLevel<TestMessage>>(&source),
        );
        assert_eq!(gain.source_audio(&Clock::new()), 0.888);
    }
}
