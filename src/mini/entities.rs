// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::traits::{IsController, IsEffect, IsInstrument};
use groove_entities::{
    controllers::{Arpeggiator, ToyController},
    effects::{BiQuadFilterLowPass24db, Reverb},
    instruments::{Drumkit, WelshSynth},
    EntityMessage,
};
use groove_toys::{ToyEffect, ToyInstrument, ToySynth};

use crate::mini::MiniSequencer;

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
