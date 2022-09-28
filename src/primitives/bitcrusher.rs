use crate::common::MonoSample;

use super::{SinksAudio, SourcesAudio, TransformsAudio};

#[derive(Default)]
pub struct Bitcrusher {
    sources: Vec<Box<dyn SourcesAudio>>,
    bits_to_crush: u8,
}

impl Bitcrusher {
    pub fn new(bits_to_crush: u8) -> Self {
        Self {
            bits_to_crush,
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    pub fn set_bits_to_crush(&mut self, n: u8) {
        self.bits_to_crush = n;
    }
}

impl SinksAudio for Bitcrusher {
    fn sources(&mut self) -> &mut Vec<Box<dyn SourcesAudio>> {
        &mut self.sources
    }
}

impl SourcesAudio for Bitcrusher {
    fn source_audio(&mut self, time_seconds: f32) -> MonoSample {
        let input = self.gather_source_audio(time_seconds);
        self.transform_audio(input)
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

#[cfg(test)]
mod tests {

    use super::*;
    use crate::primitives::tests::TestAlwaysSameLevelDevice;
    use std::f32::consts::PI;

    #[test]
    fn test_bitcrusher_basic() {
        let mut fx = Bitcrusher::new(8);
        let source = Box::new(TestAlwaysSameLevelDevice::new(PI - 3.0));
        fx.add_audio_source(source);
        assert_eq!(fx.source_audio(0.0), 0.14062929);
    }
}
