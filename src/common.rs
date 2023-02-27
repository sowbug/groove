use std::{
    iter::Sum,
    ops::{Add, AddAssign, Div, Mul, Neg, Sub},
};

// TODO: these three should be #[cfg(test)] because nobody should be assuming
// these values
pub const DEFAULT_SAMPLE_RATE: usize = 44100;
pub const DEFAULT_BPM: f32 = 128.0;
pub const DEFAULT_TIME_SIGNATURE: (usize, usize) = (4, 4);

/// SampleType is the underlying primitive that makes up MonoSample and
/// StereoSample. It exists as a transition aid while we migrate from hardcoded
/// f32/OldMonoSample to MonoSample/StereoSample.
pub type SampleType = f64;

/// SignalType is the primitive used for general digital signal-related work.
/// It's pretty important that all of these different types be the same (e.g.,
/// for now f64), but I'm hoping it's worth the hassle to use different names
/// depending on usage.
pub type SignalType = f64;

/// Use ParameterType in places where a Normal or BipolarNormal could fit,
/// except you don't have any range restrictions.
#[allow(dead_code)]
pub type ParameterType = f64;

/// Sample is an audio sample.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Sample(pub SampleType);
impl Sample {
    #[allow(dead_code)]
    pub const SILENCE_VALUE: f64 = 0.0;
    #[allow(dead_code)]
    pub const SILENCE: Sample = Sample(Self::SILENCE_VALUE);
    #[allow(dead_code)]
    pub const MAX_VALUE: f64 = 1.0;
    #[allow(dead_code)]
    pub const MAX: Sample = Sample(Self::MAX_VALUE);
    #[allow(dead_code)]
    pub const MIN_VALUE: f64 = -1.0;
    #[allow(dead_code)]
    pub const MIN: Sample = Sample(Self::MIN_VALUE);

    // TODO: deprecate
    pub fn new_from_f32(value: f32) -> Self {
        Self(value as f64)
    }
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

/// RangedF64 tries to enforce the given range limits while not becoming too
/// expensive to use compared to a plain f64. It enforces the value at creation,
/// when setting it explicitly, when converting from an f64, and when getting
/// it. But math operations (Add, Sub, etc.) are not checked! This allows
/// certain operations to (hopefully temporarily) exceed the range, or for
/// floating-point precision problems to (again hopefully) get compensated for
/// later on.
///
/// Also note that RangedF64 doesn't tell you when clamping happens. It just
/// does it, silently.
///
/// Altogether, RangedF64 is good for gatekeeping -- parameters, return values,
/// etc., -- and somewhat OK at pure math. But we might decide to clamp (heh)
/// down on out-of-bounds conditions later on, so if you want to do math, prefer
/// f64 sourced from RangedF64 rather than RangedF64 itself.
///
/// TODO: I tried implementing this using the sort-of new generic const
/// expressions, because I wanted to see whether I could have compile-time
/// errors for attempts to set the value outside the range. I did not succeed.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct RangedF64<const LOWER: i8, const UPPER: i8>(f64);
impl<const LOWER: i8, const UPPER: i8> RangedF64<LOWER, UPPER> {
    pub const MAX: f64 = UPPER as f64;
    pub const MIN: f64 = LOWER as f64;

    pub fn new(value: f64) -> Self {
        Self(value.clamp(Self::MIN, Self::MAX))
    }
    pub fn new_from_f32(value: f32) -> Self {
        Self::new(value as f64)
    }
    pub fn maximum() -> Self {
        Self(Self::MAX)
    }
    pub fn minimum() -> Self {
        Self(Self::MIN)
    }
    pub fn value(&self) -> f64 {
        self.0.clamp(Self::MIN, Self::MAX)
    }
    pub fn set(&mut self, value: f64) {
        self.0 = value.clamp(Self::MIN, Self::MAX);
    }

    pub fn scale(&self, factor: f64) -> f64 {
        self.0 * factor
    }
}
impl<const LOWER: i8, const UPPER: i8> Add for RangedF64<LOWER, UPPER> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}
impl<const LOWER: i8, const UPPER: i8> Sub for RangedF64<LOWER, UPPER> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}
impl<const LOWER: i8, const UPPER: i8> Add<f64> for RangedF64<LOWER, UPPER> {
    type Output = Self;

    fn add(self, rhs: f64) -> Self::Output {
        Self(self.0 + rhs)
    }
}
impl<const LOWER: i8, const UPPER: i8> Sub<f64> for RangedF64<LOWER, UPPER> {
    type Output = Self;

    fn sub(self, rhs: f64) -> Self::Output {
        Self(self.0 - rhs)
    }
}
impl<const LOWER: i8, const UPPER: i8> From<RangedF64<LOWER, UPPER>> for f64 {
    fn from(value: RangedF64<LOWER, UPPER>) -> Self {
        value.0.clamp(Self::MIN, Self::MAX)
    }
}
impl<const LOWER: i8, const UPPER: i8> From<f64> for RangedF64<LOWER, UPPER> {
    fn from(value: f64) -> Self {
        Self(value.clamp(Self::MIN, Self::MAX))
    }
}
impl<const LOWER: i8, const UPPER: i8> From<f32> for RangedF64<LOWER, UPPER> {
    fn from(value: f32) -> Self {
        Self(value.clamp(Self::MIN as f32, Self::MAX as f32) as f64)
    }
}

pub type Normal = RangedF64<0, 1>;
pub type BipolarNormal = RangedF64<-1, 1>;

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct TimeUnit(pub f64);
impl TimeUnit {
    pub fn zero() -> TimeUnit {
        TimeUnit(0.0)
    }

    pub fn infinite() -> TimeUnit {
        TimeUnit(-1.0)
    }
}
impl From<f64> for TimeUnit {
    fn from(value: f64) -> Self {
        Self(value)
    }
}
impl From<f32> for TimeUnit {
    fn from(value: f32) -> Self {
        Self(value as f64)
    }
}
impl Add<f64> for TimeUnit {
    type Output = TimeUnit;

    fn add(self, rhs: f64) -> Self::Output {
        TimeUnit(self.0 + rhs)
    }
}
impl Add<TimeUnit> for TimeUnit {
    type Output = TimeUnit;

    fn add(self, rhs: TimeUnit) -> Self::Output {
        TimeUnit(self.0 + rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::StereoSample;
    use crate::common::Normal;

    impl StereoSample {
        // TODO: epsilon comparisons are bad. Figure out ULP (see float-cmp)
        pub(crate) fn almost_equals(&self, rhs: Self) -> bool {
            let epsilon = 0.0000001;
            (self.0 .0 - rhs.0 .0).abs() < epsilon && (self.1 .0 - rhs.1 .0).abs() < epsilon
        }
    }

    #[test]
    fn normal_mainline() {
        let a = Normal::new(0.2);
        let b = Normal::new(0.1);

        // Add(Normal)
        assert_eq!(a + b, Normal::new(0.2 + 0.1));

        // Sub(Normal)
        assert_eq!(a - b, Normal::new(0.1));

        // Add(f64)
        assert_eq!(a + 0.2f64, Normal::new(0.4));

        // Sub(f64)
        assert_eq!(a - 0.1, Normal::new(0.1));
    }
}
