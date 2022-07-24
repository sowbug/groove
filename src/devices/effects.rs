use std::{cell::RefCell, rc::Rc};

use crate::primitives::{self, gain::MiniGain, limiter::MiniLimiter, EffectTrait};

use super::traits::DeviceTrait;
pub struct Limiter {
    source: Option<Rc<RefCell<dyn DeviceTrait>>>,
    effect: MiniLimiter,
}

impl Limiter {
    pub fn new_with_params(min: f32, max: f32) -> Self {
        Self {
            source: None,
            effect: MiniLimiter::new(min, max),
        }
    }
    pub fn new() -> Self {
        Self {
            source: None,
            effect: MiniLimiter::new(0.0, 1.0),
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
    fn get_audio_sample(&mut self) -> f32 {
        if self.source.is_some() {
            let source_ref = self.source.as_ref().unwrap();
            self.effect
                .process(source_ref.borrow_mut().get_audio_sample())
        } else {
            0.0
        }
    }
}

#[derive(Default)]
pub struct Gain {
    source: Option<Rc<RefCell<dyn DeviceTrait>>>,
    effect: MiniGain,
}

impl Gain {
    pub fn new_with_params(amount: f32) -> Self {
        Self {
            source: None,
            effect: MiniGain::new(amount), // TODO: consider new_with_params() convention
        }
    }
    pub fn new() -> Self {
        Self {
            source: None,
            effect: MiniGain::new(1.0), // TODO: what's a neutral gain?
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
        self.source = Some(source);
    }
    fn get_audio_sample(&mut self) -> f32 {
        if self.source.is_some() {
            let source_ref = self.source.as_ref().unwrap();
            self.effect
                .process(source_ref.borrow_mut().get_audio_sample())
        } else {
            0.0
        }
    }
}

pub struct Bitcrusher {
    source: Option<Rc<RefCell<dyn DeviceTrait>>>,
    effect: primitives::bitcrusher::Bitcrusher,
    time_seconds: f32,
}

impl Bitcrusher {
    pub fn new_with_params(bits_to_crush: u8) -> Self {
        Self {
            source: None,
            effect: primitives::bitcrusher::Bitcrusher::new(bits_to_crush),
            time_seconds: 0.0,
        }
    }
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

    fn get_audio_sample(&mut self) -> f32 {
        if self.source.is_some() {
            let source_ref = self.source.as_ref().unwrap();
            self.effect.process(
                source_ref.borrow_mut().get_audio_sample(),
                self.time_seconds,
            )
        } else {
            0.0
        }
    }
}
