use crate::{
    controllers::{
        arpeggiator::Arpeggiator,
        sequencers::{BeatSequencer, MidiTickSequencer},
        ControlTrip,
    },
    effects::{
        bitcrusher::Bitcrusher, delay::Delay, filter::BiQuadFilter, gain::Gain, limiter::Limiter,
        mixer::Mixer, reverb::Reverb,
    },
    instruments::{
        drumkit_sampler::DrumkitSampler, envelopes::AdsrEnvelope, oscillators::Oscillator,
        sampler::Sampler, welsh::WelshSynth,
    },
    messages::EntityMessage,
    midi::patterns::PatternManager,
    traits::{
        HasUid, IsController, IsEffect, IsInstrument, Terminates, TestController, TestEffect,
        TestInstrument, Updateable,
    },
    utils::{AudioSource, TestLfo, TestSynth, Timer},
};

// TODO: when you're REALLY bored, try generating more boilerplate for entities,
// such as HasUid. You'll need to deal with optional generics, and you'll have
// to place the code in the right modules.

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
            pub fn as_updateable(&self) -> &dyn Updateable<Message = EntityMessage> {
                match self {
                    $( BoxedEntity::$variant(e) => e.as_ref(), )*
                }
            }
            pub fn as_updateable_mut(&mut self) -> &mut dyn Updateable<Message = EntityMessage> {
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
    PatternManager: PatternManager,
    TestController: TestController<EntityMessage>,
    TestLfo: TestLfo<EntityMessage>,
    Timer: Timer<EntityMessage>,

    // Effects
    BiQuadFilter: BiQuadFilter<EntityMessage>,
    Bitcrusher: Bitcrusher,
    Delay: Delay,
    Gain: Gain<EntityMessage>,
    Limiter: Limiter,
    Mixer: Mixer<EntityMessage>,
    Reverb: Reverb,
    TestEffect: TestEffect<EntityMessage>,

    // Instruments
    AdsrEnvelope: AdsrEnvelope,
    AudioSource: AudioSource<EntityMessage>,
    DrumkitSampler: DrumkitSampler,
    Oscillator: Oscillator,
    Sampler: Sampler,
    TestInstrument: TestInstrument<EntityMessage>,
    TestSynth: TestSynth<EntityMessage>,
    WelshSynth: WelshSynth,
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
    MidiTickSequencer,
    PatternManager,
    TestController,
    TestLfo,
    Timer,
}

macro_rules! effect_crackers {
    ($($type:ident,)*) => {
        impl BoxedEntity {
            pub fn as_is_effect(&self) -> Option<&dyn IsEffect<Message = EntityMessage>> {
                match self {
                $( BoxedEntity::$type(e) => Some(e.as_ref()), )*
                    _ => None,
                }
            }
            pub fn as_is_effect_mut(&mut self) -> Option<&mut dyn IsEffect<Message = EntityMessage>> {
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
            pub fn as_is_instrument(&self) -> Option<&dyn IsInstrument<Message = EntityMessage>> {
                match self {
                $( BoxedEntity::$type(e) => Some(e.as_ref()), )*
                    _ => None,
                }
            }
            pub fn as_is_instrument_mut(&mut self) -> Option<&mut dyn IsInstrument<Message = EntityMessage>> {
                match self {
                $( BoxedEntity::$type(e) => Some(e.as_mut()), )*
                    _ => None,
                }
            }
        }
    };
}
instrument_crackers! {
    AdsrEnvelope,
    AudioSource,
    DrumkitSampler,
    Oscillator,
    Sampler,
    TestInstrument,
    TestSynth,
    WelshSynth,
}
