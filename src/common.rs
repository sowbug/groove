use groove_core::Sample;
use std::ops::{Add, Sub};

// TODO: these three should be #[cfg(test)] because nobody should be assuming
// these values
pub const DEFAULT_SAMPLE_RATE: usize = 44100;
pub const DEFAULT_BPM: f32 = 128.0;
pub const DEFAULT_TIME_SIGNATURE: (usize, usize) = (4, 4);

/// SignalType is the primitive used for general digital signal-related work.
/// It's pretty important that all of these different types be the same (e.g.,
/// for now f64), but I'm hoping it's worth the hassle to use different names
/// depending on usage.
pub type SignalType = f64;

/// Use ParameterType in places where a Normal or BipolarNormal could fit,
/// except you don't have any range restrictions.
#[allow(dead_code)]
pub type ParameterType = f64;

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

impl From<Sample> for BipolarNormal {
    // A Sample has the same range as a BipolarNormal, so no conversion is
    // necessary.
    fn from(value: Sample) -> Self {
        Self(value.0)
    }
}

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
    use crate::common::Normal;

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
