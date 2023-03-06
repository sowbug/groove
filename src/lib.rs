#![allow(clippy::box_default)]

//! A DAW (digital audio workstation) engine.

// TODO: regularly scrutinize the re-exports and make sure they really should be
// top-level
pub use entities::Entity;
pub use orchestrator::Orchestrator;

pub mod helpers;
pub mod messages;
pub mod midi;
pub mod subscriptions;

pub(crate) mod entities;
pub(crate) mod metrics;
pub(crate) mod orchestrator;
pub(crate) mod settings;
pub(crate) mod utils;

// https://stackoverflow.com/a/65972328/344467
pub fn app_version() -> &'static str {
    option_env!("GIT_DESCRIBE")
        .unwrap_or(option_env!("GIT_REV_PARSE").unwrap_or(env!("CARGO_PKG_VERSION")))
}

use groove_core::ParameterType;

// TODO: these should be #[cfg(test)] because nobody should be assuming these
// values
pub const DEFAULT_SAMPLE_RATE: usize = 44100;
pub const DEFAULT_BPM: ParameterType = 128.0;
pub const DEFAULT_TIME_SIGNATURE: (usize, usize) = (4, 4);
pub const DEFAULT_MIDI_TICKS_PER_SECOND: usize = 960;

#[cfg(feature = "scripting")]
pub(crate) mod scripting;
#[cfg(feature = "scripting")]
pub use crate::scripting::ScriptEngine;
