use std::ops::{Add, AddAssign, Mul, Neg, Sub};

/// SampleType is the underlying primitive that makes up MonoSample and
/// StereoSample. It exists as a transition aid while we migrate from hardcoded
/// f32/OldMonoSample to MonoSample/StereoSample.
pub type SampleType = f64;

/// Sample is an audio sample.
#[derive(Debug, Default, PartialEq, PartialOrd)]
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
}
impl AddAssign for Sample {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}
impl Add<Sample> for Sample {
    type Output = Self;

    fn add(self, rhs: Sample) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}
impl Mul<Sample> for Sample {
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

/// MonoSample is a single-channel sample. It exists separately from Sample for
/// cases where we specifically want a monophonic audio stream.
///
/// TODO: I'm not convinced this is useful.
#[derive(Debug, Default, PartialEq, PartialOrd)]
pub struct MonoSample(pub SampleType);

// TODO: enable this to feel bad, then fix it #[deprecated]
pub type OldMonoSample = f32;
pub const MONO_SAMPLE_SILENCE: OldMonoSample = 0.0;
pub const MONO_SAMPLE_MAX: OldMonoSample = 1.0;
pub const MONO_SAMPLE_MIN: OldMonoSample = -1.0;

/// StereoSample is a two-channel sample. Each channel is a MonoSample.
#[derive(Debug, Default, PartialEq, PartialOrd)]
pub struct StereoSample(pub SampleType, pub SampleType);
impl StereoSample {
    pub const SILENCE: StereoSample = StereoSample(0.0, 0.0);
    pub const MAX: StereoSample = StereoSample(1.0, 1.0);
    pub const MIN: StereoSample = StereoSample(-1.0, -1.0);
}

pub type DeviceId = String;

pub struct F32ControlValue(pub f32);

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

pub type Unipolar = RangedF64<0, 1>;
pub type Bipolar = RangedF64<-1, 1>;

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
        Self { 0: value }
    }
}
impl From<f32> for TimeUnit {
    fn from(value: f32) -> Self {
        Self { 0: value as f64 }
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
    use crate::common::Unipolar;

    #[test]
    fn unipolar_mainline() {
        let a = Unipolar::new(0.2);
        let b = Unipolar::new(0.1);

        // Add(Unipolar)
        assert_eq!(a + b, Unipolar::new(0.2 + 0.1));

        // Sub(Unipolar)
        assert_eq!(a - b, Unipolar::new(0.1));

        // Add(f64)
        assert_eq!(a + 0.2f64, Unipolar::new(0.4));

        // Sub(f64)
        assert_eq!(a - 0.1, Unipolar::new(0.1));
    }
}
