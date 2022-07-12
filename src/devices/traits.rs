use crate::common::MidiMessage;
use crate::primitives::clock::Clock;
use crate::primitives::gain::MiniGain;
use crate::primitives::limiter::MiniLimiter;
use std::cell::RefCell;
use std::rc::Rc;

// Composition of AudioSource and AudioSink and a bunch of other stuff.
// See https://users.rust-lang.org/t/dyn-multiple-traits-in-a-type-alias/21051
pub trait DeviceTrait {
    fn sources_midi(&self) -> bool {
        false
    }
    fn sinks_midi(&self) -> bool {
        false
    }
    fn sources_audio(&self) -> bool {
        false
    }
    fn sinks_audio(&self) -> bool {
        false
    }

    // Returns whether this device has completed all it has to do.
    // A typical audio effect or instrument will always return true,
    // because it doesn't know when it's done, but false would suggest
    // that it does need to keep doing work.
    //
    // More often used for MIDI instruments.
    fn tick(&mut self, clock: &Clock) -> bool {
        true
    }
    fn get_audio_sample(&self) -> f32 {
        0.
    }
    fn add_audio_source(&mut self, device: Rc<RefCell<dyn DeviceTrait>>) {}
    fn connect_midi_sink(&mut self, device: Rc<RefCell<dyn DeviceTrait>>) {}
    fn handle_midi_message(&mut self, message: &MidiMessage, clock: &Clock) {}
}

pub struct Gain {
    source: Rc<RefCell<dyn DeviceTrait>>,
    mini_gain: MiniGain,
}
impl Gain {
    pub fn new(source: Rc<RefCell<dyn DeviceTrait>>, amount: f32) -> Self {
        Self {
            source,
            mini_gain: MiniGain::new(amount),
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
        self.mini_gain
            .process(self.source.borrow().get_audio_sample())
    }
}

pub struct Limiter {
    source: Rc<RefCell<dyn DeviceTrait>>,
    mini_limiter: MiniLimiter,
}
impl Limiter {
    pub fn new(source: Rc<RefCell<dyn DeviceTrait>>, min: f32, max: f32) -> Self {
        Self {
            source,
            mini_limiter: MiniLimiter::new(min, max),
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
        self.source = source;
    }
    fn get_audio_sample(&self) -> f32 {
        self.mini_limiter
            .process(self.source.borrow().get_audio_sample())
    }
}
