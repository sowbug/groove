use std::ops::{Add, Sub};

pub type MonoSample = f32;
// impl Default for MonoSample {
//     fn default() -> Self {
//         MONO_SAMPLE_SILENCE
//     }
// }
#[allow(dead_code)]
pub type StereoSample = (MonoSample, MonoSample);
pub const MONO_SAMPLE_SILENCE: MonoSample = 0.0;
pub const MONO_SAMPLE_MAX: MonoSample = 1.0;
pub const MONO_SAMPLE_MIN: MonoSample = -1.0;

pub type DeviceId = String;

pub struct F32ControlValue(pub f32);

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Unipolar(pub f64);
impl Unipolar {
    pub fn maximum() -> Unipolar {
        Unipolar(1.0)
    }
    pub fn minimum() -> Unipolar {
        Unipolar(0.0)
    }
    pub fn value(&self) -> f64 {
        self.0
    }
}
impl Add<Unipolar> for Unipolar {
    type Output = Unipolar;

    fn add(self, rhs: Unipolar) -> Self::Output {
        Unipolar(self.0 + rhs.0)
    }
}
impl Sub<Unipolar> for Unipolar {
    type Output = Unipolar;

    fn sub(self, rhs: Unipolar) -> Self::Output {
        Unipolar(self.0 - rhs.0)
    }
}
impl Add<f64> for Unipolar {
    type Output = Unipolar;

    fn add(self, rhs: f64) -> Self::Output {
        Unipolar(self.0 + rhs)
    }
}
impl Sub<f64> for Unipolar {
    type Output = Unipolar;

    fn sub(self, rhs: f64) -> Self::Output {
        Unipolar(self.0 - rhs)
    }
}
impl From<Unipolar> for f64 {
    fn from(value: Unipolar) -> Self {
        value.0
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
