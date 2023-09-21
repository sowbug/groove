// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::{sequencer::SequencerBuilder, ControlTrip, EntityFactory, Key};
use ensnare::{midi::MidiChannel, prelude::*};
use groove_core::generators::Waveform;
use groove_entities::{
    controllers::{
        Arpeggiator, ArpeggiatorParams, LfoController, LfoControllerParams,
        SignalPassthroughController, Timer,
    },
    effects::{
        BiQuadFilterLowPass24db, BiQuadFilterLowPass24dbParams, Gain, GainParams, Reverb,
        ReverbParams,
    },
    instruments::{Drumkit, DrumkitParams, WelshSynth, WelshSynthParams},
};
use groove_toys::{
    ToyControllerAlwaysSendsMidiMessage, ToyInstrument, ToyInstrumentParams, ToySynth,
    ToySynthParams,
};
use groove_utils::Paths;

/// Registers all [EntityFactory]'s entities. Note that the function returns a
/// EntityFactory, rather than operating on an &mut. This encourages
/// one-and-done creation, after which the factory is immutable:
///
/// ```ignore
/// let factory = register_factory_entities(EntityFactory::default());
/// ```
#[must_use]
pub fn register_factory_entities(mut factory: EntityFactory) -> EntityFactory {
    // TODO: might be nice to move HasUid::name() to be a function.

    factory.register_entity(Key::from("arpeggiator"), || {
        Box::new(Arpeggiator::new_with(
            &ArpeggiatorParams::default(),
            MidiChannel(0),
        ))
    });
    factory.register_entity(Key::from("sequencer"), || {
        Box::new(
            SequencerBuilder::default()
                .midi_channel_out(MidiChannel(0))
                .build()
                .unwrap(),
        )
    });
    factory.register_entity(Key::from("reverb"), || {
        Box::new(Reverb::new_with(&ReverbParams {
            attenuation: Normal::from(0.8),
            seconds: 1.0,
        }))
    });
    factory.register_entity(Key::from("gain"), || {
        Box::new(Gain::new_with(&GainParams {
            ceiling: Normal::from(0.5),
        }))
    });
    // TODO: this is lazy. It's too hard right now to adjust parameters within
    // code, so I'm creating a special instrument with the parameters I want.
    factory.register_entity(Key::from("mute"), || {
        Box::new(Gain::new_with(&GainParams {
            ceiling: Normal::minimum(),
        }))
    });
    factory.register_entity(Key::from("filter-low-pass-24db"), || {
        Box::new(BiQuadFilterLowPass24db::new_with(
            &BiQuadFilterLowPass24dbParams::default(),
        ))
    });
    factory.register_entity(Key::from("timer"), || {
        Box::new(Timer::new_with(MusicalTime::DURATION_QUARTER))
    });
    factory.register_entity(Key::from("toy-synth"), || {
        Box::new(ToySynth::new_with(&ToySynthParams::default()))
    });
    factory.register_entity(Key::from("toy-instrument"), || {
        Box::new(ToyInstrument::new_with(&ToyInstrumentParams::default()))
    });
    factory.register_entity(Key::from("toy-controller-noisy"), || {
        Box::new(ToyControllerAlwaysSendsMidiMessage::default())
    });
    factory.register_entity(Key::from("welsh-synth"), || {
        Box::new(WelshSynth::new_with(&WelshSynthParams::default()))
    });
    factory.register_entity(Key::from("drumkit"), || {
        Box::new(Drumkit::new_with(
            &DrumkitParams::default(),
            &Paths::default(),
        ))
    });
    factory.register_entity(Key::from("lfo"), || {
        Box::new(LfoController::new_with(&LfoControllerParams {
            frequency: FrequencyHz::from(0.2),
            waveform: Waveform::Sawtooth,
        }))
    });
    factory.register_entity(Key::from("control-trip"), || {
        Box::new(ControlTrip::default())
    });
    factory.register_entity(Key::from("signal-passthrough"), || {
        Box::new(SignalPassthroughController::default())
    });
    factory.register_entity(Key::from("signal-amplitude-passthrough"), || {
        Box::new(SignalPassthroughController::new_amplitude_passthrough_type())
    });
    factory.register_entity(Key::from("signal-amplitude-inverted-passthrough"), || {
        Box::new(SignalPassthroughController::new_amplitude_inverted_passthrough_type())
    });

    factory.complete_registration();

    factory
}

#[cfg(test)]
use {
    groove_entities::controllers::{ToyController, ToyControllerParams},
    groove_toys::ToyEffect,
};

/// Registers all [EntityFactory]'s entities. Note that the function returns an
/// &EntityFactory. This encourages usage like this:
///
/// ```
/// let mut factory = EntityFactory::default();
/// let factory = register_test_factory_entities(&mut factory);
/// ```
///
/// This makes the factory immutable once it's set up.
#[cfg(test)]
#[must_use]
pub fn register_test_factory_entities(mut factory: EntityFactory) -> EntityFactory {
    factory.register_entity(Key::from("instrument"), || {
        Box::new(ToyInstrument::new_with(&ToyInstrumentParams::default()))
    });
    factory.register_entity(Key::from("controller"), || {
        Box::new(ToyController::new_with(
            &ToyControllerParams::default(),
            MidiChannel::from(0),
        ))
    });
    factory.register_entity(Key::from("effect"), || Box::new(ToyEffect::default()));

    factory.complete_registration();

    factory
}
