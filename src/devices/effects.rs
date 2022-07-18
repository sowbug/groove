use std::{cell::RefCell, rc::Rc};

use crate::primitives::{limiter::MiniLimiter, gain::MiniGain};

use super::traits::DeviceTrait;


pub struct Limiter {
  source: Rc<RefCell<dyn DeviceTrait>>,
  mini_limiter: MiniLimiter,
}
impl Limiter {
  pub fn new(source: Rc<RefCell<dyn DeviceTrait>>, min: f32, max: f32) -> Self {
      Self {
          source,
          mini_limiter: MiniLimiter::new(min, max),
      }
  }
}
impl DeviceTrait for Limiter {
  fn sources_audio(&self) -> bool {
      true
  }
  fn sinks_audio(&self) -> bool {
      true
  }
  fn add_audio_source(&mut self, source: Rc<RefCell<dyn DeviceTrait>>) {
      self.source = source;
  }
  fn get_audio_sample(&self) -> f32 {
      self.mini_limiter
          .process(self.source.borrow().get_audio_sample())
  }
}

pub struct Gain {
  source: Rc<RefCell<dyn DeviceTrait>>,
  mini_gain: MiniGain,
}
impl Gain {
  pub fn new(source: Rc<RefCell<dyn DeviceTrait>>, amount: f32) -> Self {
      Self {
          source,
          mini_gain: MiniGain::new(amount),
      }
  }
}
impl DeviceTrait for Gain {
  fn sources_audio(&self) -> bool {
      true
  }
  fn sinks_audio(&self) -> bool {
      true
  }
  fn add_audio_source(&mut self, source: Rc<RefCell<dyn DeviceTrait>>) {
      self.source = source;
  }
  fn get_audio_sample(&self) -> f32 {
      self.mini_gain
          .process(self.source.borrow().get_audio_sample())
  }
}
