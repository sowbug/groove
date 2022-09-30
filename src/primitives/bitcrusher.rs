use std::{cell::RefCell, rc::Rc};

use crate::common::MonoSample;

use super::{
    clock::Clock, IsEffect, SinksAudio, SinksControl, SinksControlParam, SourcesAudio,
    TransformsAudio,
};

#[derive(Default)]
pub struct Bitcrusher {
    sources: Vec<Rc<RefCell<dyn SourcesAudio>>>,
    bits_to_crush: u8,
}
impl IsEffect for Bitcrusher {}
impl Bitcrusher {
    pub fn new_with(bits_to_crush: u8) -> Self {
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
    fn sources(&mut self) -> &mut Vec<Rc<RefCell<dyn SourcesAudio>>> {
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
impl SinksControl for Bitcrusher {
    fn handle_control(&mut self, _clock: &Clock, param: &SinksControlParam) {
        match param {
            SinksControlParam::Primary { value } => {
                self.set_bits_to_crush(*value as u8);
            }
            _ => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::primitives::{clock::Clock, tests::TestAlwaysSameLevelDevice};
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
        fx.add_audio_source(Rc::new(RefCell::new(TestAlwaysSameLevelDevice::new(
            PI - 3.0,
        ))));
        fx.add_audio_source(Rc::new(RefCell::new(TestAlwaysSameLevelDevice::new(
            PI - 3.0,
        ))));
        assert_eq!(fx.source_audio(&Clock::new()), 2.0 * CRUSHED_PI);
    }
}
