use std::{cell::RefCell, rc::Rc};

use crate::{
    common::{MonoSample, MONO_SAMPLE_SILENCE},
    primitives::clock::Clock,
};

use super::traits::{AudioSink, AudioSource, AutomationSink, TimeSlicer};

#[derive(Default)]
pub struct Mixer {
    // TODO: somehow this isn't implemented in terms of primitives::mixer::Mixer
    sources: Vec<Rc<RefCell<dyn AudioSource>>>,
}

impl Mixer {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }
}

impl AudioSource for Mixer {
    fn sample(&mut self) -> MonoSample {
        if self.audio_sources().is_empty() {
            MONO_SAMPLE_SILENCE
        } else {
            self.audio_sources()
                .iter_mut()
                .map(|source| source.borrow_mut().sample())
                .sum::<f32>()
        }
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
