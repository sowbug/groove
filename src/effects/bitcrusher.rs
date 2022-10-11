use std::{cell::RefCell, rc::Rc};

use crate::{
    common::{MonoSample, W, WW},
    traits::{IsEffect, SinksAudio, SourcesAudio, TransformsAudio},
};

#[derive(Debug, Default)]
pub struct Bitcrusher {
    pub(crate) me: WW<Self>,
    sources: Vec<Rc<RefCell<dyn SourcesAudio>>>,
    bits_to_crush: u8,
}
impl IsEffect for Bitcrusher {}
impl Bitcrusher {
    pub(crate) const CONTROL_PARAM_BITS_TO_CRUSH: &str = "bits-to-crush";

    pub fn new_with(bits_to_crush: u8) -> Self {
        Self {
            bits_to_crush,
            ..Default::default()
        }
    }
    pub fn new_wrapped_with(bits_to_crush: u8) -> W<Self> {
        // TODO: Rc::new_cyclic() should make this easier, but I couldn't get the syntax right.
        // https://doc.rust-lang.org/std/rc/struct.Rc.html#method.new_cyclic

        let wrapped = Rc::new(RefCell::new(Self::new_with(bits_to_crush)));
        wrapped.borrow_mut().me = Rc::downgrade(&wrapped);
        wrapped
    }

    #[allow(dead_code)]
    pub fn set_bits_to_crush(&mut self, n: u8) {
        self.bits_to_crush = n;
    }
}
impl SinksAudio for Bitcrusher {
    fn sources(&self) -> &[Rc<RefCell<dyn SourcesAudio>>] {
        &self.sources
    }
    fn sources_mut(&mut self) -> &mut Vec<Rc<RefCell<dyn SourcesAudio>>> {
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
        fx.add_audio_source(Rc::new(RefCell::new(TestAudioSourceAlwaysSameLevel::new(
            PI - 3.0,
        ))));
        fx.add_audio_source(Rc::new(RefCell::new(TestAudioSourceAlwaysSameLevel::new(
            PI - 3.0,
        ))));
        assert_eq!(fx.source_audio(&Clock::new()), 2.0 * CRUSHED_PI);
    }
}
