// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::{BipolarNormal, Normal};
use ensnare::prelude::*;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[derive(Debug)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct ControlName(pub String);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct ControlIndex(pub usize);

/// A [ControlValue] is a standardized value range (0..=1.0) for
/// Controls/Controllable.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct ControlValue(pub f64);
impl ControlValue {
    pub const MIN: Self = Self(0.0);
    pub const MAX: Self = Self(1.0);
}
impl From<Normal> for ControlValue {
    fn from(value: Normal) -> Self {
        Self(value.0)
    }
}
impl From<ControlValue> for Normal {
    fn from(value: ControlValue) -> Self {
        Self::from(value.0)
    }
}
impl From<BipolarNormal> for ControlValue {
    fn from(value: BipolarNormal) -> Self {
        Self(Normal::from(value).into())
    }
}
impl From<ControlValue> for BipolarNormal {
    fn from(value: ControlValue) -> Self {
        Self::from(Normal::from(value))
    }
}
impl From<usize> for ControlValue {
    fn from(value: usize) -> Self {
        Self(value as f64)
    }
}
impl From<ControlValue> for usize {
    fn from(value: ControlValue) -> Self {
        value.0 as usize
    }
}
impl From<u8> for ControlValue {
    fn from(value: u8) -> Self {
        Self(value as f64 / u8::MAX as f64)
    }
}
impl From<ControlValue> for u8 {
    fn from(value: ControlValue) -> Self {
        (value.0 * u8::MAX as f64) as u8
    }
}
impl From<f32> for ControlValue {
    fn from(value: f32) -> Self {
        Self(value as f64)
    }
}
impl From<ControlValue> for f32 {
    fn from(value: ControlValue) -> Self {
        value.0 as f32
    }
}
impl From<f64> for ControlValue {
    fn from(value: f64) -> Self {
        Self(value)
    }
}
impl From<ControlValue> for f64 {
    fn from(value: ControlValue) -> Self {
        value.0 as f64
    }
}
impl From<FrequencyHz> for ControlValue {
    fn from(value: FrequencyHz) -> Self {
        FrequencyHz::frequency_to_percent(value.0).into()
    }
}
impl From<ControlValue> for FrequencyHz {
    fn from(value: ControlValue) -> Self {
        Self::percent_to_frequency(Normal::from(value)).into()
    }
}
impl From<bool> for ControlValue {
    fn from(value: bool) -> Self {
        ControlValue(if value { 1.0 } else { 0.0 })
    }
}
impl From<ControlValue> for bool {
    fn from(value: ControlValue) -> Self {
        value.0 != 0.0
    }
}
impl From<Ratio> for ControlValue {
    fn from(value: Ratio) -> Self {
        ControlValue(Normal::from(value).0)
    }
}
impl From<ControlValue> for Ratio {
    fn from(value: ControlValue) -> Self {
        Self::from(Normal::from(value))
    }
}
impl From<Tempo> for ControlValue {
    fn from(value: Tempo) -> Self {
        Self(value.0 / Tempo::MAX_VALUE)
    }
}
impl From<ControlValue> for Tempo {
    fn from(value: ControlValue) -> Self {
        Self(value.0 * Tempo::MAX_VALUE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usize_ok() {
        let a = usize::MAX;
        let cv: ControlValue = a.into();
        assert_eq!(a, <ControlValue as Into<usize>>::into(cv));

        let a = usize::MIN;
        let cv: ControlValue = a.into();
        assert_eq!(a, <ControlValue as Into<usize>>::into(cv));
    }

    #[test]
    fn u8_ok() {
        let a = u8::MAX;
        let cv: ControlValue = a.into();
        assert_eq!(a, <ControlValue as Into<u8>>::into(cv));

        let a = u8::MIN;
        let cv: ControlValue = a.into();
        assert_eq!(a, <ControlValue as Into<u8>>::into(cv));
    }

    #[test]
    fn f32_ok() {
        let a = f32::MAX;
        let cv: ControlValue = a.into();
        assert_eq!(a, <ControlValue as Into<f32>>::into(cv));

        let a = f32::MIN;
        let cv: ControlValue = a.into();
        assert_eq!(a, <ControlValue as Into<f32>>::into(cv));
    }

    #[test]
    fn f64_ok() {
        let a = 1000000.0f64;
        let cv: ControlValue = a.into();
        assert_eq!(a, <ControlValue as Into<f64>>::into(cv));

        let a = -1000000.0f64;
        let cv: ControlValue = a.into();
        assert_eq!(a, <ControlValue as Into<f64>>::into(cv));
    }

    #[test]
    fn normal_ok() {
        let a = Normal::maximum();
        let cv: ControlValue = a.into();
        assert_eq!(a, <ControlValue as Into<Normal>>::into(cv));

        let a = Normal::minimum();
        let cv: ControlValue = a.into();
        assert_eq!(a, <ControlValue as Into<Normal>>::into(cv));
    }

    #[test]
    fn bipolar_normal_ok() {
        let a = BipolarNormal::maximum();
        let cv: ControlValue = a.into();
        assert_eq!(a, <ControlValue as Into<BipolarNormal>>::into(cv));

        let a = BipolarNormal::minimum();
        let cv: ControlValue = a.into();
        assert_eq!(a, <ControlValue as Into<BipolarNormal>>::into(cv));

        let a = BipolarNormal::zero();
        let cv: ControlValue = a.into();
        assert_eq!(a, <ControlValue as Into<BipolarNormal>>::into(cv));
    }

    #[test]
    fn bool_ok() {
        let a = true;
        let cv: ControlValue = a.into();
        assert_eq!(a, <ControlValue as Into<bool>>::into(cv));

        let a = false;
        let cv: ControlValue = a.into();
        assert_eq!(a, <ControlValue as Into<bool>>::into(cv));
    }

    #[test]
    fn ratio_ok() {
        assert_eq!(Ratio::from(ControlValue(0.0)).value(), 0.125);
        assert_eq!(Ratio::from(ControlValue(0.5)).value(), 1.0);
        assert_eq!(Ratio::from(ControlValue(1.0)).value(), 8.0);

        assert_eq!(ControlValue::from(Ratio::from(0.125)).0, 0.0);
        assert_eq!(ControlValue::from(Ratio::from(1.0)).0, 0.5);
        assert_eq!(ControlValue::from(Ratio::from(8.0)).0, 1.0);

        assert_eq!(Ratio::from(BipolarNormal::from(-1.0)).value(), 0.125);
        assert_eq!(Ratio::from(BipolarNormal::from(0.0)).value(), 1.0);
        assert_eq!(Ratio::from(BipolarNormal::from(1.0)).value(), 8.0);

        assert_eq!(BipolarNormal::from(Ratio::from(0.125)).value(), -1.0);
        assert_eq!(BipolarNormal::from(Ratio::from(1.0)).value(), 0.0);
        assert_eq!(BipolarNormal::from(Ratio::from(8.0)).value(), 1.0);
    }
}
