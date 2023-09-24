// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! `groove-toys` contains "toy" entities that are useful for development. They
//! implement [Entity] traits, usually in a simple fashion. They aren't likely
//! to be useful in real music prduction.

pub use effects::{ToyEffect, ToyEffectParams};
pub use instruments::{DebugSynth, DebugSynthParams};

mod effects;
mod instruments;
