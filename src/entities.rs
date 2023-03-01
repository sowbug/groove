use crate::{
    controllers::{
        arpeggiator::Arpeggiator,
        patterns::PatternManager,
        sequencers::{BeatSequencer, MidiTickSequencer},
        ControlTrip, LfoController, SignalPassthroughController, TestController, Timer,
    },
    effects::{
        bitcrusher::Bitcrusher, chorus::Chorus, compressor::Compressor, delay::Delay,
        filter::BiQuadFilter, gain::Gain, limiter::Limiter, mixer::Mixer, reverb::Reverb,
        TestEffect,
    },
    instruments::{
        drumkit::Drumkit, sampler::Sampler, welsh::WelshSynth, AudioSource, FmSynthesizer,
        SimpleSynthesizer, TestInstrument, TestSynth,
    },
    messages::EntityMessage,
};
use groove_core::{
    midi::HandlesMidi,
    traits::{Controllable, HasUid, IsController, IsEffect, IsInstrument},
};

// PRO TIP: use `cargo expand --lib entities` to see what's being generated

macro_rules! boxed_entity_enum_and_common_crackers {
    ($($variant:ident: $type:ty,)*) => {
        #[derive(Debug)]
        pub enum Entity {
            $( $variant(Box<$type>) ),*
        }

        impl Entity {
            pub fn as_has_uid(&self) -> &dyn HasUid {
                match self {
                $( Entity::$variant(e) => e.as_ref(), )*
                }
            }
            pub fn as_has_uid_mut(&mut self) -> &mut dyn HasUid {
                match self {
                $( Entity::$variant(e) => e.as_mut(), )*
                }
            }
        }
    };
}

boxed_entity_enum_and_common_crackers! {
    // Controllers
    Arpeggiator: Arpeggiator,
    BeatSequencer: BeatSequencer,
    ControlTrip: ControlTrip,
    MidiTickSequencer:MidiTickSequencer,
    LfoController: LfoController,
    PatternManager: PatternManager,
    SignalPassthroughController: SignalPassthroughController,
    TestController: TestController,
    Timer: Timer,

    // Effects
    BiQuadFilter: BiQuadFilter,
    Bitcrusher: Bitcrusher,
    Chorus: Chorus,
    Compressor: Compressor,
    Delay: Delay,
    Gain: Gain,
    Limiter: Limiter,
    Mixer: Mixer,
    Reverb: Reverb,
    TestEffect: TestEffect,

    // Instruments
    AudioSource: AudioSource,
    Drumkit: Drumkit,
    FmSynthesizer: FmSynthesizer,
    Sampler: Sampler,
    SimpleSynthesizer: SimpleSynthesizer,
    TestInstrument: TestInstrument,
    TestSynth: TestSynth,
    WelshSynth: WelshSynth,
}

macro_rules! controllable_crackers {
    ($($type:ident,)*) => {
        impl Entity {
            pub fn as_controllable(&self) -> Option<&dyn Controllable> {
                match self {
                    $( Entity::$type(e) => Some(e.as_ref()), )*
                    _ => None,
                }
            }
            pub fn as_controllable_mut(&mut self) -> Option<&mut dyn Controllable> {
                match self {
                    $( Entity::$type(e) => Some(e.as_mut()), )*
                    _ => None,
                }
            }
        }
    };
}

controllable_crackers! {
    Arpeggiator,
    BiQuadFilter,
    Bitcrusher,
    Chorus,
    Compressor,
    Delay,
    FmSynthesizer,
    Gain,
    Limiter,
    Reverb,
    TestEffect,
    TestInstrument,
    TestSynth,
    WelshSynth,
}

macro_rules! controller_crackers {
    ($($type:ident,)*) => {
        impl Entity {
            pub fn as_is_controller(&self) -> Option<&dyn IsController<EntityMessage, Message=EntityMessage>> {
                match self {
                    $( Entity::$type(e) => Some(e.as_ref()), )*
                    _ => None,
                }
            }
            pub fn as_is_controller_mut(&mut self) -> Option<&mut dyn IsController<EntityMessage, Message=EntityMessage>> {
                match self {
                    $( Entity::$type(e) => Some(e.as_mut()), )*
                    _ => None,
                }
            }
        }
    };
}
controller_crackers! {
    Arpeggiator,
    BeatSequencer,
    ControlTrip,
    LfoController,
    MidiTickSequencer,
    PatternManager,
    SignalPassthroughController,
    TestController,
    Timer,
}

macro_rules! effect_crackers {
    ($($type:ident,)*) => {
        impl Entity {
            pub fn as_is_effect(&self) -> Option<&dyn IsEffect> {
                match self {
                $( Entity::$type(e) => Some(e.as_ref()), )*
                    _ => None,
                }
            }
            pub fn as_is_effect_mut(&mut self) -> Option<&mut dyn IsEffect> {
                match self {
                $( Entity::$type(e) => Some(e.as_mut()), )*
                    _ => None,
                }
            }
        }
    };
}
effect_crackers! {
    BiQuadFilter,
    Bitcrusher,
    Chorus,
    Compressor,
    Delay,
    Gain,
    Limiter,
    Mixer,
    Reverb,
    SignalPassthroughController,
    TestEffect,
}

macro_rules! instrument_crackers {
    ($($type:ident,)*) => {
        impl Entity {
            pub fn as_is_instrument(&self) -> Option<&dyn IsInstrument> {
                match self {
                $( Entity::$type(e) => Some(e.as_ref()), )*
                    _ => None,
                }
            }
            pub fn as_is_instrument_mut(&mut self) -> Option<&mut dyn IsInstrument> {
                match self {
                $( Entity::$type(e) => Some(e.as_mut()), )*
                    _ => None,
                }
            }
        }
    };
}
instrument_crackers! {
    AudioSource,
    Drumkit,
    FmSynthesizer,
    Sampler,
    SimpleSynthesizer,
    TestInstrument,
    TestSynth,
    WelshSynth,
}

macro_rules! handles_midi_crackers {
    ($($type:ident,)*) => {
        impl Entity {
            pub fn as_handles_midi(&self) -> Option<&dyn HandlesMidi> {
                match self {
                    $( Entity::$type(e) => Some(e.as_ref()), )*
                    _ => None
                }
            }
            pub fn as_handles_midi_mut(&mut self) -> Option<&mut dyn HandlesMidi> {
                match self {
                    $( Entity::$type(e) => Some(e.as_mut()), )*
                    _ => None
                }
            }
        }
    };
}

handles_midi_crackers! {
    Arpeggiator,
    AudioSource,
    BeatSequencer,
    ControlTrip,
    Drumkit,
    FmSynthesizer,
    LfoController,
    MidiTickSequencer,
    PatternManager,
    Sampler,
    SignalPassthroughController,
    SimpleSynthesizer,
    TestController,
    TestInstrument,
    TestSynth,
    Timer,
    WelshSynth,
}
