#![feature(specialization)]
#![feature(trait_upcasting)]
#![allow(incomplete_features)]
#![allow(clippy::box_default)]

pub use crate::{
    clock::{Clock, TimeSignature},
    controllers::{
        arpeggiator::Arpeggiator,
        orchestrator::{GrooveOrchestrator, Orchestrator},
        sequencers::BeatSequencer,
    },
    effects::{bitcrusher::Bitcrusher, filter::BiQuadFilter, gain::Gain, limiter::Limiter},
    entities::BoxedEntity,
    gui::GrooveSubscription,
    helpers::{AudioOutput, IOHelper},
    instruments::{drumkit_sampler::DrumkitSampler, sampler::Sampler, welsh::WelshSynth},
    messages::{EntityMessage, GrooveMessage},
    midi::{
        patterns::{Note, Pattern, PatternManager},
        subscription::{MidiHandlerEvent, MidiHandlerInput, MidiSubscription, PatternMessage},
        MidiHandler, MidiHandlerMessage, MidiInputStealer, MIDI_CHANNEL_RECEIVE_ALL,
    },
    settings::songs::SongSettings,
    utils::{AudioSource, Paths, TestLfo, TestSynth, Timer},
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
