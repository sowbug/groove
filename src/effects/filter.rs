use std::{cell::RefCell, rc::Rc};

use crate::backend::devices::DeviceTrait;

pub struct AudioFilter {
    source: Rc<RefCell<dyn DeviceTrait>>,
    amount: f32,
}

impl DeviceTrait for AudioFilter {
    fn sources_audio(&self) -> bool {
        true
    }
    fn sinks_audio(&self) -> bool {
        true
    }

    fn add_audio_source(&mut self, source: std::rc::Rc<std::cell::RefCell<dyn DeviceTrait>>) {
        self.source = source;
    }

    fn tick(&mut self, _clock: &crate::backend::clock::Clock) -> bool {
        true
    }

    fn get_audio_sample(&self) -> f32 {
        self.source.borrow().get_audio_sample() * self.amount
    }
}
impl AudioFilter {
    pub(crate) fn new(source: Rc<RefCell<dyn DeviceTrait>>, amount: f32) -> AudioFilter {
        AudioFilter { source, amount }
    }
}
