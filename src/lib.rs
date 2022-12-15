#![feature(specialization)]
#![feature(trait_upcasting)]
#![allow(incomplete_features)]
#![allow(clippy::box_default)]

pub use crate::{
    clock::{Clock, TimeSignature},
    controllers::orchestrator::{GrooveOrchestrator, Orchestrator},
    gui::GrooveSubscription,
    helpers::{AudioOutput, IOHelper},
    messages::GrooveMessage,
    midi::gui::MidiHandlerEvent,
    midi::gui::MidiHandlerInput,
    midi::gui::MidiSubscription,
    midi::{MidiHandler, MidiHandlerMessage, MidiInputStealer, MIDI_CHANNEL_RECEIVE_ALL},
    settings::songs::SongSettings,
    utils::Paths,
};

pub mod gui;
pub mod traits;

pub(crate) mod clock;
pub(crate) mod common;
pub(crate) mod controllers;
pub(crate) mod effects;
pub(crate) mod entities;
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
