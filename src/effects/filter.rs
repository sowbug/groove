use std::{cell::RefCell, rc::Rc};

use crate::backend::devices::DeviceTrait;

pub struct AudioFilter {
    last_sample: f32,
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
        // TODO: either we collect this here (and risk someone getting asked for audio before tick())
        // or we change get_audio_sample() to require a mutable self.
        self.last_sample = self.source.borrow().get_audio_sample();
        true
    }

    fn get_audio_sample(&self) -> f32 {
        self.last_sample + self.source.borrow().get_audio_sample() //* self.amount
    }
}
impl AudioFilter {
    pub(crate) fn new(source: Rc<RefCell<dyn DeviceTrait>>, amount: f32) -> AudioFilter {
        AudioFilter {
            source,
            amount,
            last_sample: 0.,
        }
    }
}
