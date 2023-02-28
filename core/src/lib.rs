// Copyright (c) 2023 Mike Tsao. All rights reserved.

use std::{
    iter::Sum,
    ops::{Add, AddAssign, Div, Mul, Neg, Sub},
};

/// SampleType is the underlying primitive that makes up MonoSample and
/// StereoSample. It exists as a transition aid while we migrate from hardcoded
/// f32/OldMonoSample to MonoSample/StereoSample.
pub type SampleType = f64;

/// Sample is an audio sample.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Sample(pub SampleType);
impl Sample {
    pub const SILENCE_VALUE: SampleType = 0.0;
    pub const SILENCE: Sample = Sample(Self::SILENCE_VALUE);
    pub const MAX_VALUE: SampleType = 1.0;
    pub const MAX: Sample = Sample(Self::MAX_VALUE);
    pub const MIN_VALUE: SampleType = -1.0;
    pub const MIN: Sample = Sample(Self::MIN_VALUE);
}
impl AddAssign for Sample {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}
impl Add for Sample {
    type Output = Self;

    fn add(self, rhs: Sample) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}
impl Mul for Sample {
    type Output = Self;

    fn mul(self, rhs: Sample) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}
impl Mul<f64> for Sample {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self(self.0 * rhs)
    }
}
// TODO #[deprecated] because it hides evidence that migration to SampleType
// isn't complete
impl Mul<f32> for Sample {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self(self.0 * rhs as f64)
    }
}
impl Div<f64> for Sample {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self(self.0 / rhs)
    }
}
impl Sub for Sample {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}
impl Neg for Sample {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}
impl Mul<i16> for Sample {
    type Output = Self;

    fn mul(self, rhs: i16) -> Self::Output {
        Self(self.0 * rhs as f64)
    }
}
impl From<f64> for Sample {
    fn from(value: f64) -> Self {
        Sample(value)
    }
}
impl From<f32> for Sample {
    fn from(value: f32) -> Self {
        Sample(value as f64)
    }
}
impl From<i32> for Sample {
    // TODO: this is an incomplete conversion, because we don't know what the
    // range of the i32 really is. So we leave it to someone else to divide by
    // the correct value to obtain the proper -1.0..=1.0 range.
    fn from(value: i32) -> Self {
        Sample(value as f64)
    }
}

/// MonoSample is a single-channel sample. It exists separately from Sample for
/// cases where we specifically want a monophonic audio stream.
///
/// TODO: I'm not convinced this is useful.
#[derive(Debug, Default, PartialEq, PartialOrd)]
pub struct MonoSample(pub SampleType);

/// StereoSample is a two-channel sample.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct StereoSample(pub Sample, pub Sample);
impl StereoSample {
    pub const SILENCE: StereoSample = StereoSample(Sample::SILENCE, Sample::SILENCE);
    pub const MAX: StereoSample = StereoSample(Sample::MAX, Sample::MAX);
    pub const MIN: StereoSample = StereoSample(Sample::MIN, Sample::MIN);

    pub fn new_from_f64(left: SampleType, right: SampleType) -> Self {
        Self(Sample(left), Sample(right))
    }

    pub fn new_from_single_f64(value: SampleType) -> Self {
        Self::new_from_f64(value, value)
    }

    // This method should be used only for testing. TODO: get rid of this. Now
    // that we're in a separate crate, we can't easily limit this to test cfg
    // only. That means it's part of the API.
    //
    // TODO: epsilon comparisons are bad. Recommend float-cmp crate instead of
    // this.
    pub fn almost_equals(&self, rhs: Self) -> bool {
        let epsilon = 0.0000001;
        (self.0 .0 - rhs.0 .0).abs() < epsilon && (self.1 .0 - rhs.1 .0).abs() < epsilon
    }
}
impl Add for StereoSample {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        StereoSample(self.0 + rhs.0, self.1 + rhs.1)
    }
}
impl AddAssign for StereoSample {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}
impl Sum for StereoSample {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self(Sample::SILENCE, Sample::SILENCE), |a, b| {
            Self(a.0 + b.0, a.1 + b.1)
        })
    }
}
impl From<Sample> for StereoSample {
    fn from(value: Sample) -> Self {
        Self(value, value)
    }
}
impl From<f64> for StereoSample {
    fn from(value: f64) -> Self {
        Self(Sample(value), Sample(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions() {
        let sample = Sample::MAX;
        let stereo_sample = StereoSample::MAX;

        let converted = StereoSample::from(sample);
        assert_eq!(stereo_sample, converted);
    }
}
