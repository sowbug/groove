use std::{cell::RefCell, rc::Rc};

use crate::primitives::mixer::MiniMixer;

use super::traits::DeviceTrait;

#[derive(Default)]
pub struct Mixer {
    mini_mixer: MiniMixer,
    sources: Vec<(Rc<RefCell<dyn DeviceTrait>>, f32)>,
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
        self.sources.push((audio_instrument, 1.0));
    }
    fn get_audio_sample(&mut self) -> f32 {
        let mut samples = Vec::new();
        for (source, relative_gain) in self.sources.clone() {
            samples.push((source.borrow_mut().get_audio_sample(), relative_gain));
        }
        self.mini_mixer.process(samples)
    }
}
