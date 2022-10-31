use crate::{
    common::{rrc, rrc_downgrade, MonoSample, Rrc, Ww},
    traits::{IsEffect, IsMutable, SinksAudio, SourcesAudio, TransformsAudio},
};

#[derive(Debug, Default)]
pub struct Bitcrusher {
    pub(crate) me: Ww<Self>,
    sources: Vec<Ww<dyn SourcesAudio>>,
    bits_to_crush: u8,
    is_muted: bool,
}
impl IsEffect for Bitcrusher {}
impl Bitcrusher {
    pub(crate) const CONTROL_PARAM_BITS_TO_CRUSH: &str = "bits-to-crush";

    #[allow(dead_code)]
    fn new() -> Self {
        Self::new_with(8)
    }

    fn new_with(bits_to_crush: u8) -> Self {
        Self {
            bits_to_crush,
            ..Default::default()
        }
    }
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
}
impl SinksAudio for Bitcrusher {
    fn sources(&self) -> &[Ww<dyn SourcesAudio>] {
        &self.sources
    }
    fn sources_mut(&mut self) -> &mut Vec<Ww<dyn SourcesAudio>> {
        &mut self.sources
    }
}
impl TransformsAudio for Bitcrusher {
    fn transform_audio(&mut self, input_sample: MonoSample) -> MonoSample {
        let input_i16 = (input_sample * (i16::MAX as MonoSample)) as i16;
        let squished = input_i16 >> self.bits_to_crush;
        let expanded = squished << self.bits_to_crush;
        expanded as MonoSample / (i16::MAX as MonoSample)
    }
}

impl IsMutable for Bitcrusher {
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
    use crate::{clock::Clock, utils::tests::TestAudioSourceAlwaysSameLevel};
    use std::f32::consts::PI;

    const CRUSHED_PI: f32 = 0.14062929;

    #[test]
    fn test_bitcrusher_basic() {
        let mut fx = Bitcrusher::new_with(8);
        assert_eq!(fx.transform_audio(PI - 3.0), CRUSHED_PI);
    }

    #[test]
    fn test_bitcrusher_multisource() {
        let mut fx = Bitcrusher::new_with(8);
        let source = rrc(TestAudioSourceAlwaysSameLevel::new(PI - 3.0));
        let source = rrc_downgrade(&source);
        fx.add_audio_source(source);
        let source = rrc(TestAudioSourceAlwaysSameLevel::new(PI - 3.0));
        let source = rrc_downgrade(&source);
        fx.add_audio_source(source);
        assert_eq!(fx.source_audio(&Clock::new()), 2.0 * CRUSHED_PI);
    }
}
