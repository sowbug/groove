use std::{cell::RefCell, rc::Rc};

use crate::{
    common::MonoSample,
    primitives::{clock::Clock, SinksAudio, SinksControl, SourcesAudio, WatchesClock, SinksControlParam},
};

#[derive(Default)]
pub struct Mixer {
    // TODO: somehow this isn't implemented in terms of primitives::mixer::Mixer
    sources: Vec<Rc<RefCell<dyn SourcesAudio>>>,
}

impl Mixer {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }
}

impl SourcesAudio for Mixer {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        self.gather_source_audio(clock)
    }
}

impl SinksAudio for Mixer {
    fn sources(&mut self) -> &mut Vec<Rc<RefCell<dyn SourcesAudio>>> {
        &mut self.sources
    }
}

impl SinksControl for Mixer {
    fn handle_control(&mut self, _clock: &Clock, _param: &SinksControlParam) {
        todo!()
    }
}
impl WatchesClock for Mixer {
    fn tick(&mut self, _clock: &Clock) -> bool {
        true
    }
}
