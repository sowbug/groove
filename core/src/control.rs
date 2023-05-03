// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::{BipolarNormal, FrequencyHz, Normal};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct F32ControlValue(pub f32);

impl From<F32ControlValue> for u8 {
    fn from(value: F32ControlValue) -> Self {
        (value.0 * u8::MAX as f32) as u8
    }
}
impl Into<F32ControlValue> for u8 {
    fn into(self) -> F32ControlValue {
        F32ControlValue(self as f32 / u8::MAX as f32)
    }
}
impl From<F32ControlValue> for usize {
    fn from(value: F32ControlValue) -> Self {
        value.0 as usize
    }
}
impl Into<F32ControlValue> for usize {
    fn into(self) -> F32ControlValue {
        F32ControlValue(self as f32)
    }
}
impl From<F32ControlValue> for f32 {
    fn from(value: F32ControlValue) -> Self {
        value.0
    }
}
impl Into<F32ControlValue> for f32 {
    fn into(self) -> F32ControlValue {
        F32ControlValue(self)
    }
}
impl From<F32ControlValue> for f64 {
    fn from(value: F32ControlValue) -> Self {
        value.0 as f64
    }
}
impl Into<F32ControlValue> for f64 {
    fn into(self) -> F32ControlValue {
        F32ControlValue(self as f32)
    }
}
impl From<F32ControlValue> for Normal {
    fn from(value: F32ControlValue) -> Self {
        Self::from(value.0)
    }
}
impl Into<F32ControlValue> for Normal {
    fn into(self) -> F32ControlValue {
        F32ControlValue(self.value_as_f32())
    }
}
impl From<F32ControlValue> for BipolarNormal {
    fn from(value: F32ControlValue) -> Self {
        Self::from(Normal::from(value))
    }
}
impl Into<F32ControlValue> for BipolarNormal {
    fn into(self) -> F32ControlValue {
        let n: Normal = self.into();
        n.into()
    }
}
impl From<F32ControlValue> for FrequencyHz {
    fn from(value: F32ControlValue) -> Self {
        Self::percent_to_frequency(Normal::from(value)).into()
    }
}
impl Into<F32ControlValue> for FrequencyHz {
    fn into(self) -> F32ControlValue {
        Self::frequency_to_percent(self.0).into()
    }
}
impl From<F32ControlValue> for bool {
    fn from(value: F32ControlValue) -> Self {
        value.0 != 0.0
    }
}
impl Into<F32ControlValue> for bool {
    fn into(self) -> F32ControlValue {
        F32ControlValue(if self { 1.0 } else { 0.0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Ratio;

    #[test]
    fn usize_ok() {
        let a = usize::MAX;
        let f32cv: F32ControlValue = a.into();
        assert_eq!(a, <F32ControlValue as Into<usize>>::into(f32cv));

        let a = usize::MIN;
        let f32cv: F32ControlValue = a.into();
        assert_eq!(a, <F32ControlValue as Into<usize>>::into(f32cv));
    }

    #[test]
    fn u8_ok() {
        let a = u8::MAX;
        let f32cv: F32ControlValue = a.into();
        assert_eq!(a, <F32ControlValue as Into<u8>>::into(f32cv));

        let a = u8::MIN;
        let f32cv: F32ControlValue = a.into();
        assert_eq!(a, <F32ControlValue as Into<u8>>::into(f32cv));
    }

    #[test]
    fn f32_ok() {
        let a = f32::MAX;
        let f32cv: F32ControlValue = a.into();
        assert_eq!(a, <F32ControlValue as Into<f32>>::into(f32cv));

        let a = f32::MIN;
        let f32cv: F32ControlValue = a.into();
        assert_eq!(a, <F32ControlValue as Into<f32>>::into(f32cv));
    }

    #[test]
    fn f64_ok() {
        let a = 1000000.0f64;
        let f32cv: F32ControlValue = a.into();
        assert_eq!(a, <F32ControlValue as Into<f64>>::into(f32cv));

        let a = -1000000.0f64;
        let f32cv: F32ControlValue = a.into();
        assert_eq!(a, <F32ControlValue as Into<f64>>::into(f32cv));
    }

    #[test]
    fn normal_ok() {
        let a = Normal::maximum();
        let f32cv: F32ControlValue = a.into();
        assert_eq!(a, <F32ControlValue as Into<Normal>>::into(f32cv));

        let a = Normal::minimum();
        let f32cv: F32ControlValue = a.into();
        assert_eq!(a, <F32ControlValue as Into<Normal>>::into(f32cv));
    }

    #[test]
    fn bipolar_normal_ok() {
        let a = BipolarNormal::maximum();
        let f32cv: F32ControlValue = a.into();
        assert_eq!(a, <F32ControlValue as Into<BipolarNormal>>::into(f32cv));

        let a = BipolarNormal::minimum();
        let f32cv: F32ControlValue = a.into();
        assert_eq!(a, <F32ControlValue as Into<BipolarNormal>>::into(f32cv));

        let a = BipolarNormal::zero();
        let f32cv: F32ControlValue = a.into();
        assert_eq!(a, <F32ControlValue as Into<BipolarNormal>>::into(f32cv));
    }

    #[test]
    fn bool_ok() {
        let a = true;
        let f32cv: F32ControlValue = a.into();
        assert_eq!(a, <F32ControlValue as Into<bool>>::into(f32cv));

        let a = false;
        let f32cv: F32ControlValue = a.into();
        assert_eq!(a, <F32ControlValue as Into<bool>>::into(f32cv));
    }

    #[test]
    fn ratio_ok() {
        assert_eq!(Ratio::from(F32ControlValue(0.0)).value(), 0.125);
        assert_eq!(Ratio::from(F32ControlValue(0.5)).value(), 1.0);
        assert_eq!(Ratio::from(F32ControlValue(1.0)).value(), 8.0);

        assert_eq!(F32ControlValue::from(Ratio::from(0.125)).0, 0.0);
        assert_eq!(F32ControlValue::from(Ratio::from(1.0)).0, 0.5);
        assert_eq!(F32ControlValue::from(Ratio::from(8.0)).0, 1.0);
    }
}
