use std::{cell::RefCell, rc::{Rc, Weak}};

pub type W<T> = Rc<RefCell<T>>;
pub type WW<T> = Weak<RefCell<T>>;

// TODO: some kind of wrap_me() parameterized function
//
// a HasMe trait with set_me() or something like that 
//
// pub fn wrap_me<T>(me: T) -> W<T> {
//     let wrapped = Rc::new(RefCell::new(me));
//     wrapped.borrow_mut().me = Rc::downgrade(&wrapped);
//     wrapped
// }

pub type MonoSample = f32;
#[allow(dead_code)]
pub type StereoSample = (MonoSample, MonoSample);
pub const MONO_SAMPLE_SILENCE: MonoSample = 0.0;

pub type DeviceId = String;

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum WaveformType {
    None,
    Sine,
    Square,
    PulseWidth(f32),
    Triangle,
    Sawtooth,
    Noise,
}

impl Default for WaveformType {
    fn default() -> Self {
        WaveformType::Sine
    }
}

#[cfg(test)]
pub mod tests {
    use super::MonoSample;

    pub const MONO_SAMPLE_MAX: MonoSample = 1.0;
    pub const MONO_SAMPLE_MIN: MonoSample = -1.0;
}
