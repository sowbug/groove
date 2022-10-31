#![feature(trait_upcasting)]
#![allow(incomplete_features)]

pub use crate::clock::TimeSignature;
pub use crate::helpers::AudioOutput;
pub use crate::helpers::IOHelper;
pub use crate::midi::MidiInputHandler;
pub use crate::orchestrator::Orchestrator;
pub use crate::scripting::ScriptEngine;
pub use crate::settings::songs::SongSettings;

pub mod gui;
pub mod traits;

pub(crate) mod clock;
pub(crate) mod common;
pub(crate) mod control;
pub(crate) mod effects;
pub(crate) mod envelopes;
pub(crate) mod helpers;
pub(crate) mod id_store;
pub(crate) mod midi;
pub(crate) mod orchestrator;
pub(crate) mod oscillators;
pub(crate) mod patterns;
pub(crate) mod scripting;
pub(crate) mod settings;
pub(crate) mod synthesizers;
pub(crate) mod utils;
