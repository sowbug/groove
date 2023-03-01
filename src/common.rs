use std::ops::Add;

// TODO: these three should be #[cfg(test)] because nobody should be assuming
// these values
pub const DEFAULT_SAMPLE_RATE: usize = 44100;
pub const DEFAULT_BPM: f32 = 128.0;
pub const DEFAULT_TIME_SIGNATURE: (usize, usize) = (4, 4);

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
mod tests {}
