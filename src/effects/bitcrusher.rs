use crate::{
    clock::Clock,
    common::MonoSample,
    messages::EntityMessage,
    traits::{HasUid, IsEffect, Response, TransformsAudio, Updateable},
};
use iced_audio::{IntRange, Normal};
use std::str::FromStr;
use strum_macros::{Display, EnumString, FromRepr};

#[derive(Display, Debug, EnumString, FromRepr)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum BitcrusherControlParams {
    #[strum(serialize = "bits-to-crush", serialize = "bits-to-crush-pct")]
    BitsToCrushPct,
}

#[derive(Debug, Default)]
pub struct Bitcrusher {
    uid: usize,
    bits_to_crush: u8,

    int_range: IntRange,
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
    type Message = EntityMessage;

    #[allow(unused_variables)]
    fn update(&mut self, clock: &Clock, message: Self::Message) -> Response<Self::Message> {
        match message {
            Self::Message::UpdateF32(param_id, value) => {
                self.set_indexed_param_f32(param_id, value);
            }
            EntityMessage::HSliderInt(value) => {
                self.set_bits_to_crush(self.int_range.unmap_to_value(value) as u8);
            }
            _ => todo!(),
        }
        Response::none()
    }

    fn param_id_for_name(&self, name: &str) -> usize {
        if let Ok(param) = BitcrusherControlParams::from_str(name) {
            param as usize
        } else {
            usize::MAX
        }
    }

    fn set_indexed_param_f32(&mut self, index: usize, value: f32) {
        if let Some(param) = BitcrusherControlParams::from_repr(index) {
            match param {
                BitcrusherControlParams::BitsToCrushPct => {
                    self.set_bits_to_crush(
                        self.int_range.unmap_to_value(Normal::from_clipped(value)) as u8,
                    );
                }
            }
        } else {
            todo!()
        }
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
            int_range: IntRange::new(0, 15),
            ..Default::default()
        }
    }

    pub fn bits_to_crush(&self) -> u8 {
        self.bits_to_crush
    }

    pub(crate) fn set_bits_to_crush(&mut self, n: u8) {
        self.bits_to_crush = n;
    }

    // TODO: this is a temporary and nonsensical name
    pub fn int_range(&self) -> IntRange {
        self.int_range
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
