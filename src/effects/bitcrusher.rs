use crate::{
    clock::Clock,
    common::{rrc, rrc_downgrade, MonoSample, Rrc, Ww},
    messages::GrooveMessage,
    traits::{
        HasOverhead, HasUid, NewIsEffect, NewUpdateable, Overhead, SinksAudio, SourcesAudio,
        TransformsAudio,
    },
};

#[derive(Debug, Default)]
pub struct Bitcrusher {
    uid: usize,
    pub(crate) me: Ww<Self>,
    overhead: Overhead,

    sources: Vec<Ww<dyn SourcesAudio>>,
    bits_to_crush: u8,
}
impl NewIsEffect for Bitcrusher {}
impl TransformsAudio for Bitcrusher {
    fn transform_audio(&mut self, _clock: &Clock, input_sample: MonoSample) -> MonoSample {
        let input_i16 = (input_sample * (i16::MAX as MonoSample)) as i16;
        let squished = input_i16 >> self.bits_to_crush;
        let expanded = squished << self.bits_to_crush;
        expanded as MonoSample / (i16::MAX as MonoSample)
    }
}
impl NewUpdateable for Bitcrusher {
    type Message = GrooveMessage;
}
impl HasUid for Bitcrusher {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}

impl Bitcrusher {
    #[allow(dead_code)]
    fn new() -> Self {
        Self::new_with(8)
    }

    #[deprecated]
    pub fn new_wrapped() -> Rrc<Self> {
        Self::new_wrapped_with(8)
    }

    pub(crate) fn new_with(bits_to_crush: u8) -> Self {
        Self {
            bits_to_crush,
            ..Default::default()
        }
    }

    #[deprecated]
    pub fn new_wrapped_with(bits_to_crush: u8) -> Rrc<Self> {
        let wrapped = rrc(Self::new_with(bits_to_crush));
        wrapped.borrow_mut().me = rrc_downgrade(&wrapped);
        wrapped
    }

    pub(crate) fn bits_to_crush(&self) -> u8 {
        self.bits_to_crush
    }

    pub(crate) fn set_bits_to_crush(&mut self, n: u8) {
        self.bits_to_crush = n;
    }

    pub(crate) fn set_bits_to_crush_pct(&mut self, pct: f32) {
        self.set_bits_to_crush((pct * 15.0) as u8);
    }
}
impl SinksAudio for Bitcrusher {
    fn sources(&self) -> &[Ww<dyn SourcesAudio>] {
        &self.sources
    }
    fn sources_mut(&mut self) -> &mut Vec<Ww<dyn SourcesAudio>> {
        &mut self.sources
    }
}
impl HasOverhead for Bitcrusher {
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
        clock::Clock, messages::tests::TestMessage, utils::tests::TestAudioSourceOneLevel,
    };
    use std::f32::consts::PI;

    const CRUSHED_PI: f32 = 0.14062929;

    #[test]
    fn test_bitcrusher_basic() {
        let mut fx = Bitcrusher::new_with(8);
        assert_eq!(fx.transform_audio(&Clock::default(), PI - 3.0), CRUSHED_PI);
    }

    #[test]
    fn test_bitcrusher_multisource() {
        let mut fx = Bitcrusher::new_with(8);
        let source = rrc(TestAudioSourceOneLevel::new_with(PI - 3.0));
        fx.add_audio_source(rrc_downgrade::<TestAudioSourceOneLevel<TestMessage>>(
            &source,
        ));
        let source = rrc(TestAudioSourceOneLevel::new_with(PI - 3.0));
        fx.add_audio_source(rrc_downgrade::<TestAudioSourceOneLevel<TestMessage>>(
            &source,
        ));
        assert_eq!(fx.source_audio(&Clock::default()), 2.0 * CRUSHED_PI);
    }
}
