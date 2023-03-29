// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! This crate provides the [crate::Orchestrator] struct, which coordinates the
//! generation of audio from all the [Entities](entities::Entity) in the
//! system.

pub use entities::{Entity, EntityNano, OtherEntityMessage};
pub use orchestrator::{Orchestrator, Performance};

pub mod helpers;
pub mod messages;

mod entities;
mod orchestrator;
mod util;

#[cfg(feature = "metrics")]
mod metrics;

use groove_core::ParameterType;

// TODO: these should be #[cfg(test)] because nobody should be assuming these
// values
pub const DEFAULT_SAMPLE_RATE: usize = 44100;
pub const DEFAULT_BPM: ParameterType = 128.0;
pub const DEFAULT_TIME_SIGNATURE: (usize, usize) = (4, 4);
pub const DEFAULT_MIDI_TICKS_PER_SECOND: usize = 960;
