use crate::{
    controllers::{
        arpeggiator::Arpeggiator,
        patterns::PatternManager,
        sequencers::{BeatSequencer, MidiTickSequencer},
        ControlTrip, LfoController, SignalPassthroughController, Timer,
    },
    instruments::{
        drumkit::Drumkit, sampler::Sampler, welsh::WelshSynth, FmSynthesizer, SimpleSynthesizer,
    },
    messages::EntityMessage,
};
use groove_core::{
    midi::HandlesMidi,
    traits::{Controllable, HasUid, IsController, IsEffect, IsInstrument},
};
use groove_entities::effects::{
    BiQuadFilter, Gain, Limiter, Mixer, Reverb, Bitcrusher,
    Chorus, Compressor, Delay,
};
use groove_toys::{ToyAudioSource, ToyController, ToyEffect, ToyInstrument, ToySynth};

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
    SimpleSynthesizer: SimpleSynthesizer,
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
    BeatSequencer,
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
    SimpleSynthesizer,
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
    ToyController,
    ToyInstrument,
    ToySynth,
    Timer,
    WelshSynth,
}
