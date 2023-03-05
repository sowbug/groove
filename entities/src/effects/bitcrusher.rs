use groove_core::{
    control::F32ControlValue,
    traits::{Controllable, HasUid, IsEffect, TransformsAudio},
    Sample, SampleType,
};
use groove_macros::{Control, Uid};
use std::str::FromStr;
use strum_macros::{Display, EnumString, FromRepr};

/// TODO: this is a pretty lame bitcrusher. It is hardly noticeable for values
/// below 13, and it destroys the waveform at 15. It doesn't do any simulation
/// of sample-rate reduction, either.
#[derive(Control, Debug, Uid)]
pub struct Bitcrusher {
    uid: usize,

    #[controllable]
    bits_to_crush: u8,

    c: SampleType,
}
impl IsEffect for Bitcrusher {}
impl TransformsAudio for Bitcrusher {
    fn transform_channel(&mut self, _channel: usize, input_sample: Sample) -> Sample {
        const I16_SCALE: SampleType = i16::MAX as SampleType;
        let sign = input_sample.0.signum();
        let input = (input_sample * I16_SCALE).0.abs();
        (((input / self.c).floor() * self.c / I16_SCALE) * sign).into()
    }
}
impl Default for Bitcrusher {
    fn default() -> Self {
        Self::new_with(8)
    }
}
impl Bitcrusher {
    pub fn new_with(bits_to_crush: u8) -> Self {
        let mut r = Self {
            uid: Default::default(),
            bits_to_crush,
            c: Default::default(),
        };
        r.update_c();
        r
    }

    pub fn bits_to_crush(&self) -> u8 {
        self.bits_to_crush
    }

    pub fn set_bits_to_crush(&mut self, n: u8) {
        self.bits_to_crush = n;
        self.update_c();
    }

    fn update_c(&mut self) {
        self.c = 2.0f64.powi(self.bits_to_crush as i32);
    }

    pub fn set_control_bits_to_crush(&mut self, value: F32ControlValue) {
        self.set_bits_to_crush((value.0 * 16.0).floor() as u8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use groove_core::{Sample, SampleType};
    use std::f64::consts::PI;

    const CRUSHED_PI: SampleType = 0.14062929166539506;

    #[test]
    fn bitcrusher_basic() {
        let mut fx = Bitcrusher::new_with(8);
        assert_eq!(
            fx.transform_channel(0, Sample(PI - 3.0)),
            Sample(CRUSHED_PI)
        );
    }

    #[test]
    fn bitcrusher_no_bias() {
        let mut fx = Bitcrusher::new_with(8);
        assert_eq!(
            fx.transform_channel(0, Sample(-(PI - 3.0))),
            Sample(-CRUSHED_PI)
        );
    }
}
