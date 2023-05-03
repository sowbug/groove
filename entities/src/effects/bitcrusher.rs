// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    traits::{IsEffect, Resets, TransformsAudio},
    Sample, SampleType,
};
use groove_proc_macros::{Nano, Uid};
use std::str::FromStr;
use strum::EnumCount;
use strum_macros::{Display, EnumCount as EnumCountMacro, EnumString, FromRepr, IntoStaticStr};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

/// TODO: this is a pretty lame bitcrusher. It is hardly noticeable for values
/// below 13, and it destroys the waveform at 15. It doesn't do any simulation
/// of sample-rate reduction, either.
#[derive(Debug, Nano, Uid)]
pub struct Bitcrusher {
    uid: usize,

    #[nano]
    bits: u8,

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
impl Resets for Bitcrusher {}
impl Bitcrusher {
    pub fn new_with(params: BitcrusherNano) -> Self {
        let mut r = Self {
            uid: Default::default(),
            bits: params.bits(),
            c: Default::default(),
        };
        r.update_c();
        r
    }

    pub fn bits(&self) -> u8 {
        self.bits
    }

    pub fn set_bits(&mut self, n: u8) {
        self.bits = n;
        self.update_c();
    }

    fn update_c(&mut self) {
        self.c = 2.0f64.powi(self.bits() as i32);
    }

    // TODO - write a custom type for range 0..16

    pub fn update(&mut self, message: BitcrusherMessage) {
        match message {
            BitcrusherMessage::Bitcrusher(s) => *self = Self::new_with(s),
            _ => self.derived_update(message),
        }
    }

    fn bits_range() -> std::ops::RangeInclusive<u8> {
        0..=16
    }
}

#[cfg(feature = "egui-framework")]
mod gui {
    use super::Bitcrusher;
    use eframe::egui::{DragValue, Ui};
    use groove_core::traits::gui::Shows;

    impl Shows for Bitcrusher {
        fn show(&mut self, ui: &mut Ui) {
            let mut bits = self.bits();
            if ui
                .add(
                    DragValue::new(&mut bits)
                        .clamp_range(Bitcrusher::bits_range())
                        .suffix(" bits"),
                )
                .changed()
            {
                self.set_bits(bits);
            };
        }
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
        let mut fx = Bitcrusher::new_with(BitcrusherNano { bits: 8 });
        assert_eq!(
            fx.transform_channel(0, Sample(PI - 3.0)),
            Sample(CRUSHED_PI)
        );
    }

    #[test]
    fn bitcrusher_no_bias() {
        let mut fx = Bitcrusher::new_with(BitcrusherNano { bits: 8 });
        assert_eq!(
            fx.transform_channel(0, Sample(-(PI - 3.0))),
            Sample(-CRUSHED_PI)
        );
    }
}
