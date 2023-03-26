// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::{BipolarNormal, Normal};

#[derive(Debug, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn types_ok() {
        let a = usize::MAX;
        let f32cv: F32ControlValue = a.into();
        assert_eq!(a, <F32ControlValue as Into<usize>>::into(f32cv));

        let a = usize::MIN;
        let f32cv: F32ControlValue = a.into();
        assert_eq!(a, <F32ControlValue as Into<usize>>::into(f32cv));

        let a = u8::MAX;
        let f32cv: F32ControlValue = a.into();
        assert_eq!(a, <F32ControlValue as Into<u8>>::into(f32cv));

        let a = u8::MIN;
        let f32cv: F32ControlValue = a.into();
        assert_eq!(a, <F32ControlValue as Into<u8>>::into(f32cv));

        let a = f32::MAX;
        let f32cv: F32ControlValue = a.into();
        assert_eq!(a, <F32ControlValue as Into<f32>>::into(f32cv));

        let a = f32::MIN;
        let f32cv: F32ControlValue = a.into();
        assert_eq!(a, <F32ControlValue as Into<f32>>::into(f32cv));

        let a = 1000000.0f64;
        let f32cv: F32ControlValue = a.into();
        assert_eq!(a, <F32ControlValue as Into<f64>>::into(f32cv));

        let a = -1000000.0f64;
        let f32cv: F32ControlValue = a.into();
        assert_eq!(a, <F32ControlValue as Into<f64>>::into(f32cv));

        let a = Normal::maximum();
        let f32cv: F32ControlValue = a.into();
        assert_eq!(a, <F32ControlValue as Into<Normal>>::into(f32cv));

        let a = Normal::minimum();
        let f32cv: F32ControlValue = a.into();
        assert_eq!(a, <F32ControlValue as Into<Normal>>::into(f32cv));

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
}
