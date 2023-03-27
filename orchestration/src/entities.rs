// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    midi::HandlesMidi,
    traits::{Controllable, HasUid, IsController, IsEffect, IsInstrument},
};
use groove_entities::{
    controllers::{
        Arpeggiator, ControlTrip, LfoController, MidiTickSequencer, PatternManager, Sequencer,
        SignalPassthroughController, Timer,
    },
    effects::{BiQuadFilter, Bitcrusher, Chorus, Compressor, Delay, Gain, Limiter, Mixer, Reverb},
    instruments::{Drumkit, FmSynthesizer, Sampler, WelshSynth},
    EntityMessage,
};
use groove_toys::{ToyAudioSource, ToyController, ToyEffect, ToyInstrument, ToySynth};

// PRO TIP: use `cargo expand --lib entities` to see what's being generated

/// An [Entity] wraps a musical device, giving it the ability to be managed by
/// [Orchestrator] and automated by other devices in the system.
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
    Sequencer: Sequencer,
    ControlTrip: ControlTrip,
    MidiTickSequencer: MidiTickSequencer,
    LfoController: LfoController,
    PatternManager: PatternManager,
    SignalPassthroughController: SignalPassthroughController,
    ToyController: ToyController<EntityMessage>,
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
    ToyEffect: ToyEffect,

    // Instruments
    Drumkit: Drumkit,
    FmSynthesizer: FmSynthesizer,
    Sampler: Sampler,
    ToyAudioSource: ToyAudioSource,
    ToyInstrument: ToyInstrument,
    ToySynth: ToySynth,
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
    ToyEffect,
    ToyInstrument,
    ToySynth,
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
    Sequencer,
    ControlTrip,
    LfoController,
    MidiTickSequencer,
    PatternManager,
    SignalPassthroughController,
    ToyController,
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
    ToyEffect,
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
    ToyAudioSource,
    Drumkit,
    FmSynthesizer,
    Sampler,
    ToyInstrument,
    ToySynth,
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
    ToyAudioSource,
    Sequencer,
    ControlTrip,
    Drumkit,
    FmSynthesizer,
    LfoController,
    MidiTickSequencer,
    PatternManager,
    Sampler,
    SignalPassthroughController,
    ToyController,
    ToyInstrument,
    ToySynth,
    Timer,
    WelshSynth,
}

//////////////////////////
use groove_entities::{
    controllers::{
        ArpeggiatorParams, ArpeggiatorParamsMessage, LfoControllerParams,
        LfoControllerParamsMessage, PatternManagerParams, PatternManagerParamsMessage,
        SequencerParams, SequencerParamsMessage,
    },
    effects::{
        BitcrusherParams, BitcrusherParamsMessage, GainParams, GainParamsMessage, MixerParams,
        MixerParamsMessage, ReverbParams, ReverbParamsMessage,
    },
    instruments::{WelshSynthParams, WelshSynthParamsMessage},
};

macro_rules! register_impl {
    ($trait_:ident for $ty:ty, true) => {
        impl<'a> MaybeImplements<'a, dyn $trait_> for $ty {
            fn as_trait_ref(&self) -> Option<&(dyn $trait_ + 'static)> {
                Some(self)
            }
            fn as_trait_mut(&mut self) -> Option<&mut (dyn $trait_ + 'static)> {
                Some(self)
            }
        }
    };
    ($trait_:ident for $ty:ty, false) => {
        impl<'a> MaybeImplements<'a, dyn $trait_> for $ty {
            fn as_trait_ref(&self) -> Option<&(dyn $trait_ + 'static)> {
                None
            }
            fn as_trait_mut(&mut self) -> Option<&mut (dyn $trait_ + 'static)> {
                None
            }
        }
    };
}

macro_rules! all_entities {
($($entity:ident; $params:tt; $message:ident; $is_controller:tt; $is_controllable:tt ,)*) => {
    #[derive(Clone, Debug)]
    pub enum OtherEntityMessage {
        $( $params($message) ),*
    }
    #[derive(Debug)]
    pub enum EntityParams {
        $( $entity(Box<$params>) ),*
    }
    impl EntityParams {
        pub fn is_controller(&self) -> bool {
            match self {
                $( EntityParams::$entity(e) => $is_controller, )*
            }
        }
        pub fn is_controllable(&self) -> bool {
            match self {
                $( EntityParams::$entity(e) => $is_controllable, )*
            }
        }
        pub fn as_controllable_ref(&self) -> Option<&(dyn Controllable + 'static)> {
            match self {
                $( EntityParams::$entity(e) => e.as_trait_ref(), )*
            }
        }
        pub fn as_controllable_mut(&mut self) -> Option<&mut (dyn Controllable + 'static)> {
            match self {
                $( EntityParams::$entity(e) => e.as_trait_mut(), )*
            }
        }
    }
    trait MaybeImplements<'a, Trait: ?Sized> {
        fn as_trait_ref(&'a self) -> Option<&'a Trait>;
        fn as_trait_mut(&mut self) -> Option<&mut Trait>;
    }
    $( register_impl!(Controllable for $params, $is_controllable); )*
};
}

all_entities! {
    // struct; params; message; is_controller; is_controllable,

    // Controllers
    Arpeggiator; ArpeggiatorParams; ArpeggiatorParamsMessage; true; true,
    LfoController; LfoControllerParams; LfoControllerParamsMessage; true; false,
    PatternManager; PatternManagerParams; PatternManagerParamsMessage; true; false,
    Sequencer; SequencerParams; SequencerParamsMessage; false; true,

    // Effects
    Bitcrusher; BitcrusherParams; BitcrusherParamsMessage; false; true,
    Gain; GainParams; GainParamsMessage; false; true,
    Mixer; MixerParams; MixerParamsMessage; false; true,
    Reverb; ReverbParams; ReverbParamsMessage; false; true,

    // Instruments
    WelshSynth; WelshSynthParams; WelshSynthParamsMessage; false; true,
}
