#![feature(specialization)]
#![feature(trait_upcasting)]
#![allow(incomplete_features)]
#![allow(clippy::box_default)]

pub use crate::clock::{Clock, TimeSignature};
pub use crate::utils::Paths;
pub use crate::{
    controllers::orchestrator::{GrooveOrchestrator, Orchestrator},
    helpers::{AudioOutput, IOHelper},
    messages::GrooveMessage,
    midi::MidiHandlerMessage,
};
pub use crate::{
    midi::{MidiHandler, MidiInputStealer, MIDI_CHANNEL_RECEIVE_ALL},
    settings::songs::SongSettings,
};

pub mod gui;
pub mod traits;

pub(crate) mod clock;
pub(crate) mod common;
pub(crate) mod controllers;
pub(crate) mod effects;
pub(crate) mod helpers;
pub(crate) mod instruments;
pub(crate) mod messages;
pub(crate) mod metrics;
pub(crate) mod midi;
pub(crate) mod settings;
pub(crate) mod utils;

#[cfg(feature = "scripting")]
pub(crate) mod scripting;
#[cfg(feature = "scripting")]
pub use crate::scripting::ScriptEngine;
