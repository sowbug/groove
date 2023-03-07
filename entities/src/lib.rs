// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! The suite of instruments, effects, and controllers supplied with Groove.

pub use messages::EntityMessage;
pub use messages::ToyMessageMaker;

pub mod controllers;
pub mod effects;
pub mod instruments;
mod messages;

#[cfg(test)]
mod tests {
    use groove_core::ParameterType;

    pub(crate) const DEFAULT_SAMPLE_RATE: usize = 44100;
    pub(crate) const DEFAULT_BPM: ParameterType = 128.0;
    #[allow(dead_code)]
    pub(crate) const DEFAULT_TIME_SIGNATURE: (usize, usize) = (4, 4);
    pub(crate) const DEFAULT_MIDI_TICKS_PER_SECOND: usize = 960;
}
