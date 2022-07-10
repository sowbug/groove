use std::{rc::Rc, cell::RefCell};

use crate::primitives::mixer::MiniMixer;

use super::traits::DeviceTrait;

pub struct Mixer {
  mini_mixer: MiniMixer,
  sources: Vec<Rc<RefCell<dyn DeviceTrait>>>,
}

impl Mixer {
  pub fn new() -> Self {
      Self {
          mini_mixer: MiniMixer::new(),
          sources: Vec::new(),
      }
  }
}
impl DeviceTrait for Mixer {
  fn sources_audio(&self) -> bool {
      true
  }
  fn sinks_audio(&self) -> bool {
      true
  }
  fn add_audio_source(&mut self, audio_instrument: Rc<RefCell<dyn DeviceTrait>>) {
      self.sources.push(audio_instrument);
  }
  fn get_audio_sample(&self) -> f32 {
      let mut samples = Vec::new();
      for source in self.sources.iter() {
          samples.push(source.borrow().get_audio_sample());
      }
      self.mini_mixer.process(samples)
  }
}
