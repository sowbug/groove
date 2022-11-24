#![feature(specialization)]
#![feature(trait_upcasting)]
#![allow(incomplete_features)]

//pub use crate::scripting::ScriptEngine;
pub use crate::clock::Clock;
pub use crate::clock::TimeSignature;
pub use crate::controllers::orchestrator::GrooveRunner;
pub use crate::controllers::orchestrator::{GrooveOrchestrator, Orchestrator};
pub use crate::helpers::AudioOutput;
pub use crate::helpers::IOHelper;
pub use crate::messages::GrooveMessage;
pub use crate::midi::MidiHandler;
pub use crate::midi::MIDI_CHANNEL_RECEIVE_ALL;
pub use crate::settings::songs::SongSettings;

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
pub(crate) mod scripting;
pub(crate) mod settings;
pub(crate) mod utils;
