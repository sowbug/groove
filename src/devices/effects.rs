use std::{cell::RefCell, rc::Rc};

use crate::primitives::{self, gain::MiniGain, limiter::MiniLimiter};

use super::traits::DeviceTrait;
pub struct Limiter {
    source: Option<Rc<RefCell<dyn DeviceTrait>>>,
    effect: MiniLimiter,
}

impl Limiter {
    pub fn new(source: Option<Rc<RefCell<dyn DeviceTrait>>>, min: f32, max: f32) -> Self {
        Self {
            source,
            effect: MiniLimiter::new(min, max),
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
        self.source = Some(source);
    }
    fn get_audio_sample(&self) -> f32 {
        if self.source.is_some() {
            // self.effect
            //     .process(self.source.unwrap().borrow().get_audio_sample())
            0.0
        } else {
            0.0
        }
    }
}

pub struct Gain {
    source: Rc<RefCell<dyn DeviceTrait>>,
    effect: MiniGain,
}

impl Gain {
    pub fn new(source: Rc<RefCell<dyn DeviceTrait>>, amount: f32) -> Self {
        Self {
            source,
            effect: MiniGain::new(amount),
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
        self.effect.process(self.source.borrow().get_audio_sample())
    }
}

pub struct Bitcrusher {
    source: Option<Rc<RefCell<dyn DeviceTrait>>>,
    effect: primitives::bitcrusher::Bitcrusher,
    time_seconds: f32,
}

impl Bitcrusher {
    pub fn new() -> Self {
        Self {
            source: None,
            effect: primitives::bitcrusher::Bitcrusher::new(8),
            time_seconds: 0.0,
        }
    }
}

impl DeviceTrait for Bitcrusher {
    fn sources_audio(&self) -> bool {
        true
    }

    fn sinks_audio(&self) -> bool {
        true
    }

    fn add_audio_source(&mut self, source: Rc<RefCell<dyn DeviceTrait>>) {
        self.source = Some(source);
    }

    fn tick(&mut self, clock: &primitives::clock::Clock) -> bool {
        self.time_seconds = clock.seconds;
        true
    }

    fn get_audio_sample(&self) -> f32 {
        if self.source.is_some() {
            // self.effect.process(
            //     self.source.unwrap().borrow().get_audio_sample(),
            //     self.time_seconds,
            0.0
            //)
        } else {
            0.0
        }
    }
}
