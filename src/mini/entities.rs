// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::{entity_factory::ThingType, EntityFactory, Key, MiniSequencerParams, Transport};
use crate::mini::{entity_factory::Thing, MiniSequencer};
use groove_core::{
    generators::Waveform,
    midi::MidiChannel,
    traits::{HandlesMidi, IsController, IsEffect, IsInstrument},
    FrequencyHz, Normal,
};
use groove_entities::{
    controllers::{
        Arpeggiator, ArpeggiatorParams, LfoController, LfoControllerParams, Timer, ToyController,
    },
    effects::{BiQuadFilterLowPass24db, BiQuadFilterLowPass24dbParams, Reverb, ReverbParams},
    instruments::{Drumkit, DrumkitParams, WelshSynth, WelshSynthParams},
    EntityMessage,
};
use groove_toys::{ToyEffect, ToyInstrument, ToyInstrumentParams, ToySynth, ToySynthParams};
use groove_utils::Paths;

// TODO: I think these can be moved to each instrument, but I'm not sure and
// don't care right now.
#[typetag::serde]
impl Thing for Arpeggiator {
    fn thing_type(&self) -> ThingType {
        ThingType::Controller
    }
    fn as_controller(&self) -> Option<&dyn IsController<Message = EntityMessage>> {
        Some(self)
    }
    fn as_controller_mut(&mut self) -> Option<&mut dyn IsController<Message = EntityMessage>> {
        Some(self)
    }
    fn as_handles_midi(&self) -> Option<&dyn HandlesMidi> {
        Some(self)
    }
    fn as_handles_midi_mut(&mut self) -> Option<&mut dyn HandlesMidi> {
        Some(self)
    }
}
#[typetag::serde]
impl Thing for BiQuadFilterLowPass24db {
    fn thing_type(&self) -> ThingType {
        ThingType::Effect
    }
    fn as_effect(&self) -> Option<&dyn IsEffect> {
        Some(self)
    }
    fn as_effect_mut(&mut self) -> Option<&mut dyn IsEffect> {
        Some(self)
    }
}
#[typetag::serde]
impl Thing for Drumkit {
    fn thing_type(&self) -> ThingType {
        ThingType::Instrument
    }
    fn as_instrument(&self) -> Option<&dyn IsInstrument> {
        Some(self)
    }
    fn as_instrument_mut(&mut self) -> Option<&mut dyn IsInstrument> {
        Some(self)
    }
    fn as_handles_midi(&self) -> Option<&dyn HandlesMidi> {
        Some(self)
    }
    fn as_handles_midi_mut(&mut self) -> Option<&mut dyn HandlesMidi> {
        Some(self)
    }
}
#[typetag::serde]
impl Thing for LfoController {
    fn thing_type(&self) -> ThingType {
        ThingType::Controller
    }
    fn as_controller(&self) -> Option<&dyn IsController<Message = EntityMessage>> {
        Some(self)
    }
    fn as_controller_mut(&mut self) -> Option<&mut dyn IsController<Message = EntityMessage>> {
        Some(self)
    }
    fn as_handles_midi(&self) -> Option<&dyn HandlesMidi> {
        Some(self)
    }
    fn as_handles_midi_mut(&mut self) -> Option<&mut dyn HandlesMidi> {
        Some(self)
    }
}
#[typetag::serde]
impl Thing for MiniSequencer {
    fn thing_type(&self) -> ThingType {
        ThingType::Controller
    }
    fn as_controller(&self) -> Option<&dyn IsController<Message = EntityMessage>> {
        Some(self)
    }
    fn as_controller_mut(&mut self) -> Option<&mut dyn IsController<Message = EntityMessage>> {
        Some(self)
    }
    fn as_handles_midi(&self) -> Option<&dyn HandlesMidi> {
        Some(self)
    }
    fn as_handles_midi_mut(&mut self) -> Option<&mut dyn HandlesMidi> {
        Some(self)
    }
}
#[typetag::serde]
impl Thing for Reverb {
    fn thing_type(&self) -> ThingType {
        ThingType::Effect
    }
    fn as_effect(&self) -> Option<&dyn IsEffect> {
        Some(self)
    }
    fn as_effect_mut(&mut self) -> Option<&mut dyn IsEffect> {
        Some(self)
    }
}
#[typetag::serde]
impl Thing for Timer {
    fn thing_type(&self) -> ThingType {
        ThingType::Controller
    }
    fn as_controller(&self) -> Option<&dyn IsController<Message = EntityMessage>> {
        Some(self)
    }

