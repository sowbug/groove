use std::{cell::RefCell, rc::Rc};

use crate::{common::MonoSample, primitives::mixer::MiniMixer};

use super::traits::{AudioSink, AudioSource, AutomationSink, TimeSlicer};

#[derive(Default)]
pub struct Mixer {
    mini_mixer: MiniMixer,
    sources: Vec<(Rc<RefCell<dyn AudioSource>>, f32)>,
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
        for (source, relative_gain) in self.sources.clone() {
            samples.push((source.borrow_mut().sample(), relative_gain));
        }
        self.mini_mixer.process(samples)
    }
}

impl AudioSink for Mixer {
    fn add_source(&mut self, audio_instrument: Rc<RefCell<dyn AudioSource>>) {
        self.sources.push((audio_instrument, 1.0));
    }
}

impl AutomationSink for Mixer {}
impl TimeSlicer for Mixer {}
