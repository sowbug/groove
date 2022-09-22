use std::{cell::RefCell, rc::Rc};

use crate::{
    common::MonoSample,
    primitives::{clock::Clock, mixer::MiniMixer},
};

use super::traits::{AudioSink, AudioSource, AutomationSink, TimeSlicer};

#[derive(Default)]
pub struct Mixer {
    mini_mixer: MiniMixer,
    // TODO: how do we get this back in again?
    //    sources: Vec<(Rc<RefCell<dyn AudioSource>>, f32)>,
    sources: Vec<Rc<RefCell<dyn AudioSource>>>,
}

impl Mixer {
    pub fn new() -> Self {
        Self {
            mini_mixer: MiniMixer::new(),
            sources: Vec::new(),
        }
    }
}

impl AudioSource for Mixer {
    fn sample(&mut self) -> MonoSample {
        let mut samples = Vec::new();
        // for (source, relative_gain) in self.sources.clone() {
        //     samples.push((source.borrow_mut().sample(), relative_gain));
        // }
        for source in self.sources.clone() {
            samples.push((source.borrow_mut().sample(), 1.0));
        }
        self.mini_mixer.process(samples)
    }
}

impl AudioSink for Mixer {
    fn audio_sources(&mut self) -> &mut Vec<Rc<RefCell<dyn AudioSource>>> {
        &mut self.sources
    }
}

impl AutomationSink for Mixer {
    fn handle_automation_message(&mut self, _message: &super::traits::AutomationMessage) {
        todo!()
    }
}
impl TimeSlicer for Mixer {
    fn tick(&mut self, _clock: &Clock) -> bool {
        true
    }
}
