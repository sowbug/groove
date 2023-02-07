use crate::{
    controllers::{
        arpeggiator::Arpeggiator,
        sequencers::{BeatSequencer, MidiTickSequencer},
        ControlTrip, LfoController,
    },
    effects::{
        bitcrusher::Bitcrusher, chorus::Chorus, compressor::Compressor, delay::Delay,
        filter::BiQuadFilter, gain::Gain, limiter::Limiter, mixer::Mixer, reverb::Reverb,
    },
    instruments::{
        drumkit_sampler::DrumkitSampler, sampler::Sampler, welsh::WelshSynth, FmSynthesizer,
        HandlesMidi, SimpleSynthesizer,
    },
    messages::EntityMessage,
    midi::patterns::PatternManager,
    traits::{
        Controllable, HasUid, IsController, IsEffect, IsInstrument, Terminates, TestController,
        TestEffect, TestInstrument, Updateable,
    },
    utils::{AudioSource, TestLfo, TestSynth, Timer},
};

// PRO TIP: use `cargo expand --lib entities` to see what's being generated

macro_rules! boxed_entity_enum_and_common_crackers {
    ($($variant:ident: $type:ty,)*) => {
        #[derive(Debug)]
        pub enum BoxedEntity {
            $( $variant(Box<$type>) ),*
        }

        impl BoxedEntity {
            pub fn as_has_uid(&self) -> &dyn HasUid {
                match self {
                $( BoxedEntity::$variant(e) => e.as_ref(), )*
                }
            }
            pub fn as_has_uid_mut(&mut self) -> &mut dyn HasUid {
                match self {
                $( BoxedEntity::$variant(e) => e.as_mut(), )*
                }
            }
        }
    };
}

boxed_entity_enum_and_common_crackers! {
    // Controllers
    Arpeggiator: Arpeggiator,
    BeatSequencer: BeatSequencer<EntityMessage>,
    ControlTrip: ControlTrip<EntityMessage>,
    MidiTickSequencer:MidiTickSequencer<EntityMessage>,
    LfoController: LfoController,
    PatternManager: PatternManager,
    TestController: TestController<EntityMessage>,
    TestLfo: TestLfo<EntityMessage>,
    Timer: Timer<EntityMessage>,

    // Effects
    BiQuadFilter: BiQuadFilter<EntityMessage>,
    Bitcrusher: Bitcrusher,
    Chorus: Chorus,
    Compressor: Compressor,
    Delay: Delay,
    Gain: Gain<EntityMessage>,
    Limiter: Limiter,
    Mixer: Mixer<EntityMessage>,
    Reverb: Reverb,
    TestEffect: TestEffect<EntityMessage>,

    // Instruments
    AudioSource: AudioSource<EntityMessage>,
    DrumkitSampler: DrumkitSampler,
    FmSynthesizer: FmSynthesizer,
    Sampler: Sampler,
    SimpleSynthesizer: SimpleSynthesizer,
    TestInstrument: TestInstrument<EntityMessage>,
    TestSynth: TestSynth<EntityMessage>,
    WelshSynth: WelshSynth,
}

macro_rules! controllable_crackers {
    ($($type:ident,)*) => {
        impl BoxedEntity {
            pub fn as_controllable(&self) -> Option<&dyn Controllable> {
                match self {
                    $( BoxedEntity::$type(e) => Some(e.as_ref()), )*
                    _ => None,
                }
            }
            pub fn as_controllable_mut(&mut self) -> Option<&mut dyn Controllable> {
                match self {
                    $( BoxedEntity::$type(e) => Some(e.as_mut()), )*
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
        impl BoxedEntity {
            pub fn as_is_controller(&self) -> Option<&dyn IsController<Message = EntityMessage>> {
                match self {
                    $( BoxedEntity::$type(e) => Some(e.as_ref()), )*
                    _ => None,
                }
            }
            pub fn as_is_controller_mut(&mut self) -> Option<&mut dyn IsController<Message = EntityMessage>> {
                match self {
                    $( BoxedEntity::$type(e) => Some(e.as_mut()), )*
                    _ => None,
                }
            }
            pub fn as_terminates(&self) -> Option<&dyn Terminates> {
                match self {
                    $( BoxedEntity::$type(e) => Some(e.as_ref()), )*
                    _ => None
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
    TestController,
    TestLfo,
    Timer,
}

macro_rules! effect_crackers {
    ($($type:ident,)*) => {
        impl BoxedEntity {
            pub fn as_is_effect(&self) -> Option<&dyn IsEffect> {
                match self {
                $( BoxedEntity::$type(e) => Some(e.as_ref()), )*
                    _ => None,
                }
            }
            pub fn as_is_effect_mut(&mut self) -> Option<&mut dyn IsEffect> {
                match self {
                $( BoxedEntity::$type(e) => Some(e.as_mut()), )*
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
    TestEffect,
}

macro_rules! instrument_crackers {
    ($($type:ident,)*) => {
        impl BoxedEntity {
            pub fn as_is_instrument(&self) -> Option<&dyn IsInstrument> {
                match self {
                $( BoxedEntity::$type(e) => Some(e.as_ref()), )*
                    _ => None,
                }
            }
            pub fn as_is_instrument_mut(&mut self) -> Option<&mut dyn IsInstrument> {
                match self {
                $( BoxedEntity::$type(e) => Some(e.as_mut()), )*
                    _ => None,
                }
            }
        }
    };
}
instrument_crackers! {
    AudioSource,
    DrumkitSampler,
    FmSynthesizer,
    Sampler,
    SimpleSynthesizer,
    TestInstrument,
    TestSynth,
    WelshSynth,
}

macro_rules! updateable_crackers {
    ($($type:ident,)*) => {
        impl BoxedEntity {
            pub fn as_updateable(&self) -> Option<&dyn Updateable<Message = EntityMessage>> {
                match self {
                    $( BoxedEntity::$type(e) => Some(e.as_ref()), )*
                    _ => None
                }
            }
            pub fn as_updateable_mut(&mut self) -> Option<&mut dyn Updateable<Message = EntityMessage>> {
                match self {
                    $( BoxedEntity::$type(e) => Some(e.as_mut()), )*
                    _ => None
                }
            }
        }
    };
}

// Everything in controllers and instruments (and effects while removing the trait)
updateable_crackers! {
    Arpeggiator,
    BeatSequencer,
    ControlTrip,
    LfoController,
    MidiTickSequencer,
    PatternManager,
    TestController,
    TestEffect,
    TestInstrument,
    TestLfo,
    Timer,
}

macro_rules! handles_midi_crackers {
    ($($type:ident,)*) => {
        impl BoxedEntity {
            pub fn as_handles_midi(&self) -> Option<&dyn HandlesMidi> {
                match self {
                    $( BoxedEntity::$type(e) => Some(e.as_ref()), )*
                    _ => None
                }
            }
            pub fn as_handles_midi_mut(&mut self) -> Option<&mut dyn HandlesMidi> {
                match self {
                    $( BoxedEntity::$type(e) => Some(e.as_mut()), )*
                    _ => None
                }
            }
        }
    };
}

handles_midi_crackers! {
    AudioSource,
    DrumkitSampler,
    FmSynthesizer,
    Sampler,
    SimpleSynthesizer,
    TestSynth,
    WelshSynth,
}
