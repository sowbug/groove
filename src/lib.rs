#![allow(clippy::box_default)]

pub use crate::{
    clock::{Clock, TimeSignature},
    common::{BipolarNormal, F32ControlValue, Normal, StereoSample},
    controllers::{
        arpeggiator::Arpeggiator,
        orchestrator::Orchestrator,
        sequencers::{BeatSequencer, MidiTickSequencer},
        ControlTrip, LfoController, SignalPassthroughController,
    },
    effects::{
        bitcrusher::Bitcrusher, chorus::Chorus, compressor::Compressor, delay::Delay,
        filter::BiQuadFilter, gain::Gain, limiter::Limiter, mixer::Mixer, reverb::Reverb,
    },
    entities::BoxedEntity,
    gui::GrooveSubscription,
    helpers::{AudioOutput, IOHelper},
    instruments::{
        drumkit_sampler::DrumkitSampler, oscillators::Oscillator, sampler::Sampler,
        welsh::WelshSynth, FmSynthesizer, SimpleSynthesizer,
    },
    messages::{EntityMessage, GrooveMessage},
    midi::{
        patterns::{Note, Pattern, PatternManager},
        subscription::{MidiHandlerEvent, MidiHandlerInput, MidiSubscription, PatternMessage},
        MidiHandler, MidiHandlerMessage, MidiInputStealer,
    },
    settings::{songs::SongSettings, ClockSettings},
    utils::{AudioSource, Paths, TestSynth, Timer},
};

pub mod gui;
pub mod traits;

// https://stackoverflow.com/a/65972328/344467
pub fn app_version() -> &'static str {
    option_env!("GIT_DESCRIBE")
        .unwrap_or(option_env!("GIT_REV_PARSE").unwrap_or(env!("CARGO_PKG_VERSION")))
}

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
