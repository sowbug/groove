use std::{rc::Rc, cell::RefCell};

use super::devices::DeviceTrait;

pub struct Mixer {
    sources: Vec<Rc<RefCell<dyn DeviceTrait>>>,
}

impl Mixer {
    pub fn new() -> Mixer {
        Mixer {
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
        let mut sample: f32 = 0.;
        for i in self.sources.clone() {
            let weight: f32 = 1. / self.sources.len() as f32;
            sample += i.borrow().get_audio_sample() * weight;
        }
        sample
    }
}

pub struct Quietener {
    source: Rc<RefCell<dyn DeviceTrait>>,
}
impl Quietener {
    pub fn new(source: Rc<RefCell<dyn DeviceTrait>>) -> Quietener {
        Quietener { source }
    }
}
// TODO(miket): idea: ticks are called only if the entity was asked for its sample, as a power optimization
impl DeviceTrait for Quietener {
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
        self.source.borrow().get_audio_sample() * 0.8
    }
}
