use std::ops::{Add, Sub};

pub type MonoSample = f32;
pub const MONO_SAMPLE_SILENCE: MonoSample = 0.0;
pub const MONO_SAMPLE_MAX: MonoSample = 1.0;
pub const MONO_SAMPLE_MIN: MonoSample = -1.0;
// impl Default for MonoSample {
//     fn default() -> Self {
//         MONO_SAMPLE_SILENCE
//     }
// }

#[derive(Debug, Default, PartialEq, PartialOrd)]
pub struct StereoSample(pub f64, pub f64);
impl StereoSample {
    pub const SILENCE: StereoSample = StereoSample(0.0, 0.0);
    pub const MAX: StereoSample = StereoSample(1.0, 1.0);
    pub const MIN: StereoSample = StereoSample(-1.0, -1.0);
}

pub type DeviceId = String;

pub struct F32ControlValue(pub f32);

// TODO: how expensive are all these clamp()s?
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Unipolar(pub f64);
impl Unipolar {
    const MAX: f64 = 1.0;
    const MIN: f64 = 0.0;

    pub fn maximum() -> Self {
        Self(1.0)
    }
    pub fn minimum() -> Self {
        Self(0.0)
    }
    pub fn value(&self) -> f64 {
        self.0
    }
}
impl Add<Unipolar> for Unipolar {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}
impl Sub<Unipolar> for Unipolar {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}
impl Add<f64> for Unipolar {
    type Output = Self;

    fn add(self, rhs: f64) -> Self::Output {
        Self(self.0 + rhs)
    }
}
impl Sub<f64> for Unipolar {
    type Output = Self;

    fn sub(self, rhs: f64) -> Self::Output {
        Self(self.0 - rhs)
    }
}
impl From<Unipolar> for f64 {
    fn from(value: Unipolar) -> Self {
        value.0.clamp(Unipolar::MIN, Unipolar::MAX)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Bipolar(f64);
impl Bipolar {
    const MAX: f64 = 1.0;
    const MIN: f64 = -1.0;

    pub fn new(value: f64) -> Self {
        if value < Self::MIN || value > Self::MAX {
            panic!("Attempt to create Bipolar outside valid range");
        }
        Self(value)
    }
    pub fn maximum() -> Self {
        Self(Self::MAX)
    }
    pub fn minimum() -> Self {
        Self(Self::MIN)
    }
    pub fn value(&self) -> f64 {
        self.0
    }
    pub fn safe_value(&self) -> f64 {
        self.0.clamp(Self::MIN, Self::MAX)
    }
}
impl Add<Bipolar> for Bipolar {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self((self.0 + rhs.0).clamp(Self::MIN, Self::MAX))
    }
}
impl Sub<Bipolar> for Bipolar {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self((self.0 - rhs.0).clamp(Self::MIN, Self::MAX))
    }
}
impl Add<f64> for Bipolar {
    type Output = Self;

    fn add(self, rhs: f64) -> Self::Output {
        Self((self.0 + rhs).clamp(Self::MIN, Self::MAX))
    }
}
impl Sub<f64> for Bipolar {
    type Output = Self;

    fn sub(self, rhs: f64) -> Self::Output {
        Self((self.0 - rhs).clamp(Self::MIN, Self::MAX))
    }
}
impl From<Bipolar> for f64 {
    fn from(value: Bipolar) -> Self {
        value.0.clamp(Bipolar::MIN, Bipolar::MAX)
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
