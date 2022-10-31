use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

type Refcounted<T> = Rc<T>;
pub(crate) type Rrc<T> = Refcounted<RefCell<T>>;
pub(crate) type Ww<T> = Weak<RefCell<T>>;
pub(crate) fn rrc<T>(t: T) -> Rrc<T> {
    Refcounted::new(RefCell::new(t))
}
pub(crate) fn rrc_downgrade<T: ?Sized>(t: &Rrc<T>) -> Ww<T> {
    Refcounted::downgrade(t)
}
pub(crate) fn rrc_clone<T: ?Sized>(t: &Rrc<T>) -> Rrc<T> {
    Refcounted::clone(t)
}
pub(crate) fn wrc_clone<T: ?Sized>(t: &Ww<T>) -> Ww<T> {
    Weak::clone(t)
}
pub(crate) fn weak_new<T>() -> Weak<T> {
    Weak::<T>::new()
}

pub type MonoSample = f32;
#[allow(dead_code)]
pub type StereoSample = (MonoSample, MonoSample);
pub const MONO_SAMPLE_SILENCE: MonoSample = 0.0;
pub const MONO_SAMPLE_MAX: MonoSample = 1.0;
pub const MONO_SAMPLE_MIN: MonoSample = -1.0;

pub type DeviceId = String;
