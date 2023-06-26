// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::{EntityFactory, Key, MiniSequencerParams};
use crate::mini::MiniSequencer;
use groove_core::{
    midi::MidiChannel,
    traits::{IsController, IsEffect, IsInstrument},
    Normal,
};
use groove_entities::{
    controllers::{Arpeggiator, ArpeggiatorParams, ToyController},
    effects::{BiQuadFilterLowPass24db, BiQuadFilterLowPass24dbParams, Reverb, ReverbParams},
    instruments::{Drumkit, DrumkitParams, WelshSynth, WelshSynthParams},
    EntityMessage,
};
use groove_toys::{ToyEffect, ToyInstrument, ToyInstrumentParams, ToySynth, ToySynthParams};
use groove_utils::Paths;

#[typetag::serde(tag = "type")]
pub trait NewIsController: IsController<Message = EntityMessage> {}

#[typetag::serde(tag = "type")]
pub trait NewIsInstrument: IsInstrument {}

#[typetag::serde(tag = "type")]
pub trait NewIsEffect: IsEffect {}

// TODO: I think these can be moved to each instrument, but I'm not sure and
// don't care right now.
#[typetag::serde]
impl NewIsController for Arpeggiator {}
#[typetag::serde]
impl NewIsEffect for BiQuadFilterLowPass24db {}
#[typetag::serde]
impl NewIsInstrument for Drumkit {}
#[typetag::serde]
impl NewIsController for MiniSequencer {}
#[typetag::serde]
impl NewIsEffect for Reverb {}
#[typetag::serde]
impl NewIsInstrument for WelshSynth {}
#[typetag::serde]
impl NewIsController for ToyController {}
#[typetag::serde]
impl NewIsEffect for ToyEffect {}
#[typetag::serde]
impl NewIsInstrument for ToyInstrument {}
#[typetag::serde]
impl NewIsInstrument for ToySynth {}

/// Registers all the entities we want for the minidaw example's EntityFactory.
pub fn register_mini_factory_entities(factory: &mut EntityFactory) {
    // TODO: might be nice to move HasUid::name() to be a function... and
    // while we're at it, I guess make the mondo IsEntity trait that allows
    // discovery of IsInstrument/Effect/Controller.

    factory.register_controller(Key::from("arpeggiator"), || {
        Box::new(Arpeggiator::new_with(
            &ArpeggiatorParams::default(),
            MidiChannel(0),
        ))
    });
    factory.register_controller(Key::from("sequencer"), || {
        Box::new(MiniSequencer::new_with(
            &MiniSequencerParams::default(),
            MidiChannel(0),
        ))
    });
    factory.register_effect(Key::from("reverb"), || {
        Box::new(Reverb::new_with(&ReverbParams {
            attenuation: Normal::from(0.8),
            seconds: 1.0,
            wet_dry_mix: 0.8,
        }))
    });
    factory.register_effect(Key::from("filter-low-pass-24db"), || {
        Box::new(BiQuadFilterLowPass24db::new_with(
            &BiQuadFilterLowPass24dbParams::default(),
        ))
    });
    factory.register_instrument(Key::from("toy-synth"), || {
        Box::new(ToySynth::new_with(&ToySynthParams::default()))
    });
    factory.register_instrument(Key::from("toy-instrument"), || {
        Box::new(ToyInstrument::new_with(&ToyInstrumentParams::default()))
    });
    factory.register_instrument(Key::from("welsh-synth"), || {
        Box::new(WelshSynth::new_with(&WelshSynthParams::default()))
    });
    factory.register_instrument(Key::from("drumkit"), || {
        Box::new(Drumkit::new_with(
            &DrumkitParams::default(),
            &Paths::default(),
        ))
    });
}

#[cfg(test)]
use {groove_entities::controllers::ToyControllerParams, groove_toys::ToyEffectParams};

/// Registers all the entities we want for the minidaw example's EntityFactory.
#[cfg(test)]
pub fn register_test_factory_entities(factory: &mut EntityFactory) {
    factory.register_instrument(Key::from("instrument"), || {
        Box::new(ToyInstrument::new_with(&ToyInstrumentParams::default()))
    });
    factory.register_controller(Key::from("controller"), || {
        Box::new(ToyController::new_with(
            &ToyControllerParams::default(),
            MidiChannel::from(0),
        ))
    });
    factory.register_effect(Key::from("effect"), || {
        Box::new(ToyEffect::new_with(&ToyEffectParams::default()))
    });
}
