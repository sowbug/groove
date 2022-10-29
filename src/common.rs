use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

pub(crate) type Rrc<T> = Rc<RefCell<T>>;
pub(crate) type Ww<T> = Weak<RefCell<T>>;
pub(crate) fn rrc<T>(t: T) -> Rrc<T> {
    Rc::new(RefCell::new(t))
}
pub(crate) fn rrc_downgrade<T: ?Sized>(t: &Rc<RefCell<T>>) -> Weak<RefCell<T>> {
    Rc::downgrade(t)
}
pub(crate) fn wrc_clone<T: ?Sized>(t: &Weak<RefCell<T>>) -> Weak<RefCell<T>> {
    Weak::clone(t)
}

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
pub const MONO_SAMPLE_MAX: MonoSample = 1.0;
pub const MONO_SAMPLE_MIN: MonoSample = -1.0;

pub type DeviceId = String;
