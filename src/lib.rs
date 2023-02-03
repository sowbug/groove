#![feature(specialization)]
#![feature(trait_upcasting)]
#![allow(incomplete_features)]
#![allow(clippy::box_default)]

pub use crate::{
    clock::{Clock, TimeSignature},
    common::{BipolarNormal, Normal, StereoSample},
    controllers::{
        arpeggiator::Arpeggiator,
        orchestrator::{GrooveOrchestrator, Orchestrator},
        sequencers::{BeatSequencer, MidiTickSequencer},
        ControlTrip, LfoController,
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
