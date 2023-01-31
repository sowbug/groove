use crate::{
    clock::Clock,
    common::F32ControlValue,
    common::MonoSample,
    messages::EntityMessage,
    traits::{Controllable, HasUid, IsEffect, TransformsAudio, Updateable},
};
use groove_macros::{Control, Uid};
use iced_audio::{IntRange, Normal};
use std::str::FromStr;
use strum_macros::{Display, EnumString, FromRepr};

#[derive(Control, Debug, Default, Uid)]
pub struct Bitcrusher {
    uid: usize,

    #[controllable]
    bits_to_crush: u8,

    bits_to_crush_int_range: IntRange,
}
impl IsEffect for Bitcrusher {}
impl TransformsAudio for Bitcrusher {
    fn transform_audio(&mut self, _clock: &Clock, input_sample: MonoSample) -> MonoSample {
        let sign = input_sample.signum();
        let input_i16: i16 = (input_sample.abs() * (i16::MAX as MonoSample)) as i16;
        let squished = input_i16 >> self.bits_to_crush;
        let expanded = squished << self.bits_to_crush;
        expanded as MonoSample / (i16::MAX as MonoSample) * sign
    }
}

impl Updateable for Bitcrusher {
    type Message = EntityMessage;
}

impl Bitcrusher {
    #[allow(dead_code)]
    fn new() -> Self {
        Self::new_with(8)
    }

    pub(crate) fn new_with(bits_to_crush: u8) -> Self {
        Self {
            bits_to_crush,
            bits_to_crush_int_range: IntRange::new(0, 15),
            ..Default::default()
        }
    }

    pub fn bits_to_crush(&self) -> u8 {
        self.bits_to_crush
    }

    pub fn set_bits_to_crush(&mut self, n: u8) {
        self.bits_to_crush = n;
    }

    pub(crate) fn set_control_bits_to_crush(&mut self, value: F32ControlValue) {
        self.set_bits_to_crush(
            self.bits_to_crush_int_range
                .unmap_to_value(Normal::from_clipped(value.0)) as u8,
        );
    }

    pub fn bits_to_crush_int_range(&self) -> IntRange {
        self.bits_to_crush_int_range
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::Clock;
    use std::f32::consts::PI;

    const CRUSHED_PI: f32 = 0.14062929;

    #[test]
    fn bitcrusher_basic() {
        let mut fx = Bitcrusher::new_with(8);
        assert_eq!(fx.transform_audio(&Clock::default(), PI - 3.0), CRUSHED_PI);
    }

    #[test]
    fn bitcrusher_no_bias() {
        let mut fx = Bitcrusher::new_with(8);
        assert_eq!(fx.transform_audio(&Clock::default(), PI - 3.0), CRUSHED_PI);
        assert_eq!(
            fx.transform_audio(&Clock::default(), -(PI - 3.0)),
            -CRUSHED_PI
        );
    }
}
