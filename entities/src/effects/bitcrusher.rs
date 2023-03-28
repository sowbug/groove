// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    traits::{IsEffect, TransformsAudio},
    Sample, SampleType,
};
use groove_macros::{Control, Synchronization, Uid};
use std::str::FromStr;

use strum::EnumCount;
use strum_macros::{
    Display, EnumCount as EnumCountMacro, EnumIter, EnumString, FromRepr, IntoStaticStr,
};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Synchronization)]
#[cfg_attr(
    feature = "serialization",
    derive(Serialize, Deserialize),
    serde(rename = "bitcrusher", rename_all = "kebab-case")
)]
pub struct BitcrusherParams {
    #[sync]
    pub bits: u8,
}

impl BitcrusherParams {
    pub fn bits(&self) -> u8 {
        self.bits
    }

    pub fn set_bits(&mut self, bits: u8) {
        self.bits = bits;
    }
}

/// TODO: this is a pretty lame bitcrusher. It is hardly noticeable for values
/// below 13, and it destroys the waveform at 15. It doesn't do any simulation
/// of sample-rate reduction, either.
#[derive(Control, Debug, Uid)]
pub struct Bitcrusher {
    uid: usize,

    params: BitcrusherParams,

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
impl Bitcrusher {
    pub fn new_with_params(params: BitcrusherParams) -> Self {
        let mut r = Self {
            uid: Default::default(),
            params,
            bits_to_crush: params.bits(),
            c: Default::default(),
        };
        r.update_c();
        r
    }

    pub fn bits_to_crush(&self) -> u8 {
        self.params.bits()
    }

    pub fn set_bits_to_crush(&mut self, n: u8) {
        self.params.set_bits(n);
        self.update_c();
    }

    fn update_c(&mut self) {
        self.c = 2.0f64.powi(self.params.bits() as i32);
    }

    pub fn set_control_bits_to_crush(&mut self, value: groove_core::control::F32ControlValue) {
        self.set_bits_to_crush((value.0 * 16.0).floor() as u8);
    }

    pub fn params(&self) -> BitcrusherParams {
        self.params
    }

    pub fn update(&mut self, message: BitcrusherParamsMessage) {
        self.params.update(message)
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
        let mut fx = Bitcrusher::new_with_params(BitcrusherParams { bits: 8 });
        assert_eq!(
            fx.transform_channel(0, Sample(PI - 3.0)),
            Sample(CRUSHED_PI)
        );
    }

    #[test]
    fn bitcrusher_no_bias() {
        let mut fx = Bitcrusher::new_with_params(BitcrusherParams { bits: 8 });
        assert_eq!(
            fx.transform_channel(0, Sample(-(PI - 3.0))),
            Sample(-CRUSHED_PI)
        );
    }
}
