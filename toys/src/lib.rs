// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! `groove-toys` contains "toy" entities that are useful for development. They
//! implement [Entity] traits, usually in a simple fashion. They aren't likely
//! to be useful in real music prduction.

pub use controllers::ToyControllerAlwaysSendsMidiMessage;
pub use effects::{ToyEffect, ToyEffectParams};
pub use instruments::{
    DebugSynth, DebugSynthParams, ToyAudioSource, ToyAudioSourceParams, ToyInstrument,
    ToyInstrumentParams, ToySynth, ToySynthParams,
};

mod controllers;
mod effects;
mod instruments;
