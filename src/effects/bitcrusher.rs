use crate::{
    clock::Clock,
    common::MonoSample,
    messages::GrooveMessage,
    traits::{HasUid, IsEffect, Updateable, TransformsAudio},
};

#[derive(Debug, Default)]
pub struct Bitcrusher {
    uid: usize,
    bits_to_crush: u8,
}
impl IsEffect for Bitcrusher {}
impl TransformsAudio for Bitcrusher {
    fn transform_audio(&mut self, _clock: &Clock, input_sample: MonoSample) -> MonoSample {
        let input_i16 = (input_sample * (i16::MAX as MonoSample)) as i16;
        let squished = input_i16 >> self.bits_to_crush;
        let expanded = squished << self.bits_to_crush;
        expanded as MonoSample / (i16::MAX as MonoSample)
    }
}
impl Updateable for Bitcrusher {
    type Message = GrooveMessage;

    fn update(
        &mut self,
        clock: &Clock,
        message: Self::Message,
    ) -> crate::traits::EvenNewerCommand<Self::Message> {
        //TODO        Self::Message::BitcrusherValueChanged(new_value)
        crate::traits::EvenNewerCommand::none()
    }
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

    pub(crate) fn new_with(bits_to_crush: u8) -> Self {
        Self {
            bits_to_crush,
            ..Default::default()
        }
    }

    pub(crate) fn bits_to_crush(&self) -> u8 {
        self.bits_to_crush
    }

    pub(crate) fn set_bits_to_crush(&mut self, n: u8) {
        self.bits_to_crush = n;
    }

    #[allow(dead_code)]
    pub(crate) fn set_bits_to_crush_pct(&mut self, pct: f32) {
        self.set_bits_to_crush((pct * 15.0) as u8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::Clock;
    use std::f32::consts::PI;

    const CRUSHED_PI: f32 = 0.14062929;

    #[test]
    fn test_bitcrusher_basic() {
        let mut fx = Bitcrusher::new_with(8);
        assert_eq!(fx.transform_audio(&Clock::default(), PI - 3.0), CRUSHED_PI);
    }
}