    fn as_controller_mut(&mut self) -> Option<&mut dyn IsController<Message = EntityMessage>> {
        Some(self)
    }

    fn as_handles_midi(&self) -> Option<&dyn HandlesMidi> {
        Some(self)
    }

    fn as_handles_midi_mut(&mut self) -> Option<&mut dyn HandlesMidi> {
        Some(self)
    }
}
#[typetag::serde]
impl Thing for ToyController {
    fn thing_type(&self) -> ThingType {
        ThingType::Controller
    }
    fn as_controller(&self) -> Option<&dyn IsController<Message = EntityMessage>> {
        Some(self)
    }
    fn as_controller_mut(&mut self) -> Option<&mut dyn IsController<Message = EntityMessage>> {
        Some(self)
    }
    fn as_handles_midi(&self) -> Option<&dyn HandlesMidi> {
        Some(self)
    }
    fn as_handles_midi_mut(&mut self) -> Option<&mut dyn HandlesMidi> {
        Some(self)
    }
}
#[typetag::serde]
impl Thing for ToyEffect {
    fn thing_type(&self) -> ThingType {
        ThingType::Effect
    }
    fn as_effect(&self) -> Option<&dyn IsEffect> {
        Some(self)
    }
    fn as_effect_mut(&mut self) -> Option<&mut dyn IsEffect> {
        Some(self)
    }
}
#[typetag::serde]
impl Thing for ToyInstrument {
    fn thing_type(&self) -> ThingType {
        ThingType::Instrument
    }
    fn as_instrument(&self) -> Option<&dyn IsInstrument> {
        Some(self)
    }
    fn as_instrument_mut(&mut self) -> Option<&mut dyn IsInstrument> {
        Some(self)
    }
    fn as_handles_midi(&self) -> Option<&dyn HandlesMidi> {
        Some(self)
    }
    fn as_handles_midi_mut(&mut self) -> Option<&mut dyn HandlesMidi> {
        Some(self)
    }
}
#[typetag::serde]
impl Thing for ToySynth {
    fn thing_type(&self) -> ThingType {
        ThingType::Instrument
    }
    fn as_instrument(&self) -> Option<&dyn IsInstrument> {
        Some(self)
    }
    fn as_instrument_mut(&mut self) -> Option<&mut dyn IsInstrument> {
        Some(self)
    }
    fn as_handles_midi(&self) -> Option<&dyn HandlesMidi> {
        Some(self)
    }
    fn as_handles_midi_mut(&mut self) -> Option<&mut dyn HandlesMidi> {
        Some(self)
    }
}
#[typetag::serde]
impl Thing for Transport {
    fn thing_type(&self) -> ThingType {
        ThingType::Controller
    }
    fn as_controller(&self) -> Option<&dyn IsController<Message = EntityMessage>> {
        Some(self)
    }
    fn as_controller_mut(&mut self) -> Option<&mut dyn IsController<Message = EntityMessage>> {
        Some(self)
    }
}
#[typetag::serde]
impl Thing for WelshSynth {
    fn thing_type(&self) -> ThingType {
        ThingType::Instrument
    }
    fn as_instrument(&self) -> Option<&dyn IsInstrument> {
        Some(self)
    }
    fn as_instrument_mut(&mut self) -> Option<&mut dyn IsInstrument> {
        Some(self)
    }
    fn as_handles_midi(&self) -> Option<&dyn HandlesMidi> {
        Some(self)
    }
    fn as_handles_midi_mut(&mut self) -> Option<&mut dyn HandlesMidi> {
        Some(self)
    }
}

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
use {groove_entities::controllers::ToyControllerParams, groove_toys::ToyEffectParams};

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
