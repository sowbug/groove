// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::{EntityFactory, Key, MiniSequencerParams};
use crate::mini::MiniSequencer;
use groove_core::{generators::Waveform, midi::MidiChannel, FrequencyHz, Normal};
use groove_entities::{
    controllers::{
        Arpeggiator, ArpeggiatorParams, LfoController, LfoControllerParams, Timer, TimerParams,
    },
    effects::{BiQuadFilterLowPass24db, BiQuadFilterLowPass24dbParams, Reverb, ReverbParams},
    instruments::{Drumkit, DrumkitParams, WelshSynth, WelshSynthParams},
};
use groove_toys::{ToyInstrument, ToyInstrumentParams, ToySynth, ToySynthParams};
use groove_utils::Paths;

/// Registers all the entities we want for the minidaw example's EntityFactory.
pub fn register_mini_factory_entities(factory: &mut EntityFactory) {
    // TODO: might be nice to move HasUid::name() to be a function... and
    // while we're at it, I guess make the mondo IsEntity trait that allows
    // discovery of IsInstrument/Effect/Controller.

    factory.register_thing(Key::from("arpeggiator"), || {
        Box::new(Arpeggiator::new_with(
            &ArpeggiatorParams::default(),
            MidiChannel(0),
        ))
    });
    factory.register_thing(Key::from("sequencer"), || {
        Box::new(MiniSequencer::new_with(
            &MiniSequencerParams::default(),
            MidiChannel(0),
        ))
    });
    factory.register_thing(Key::from("reverb"), || {
        Box::new(Reverb::new_with(&ReverbParams {
            attenuation: Normal::from(0.8),
            seconds: 1.0,
        }))
    });
    factory.register_thing(Key::from("filter-low-pass-24db"), || {
        Box::new(BiQuadFilterLowPass24db::new_with(
            &BiQuadFilterLowPass24dbParams::default(),
        ))
    });
    factory.register_thing(Key::from("timer"), || {
        Box::new(Timer::new_with(&TimerParams::default()))
    });
    factory.register_thing(Key::from("toy-synth"), || {
        Box::new(ToySynth::new_with(&ToySynthParams::default()))
    });
    factory.register_thing(Key::from("toy-instrument"), || {
        Box::new(ToyInstrument::new_with(&ToyInstrumentParams::default()))
    });
    factory.register_thing(Key::from("welsh-synth"), || {
        Box::new(WelshSynth::new_with(&WelshSynthParams::default()))
    });
    factory.register_thing(Key::from("drumkit"), || {
        Box::new(Drumkit::new_with(
            &DrumkitParams::default(),
            &Paths::default(),
        ))
    });
    factory.register_thing(Key::from("lfo"), || {
        Box::new(LfoController::new_with(&LfoControllerParams {
            frequency: FrequencyHz(0.2),
            waveform: Waveform::Sawtooth,
        }))
    });
}

#[cfg(test)]
use {
    groove_entities::controllers::{ToyController, ToyControllerParams},
    groove_toys::{ToyEffect, ToyEffectParams},
};

/// Registers all the entities we want for the minidaw example's EntityFactory.
#[cfg(test)]
pub fn register_test_factory_entities(factory: &mut EntityFactory) {
    factory.register_thing(Key::from("instrument"), || {
        Box::new(ToyInstrument::new_with(&ToyInstrumentParams::default()))
    });
    factory.register_thing(Key::from("controller"), || {
        Box::new(ToyController::new_with(
            &ToyControllerParams::default(),
            MidiChannel::from(0),
        ))
    });
    factory.register_thing(Key::from("effect"), || {
        Box::new(ToyEffect::new_with(&ToyEffectParams::default()))
    });
}
