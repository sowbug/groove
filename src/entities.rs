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
    gui::Viewable,
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

#[derive(Debug)]
pub enum BoxedEntity {
    AdsrEnvelope(Box<AdsrEnvelope>),
    Arpeggiator(Box<Arpeggiator>),
    AudioSource(Box<AudioSource<EntityMessage>>),
    BeatSequencer(Box<BeatSequencer<EntityMessage>>),
    BiQuadFilter(Box<BiQuadFilter<EntityMessage>>),
    Bitcrusher(Box<Bitcrusher>),
    ControlTrip(Box<ControlTrip<EntityMessage>>),
    Delay(Box<Delay>),
    DrumkitSampler(Box<DrumkitSampler>),
    Gain(Box<Gain<EntityMessage>>),
    Limiter(Box<Limiter>),
    MidiTickSequencer(Box<MidiTickSequencer<EntityMessage>>),
    Mixer(Box<Mixer<EntityMessage>>),
    Oscillator(Box<Oscillator>),
    PatternManager(Box<PatternManager>),
    Reverb(Box<Reverb>),
    Sampler(Box<Sampler>),
    TestController(Box<TestController<EntityMessage>>),
    TestEffect(Box<TestEffect<EntityMessage>>),
    TestInstrument(Box<TestInstrument<EntityMessage>>),
    TestLfo(Box<TestLfo<EntityMessage>>),
    TestSynth(Box<TestSynth<EntityMessage>>),
    Timer(Box<Timer<EntityMessage>>),
    WelshSynth(Box<WelshSynth>),
}

impl BoxedEntity {
    pub fn as_has_uid(&self) -> &dyn HasUid {
        match self {
            BoxedEntity::AdsrEnvelope(e) => e.as_ref(),
            BoxedEntity::Arpeggiator(e) => e.as_ref(),
            BoxedEntity::AudioSource(e) => e.as_ref(),
            BoxedEntity::BeatSequencer(e) => e.as_ref(),
            BoxedEntity::BiQuadFilter(e) => e.as_ref(),
            BoxedEntity::Bitcrusher(e) => e.as_ref(),
            BoxedEntity::ControlTrip(e) => e.as_ref(),
            BoxedEntity::Delay(e) => e.as_ref(),
            BoxedEntity::DrumkitSampler(e) => e.as_ref(),
            BoxedEntity::Gain(e) => e.as_ref(),
            BoxedEntity::Limiter(e) => e.as_ref(),
            BoxedEntity::MidiTickSequencer(e) => e.as_ref(),
            BoxedEntity::Mixer(e) => e.as_ref(),
            BoxedEntity::Oscillator(e) => e.as_ref(),
            BoxedEntity::PatternManager(e) => e.as_ref(),
            BoxedEntity::Reverb(e) => e.as_ref(),
            BoxedEntity::Sampler(e) => e.as_ref(),
            BoxedEntity::TestController(e) => e.as_ref(),
            BoxedEntity::TestEffect(e) => e.as_ref(),
            BoxedEntity::TestInstrument(e) => e.as_ref(),
            BoxedEntity::TestSynth(e) => e.as_ref(),
            BoxedEntity::Timer(e) => e.as_ref(),
            BoxedEntity::WelshSynth(e) => e.as_ref(),
            BoxedEntity::TestLfo(e) => e.as_ref(),
        }
    }
    pub fn as_has_uid_mut(&mut self) -> &mut dyn HasUid {
        match self {
            BoxedEntity::AdsrEnvelope(e) => e.as_mut(),
            BoxedEntity::Arpeggiator(e) => e.as_mut(),
            BoxedEntity::AudioSource(e) => e.as_mut(),
            BoxedEntity::BeatSequencer(e) => e.as_mut(),
            BoxedEntity::BiQuadFilter(e) => e.as_mut(),
            BoxedEntity::Bitcrusher(e) => e.as_mut(),
            BoxedEntity::ControlTrip(e) => e.as_mut(),
            BoxedEntity::Delay(e) => e.as_mut(),
            BoxedEntity::DrumkitSampler(e) => e.as_mut(),
            BoxedEntity::Gain(e) => e.as_mut(),
            BoxedEntity::Limiter(e) => e.as_mut(),
            BoxedEntity::MidiTickSequencer(e) => e.as_mut(),
            BoxedEntity::Mixer(e) => e.as_mut(),
            BoxedEntity::Oscillator(e) => e.as_mut(),
            BoxedEntity::PatternManager(e) => e.as_mut(),
            BoxedEntity::Reverb(e) => e.as_mut(),
            BoxedEntity::Sampler(e) => e.as_mut(),
            BoxedEntity::TestController(e) => e.as_mut(),
            BoxedEntity::TestEffect(e) => e.as_mut(),
            BoxedEntity::TestInstrument(e) => e.as_mut(),
            BoxedEntity::TestLfo(e) => e.as_mut(),
            BoxedEntity::TestSynth(e) => e.as_mut(),
            BoxedEntity::Timer(e) => e.as_mut(),
            BoxedEntity::WelshSynth(e) => e.as_mut(),
        }
    }
    pub fn as_updateable(&self) -> &dyn Updateable<Message = EntityMessage> {
        match self {
            BoxedEntity::AdsrEnvelope(e) => e.as_ref(),
            BoxedEntity::Arpeggiator(e) => e.as_ref(),
            BoxedEntity::AudioSource(e) => e.as_ref(),
            BoxedEntity::BeatSequencer(e) => e.as_ref(),
            BoxedEntity::BiQuadFilter(e) => e.as_ref(),
            BoxedEntity::Bitcrusher(e) => e.as_ref(),
            BoxedEntity::ControlTrip(e) => e.as_ref(),
            BoxedEntity::Delay(e) => e.as_ref(),
            BoxedEntity::DrumkitSampler(e) => e.as_ref(),
            BoxedEntity::Gain(e) => e.as_ref(),
            BoxedEntity::Limiter(e) => e.as_ref(),
            BoxedEntity::MidiTickSequencer(e) => e.as_ref(),
            BoxedEntity::Mixer(e) => e.as_ref(),
            BoxedEntity::Oscillator(e) => e.as_ref(),
            BoxedEntity::PatternManager(e) => e.as_ref(),
            BoxedEntity::Reverb(e) => e.as_ref(),
            BoxedEntity::Sampler(e) => e.as_ref(),
            BoxedEntity::TestController(e) => e.as_ref(),
            BoxedEntity::TestEffect(e) => e.as_ref(),
            BoxedEntity::TestInstrument(e) => e.as_ref(),
            BoxedEntity::TestLfo(e) => e.as_ref(),
            BoxedEntity::TestSynth(e) => e.as_ref(),
            BoxedEntity::Timer(e) => e.as_ref(),
            BoxedEntity::WelshSynth(e) => e.as_ref(),
        }
    }
    pub fn as_updateable_mut(&mut self) -> &mut dyn Updateable<Message = EntityMessage> {
        match self {
            BoxedEntity::AdsrEnvelope(e) => e.as_mut(),
            BoxedEntity::Arpeggiator(e) => e.as_mut(),
            BoxedEntity::AudioSource(e) => e.as_mut(),
            BoxedEntity::BeatSequencer(e) => e.as_mut(),
            BoxedEntity::BiQuadFilter(e) => e.as_mut(),
            BoxedEntity::Bitcrusher(e) => e.as_mut(),
            BoxedEntity::ControlTrip(e) => e.as_mut(),
            BoxedEntity::Delay(e) => e.as_mut(),
            BoxedEntity::DrumkitSampler(e) => e.as_mut(),
            BoxedEntity::Gain(e) => e.as_mut(),
            BoxedEntity::Limiter(e) => e.as_mut(),
            BoxedEntity::MidiTickSequencer(e) => e.as_mut(),
            BoxedEntity::Mixer(e) => e.as_mut(),
            BoxedEntity::Oscillator(e) => e.as_mut(),
            BoxedEntity::PatternManager(e) => e.as_mut(),
            BoxedEntity::Reverb(e) => e.as_mut(),
            BoxedEntity::Sampler(e) => e.as_mut(),
            BoxedEntity::TestController(e) => e.as_mut(),
            BoxedEntity::TestEffect(e) => e.as_mut(),
            BoxedEntity::TestInstrument(e) => e.as_mut(),
            BoxedEntity::TestLfo(e) => e.as_mut(),
            BoxedEntity::TestSynth(e) => e.as_mut(),
            BoxedEntity::Timer(e) => e.as_mut(),
            BoxedEntity::WelshSynth(e) => e.as_mut(),
        }
    }
    pub fn as_viewable(&self) -> &dyn Viewable<ViewMessage = EntityMessage> {
        match self {
            BoxedEntity::AdsrEnvelope(e) => e.as_ref(),
            BoxedEntity::Arpeggiator(e) => e.as_ref(),
            BoxedEntity::AudioSource(e) => e.as_ref(),
            BoxedEntity::BeatSequencer(e) => e.as_ref(),
            BoxedEntity::BiQuadFilter(e) => e.as_ref(),
            BoxedEntity::Bitcrusher(e) => e.as_ref(),
            BoxedEntity::ControlTrip(e) => e.as_ref(),
            BoxedEntity::Delay(e) => e.as_ref(),
            BoxedEntity::DrumkitSampler(e) => e.as_ref(),
            BoxedEntity::Gain(e) => e.as_ref(),
            BoxedEntity::Limiter(e) => e.as_ref(),
            BoxedEntity::MidiTickSequencer(e) => e.as_ref(),
            BoxedEntity::Mixer(e) => e.as_ref(),
            BoxedEntity::Oscillator(e) => e.as_ref(),
            BoxedEntity::PatternManager(e) => e.as_ref(),
            BoxedEntity::Reverb(e) => e.as_ref(),
            BoxedEntity::Sampler(e) => e.as_ref(),
            BoxedEntity::TestController(e) => e.as_ref(),
            BoxedEntity::TestEffect(e) => e.as_ref(),
            BoxedEntity::TestInstrument(e) => e.as_ref(),
            BoxedEntity::TestLfo(e) => e.as_ref(),
            BoxedEntity::TestSynth(e) => e.as_ref(),
            BoxedEntity::Timer(e) => e.as_ref(),
            BoxedEntity::WelshSynth(e) => e.as_ref(),
        }
    }
    pub fn as_terminates(&self) -> Option<&dyn Terminates> {
        match self {
            BoxedEntity::Arpeggiator(e) => Some(e.as_ref()),
            BoxedEntity::BeatSequencer(e) => Some(e.as_ref()),
            BoxedEntity::ControlTrip(e) => Some(e.as_ref()),
            BoxedEntity::MidiTickSequencer(e) => Some(e.as_ref()),
            BoxedEntity::PatternManager(e) => Some(e.as_ref()),
            BoxedEntity::TestController(e) => Some(e.as_ref()),
            BoxedEntity::TestLfo(e) => Some(e.as_ref()),
            BoxedEntity::Timer(e) => Some(e.as_ref()),
            _ => None,
        }
    }
    pub fn as_is_controller(
        &self,
    ) -> Option<&dyn IsController<Message = EntityMessage, ViewMessage = EntityMessage>> {
        match self {
            BoxedEntity::Arpeggiator(e) => Some(e.as_ref()),
            BoxedEntity::BeatSequencer(e) => Some(e.as_ref()),
            BoxedEntity::ControlTrip(e) => Some(e.as_ref()),
            BoxedEntity::MidiTickSequencer(e) => Some(e.as_ref()),
            BoxedEntity::PatternManager(e) => Some(e.as_ref()),
            BoxedEntity::TestController(e) => Some(e.as_ref()),
            BoxedEntity::TestLfo(e) => Some(e.as_ref()),
            BoxedEntity::Timer(e) => Some(e.as_ref()),
            _ => None,
        }
    }
    pub fn as_is_controller_mut(
        &mut self,
    ) -> Option<&mut dyn IsController<Message = EntityMessage, ViewMessage = EntityMessage>> {
        match self {
            BoxedEntity::Arpeggiator(e) => Some(e.as_mut()),
            BoxedEntity::BeatSequencer(e) => Some(e.as_mut()),
            BoxedEntity::ControlTrip(e) => Some(e.as_mut()),
            BoxedEntity::MidiTickSequencer(e) => Some(e.as_mut()),
            BoxedEntity::PatternManager(e) => Some(e.as_mut()),
            BoxedEntity::TestController(e) => Some(e.as_mut()),
            BoxedEntity::TestLfo(e) => Some(e.as_mut()),
            BoxedEntity::Timer(e) => Some(e.as_mut()),
            _ => None,
        }
    }
    pub fn as_is_effect(
        &self,
    ) -> Option<&dyn IsEffect<Message = EntityMessage, ViewMessage = EntityMessage>> {
        match self {
            BoxedEntity::BiQuadFilter(e) => Some(e.as_ref()),
            BoxedEntity::Bitcrusher(e) => Some(e.as_ref()),
            BoxedEntity::Delay(e) => Some(e.as_ref()),
            BoxedEntity::Gain(e) => Some(e.as_ref()),
            BoxedEntity::Limiter(e) => Some(e.as_ref()),
            BoxedEntity::Mixer(e) => Some(e.as_ref()),
            BoxedEntity::Reverb(e) => Some(e.as_ref()),
            BoxedEntity::TestEffect(e) => Some(e.as_ref()),
            _ => None,
        }
    }
    pub fn as_is_effect_mut(
        &mut self,
    ) -> Option<&mut dyn IsEffect<Message = EntityMessage, ViewMessage = EntityMessage>> {
        match self {
            BoxedEntity::BiQuadFilter(e) => Some(e.as_mut()),
            BoxedEntity::Bitcrusher(e) => Some(e.as_mut()),
            BoxedEntity::Delay(e) => Some(e.as_mut()),
            BoxedEntity::Gain(e) => Some(e.as_mut()),
            BoxedEntity::Limiter(e) => Some(e.as_mut()),
            BoxedEntity::Mixer(e) => Some(e.as_mut()),
            BoxedEntity::Reverb(e) => Some(e.as_mut()),
            BoxedEntity::TestEffect(e) => Some(e.as_mut()),
            _ => None,
        }
    }
    pub fn as_is_instrument(
        &self,
    ) -> Option<&dyn IsInstrument<Message = EntityMessage, ViewMessage = EntityMessage>> {
        match self {
            BoxedEntity::AdsrEnvelope(e) => Some(e.as_ref()),
            BoxedEntity::AudioSource(e) => Some(e.as_ref()),
            BoxedEntity::DrumkitSampler(e) => Some(e.as_ref()),
            BoxedEntity::Oscillator(e) => Some(e.as_ref()),
            BoxedEntity::Sampler(e) => Some(e.as_ref()),
            BoxedEntity::TestInstrument(e) => Some(e.as_ref()),
            BoxedEntity::TestSynth(e) => Some(e.as_ref()),
            BoxedEntity::WelshSynth(e) => Some(e.as_ref()),
            _ => None,
        }
    }
    pub fn as_is_instrument_mut(
        &mut self,
    ) -> Option<&mut dyn IsInstrument<Message = EntityMessage, ViewMessage = EntityMessage>> {
        match self {
            BoxedEntity::AdsrEnvelope(e) => Some(e.as_mut()),
            BoxedEntity::AudioSource(e) => Some(e.as_mut()),
            BoxedEntity::DrumkitSampler(e) => Some(e.as_mut()),
            BoxedEntity::Oscillator(e) => Some(e.as_mut()),
            BoxedEntity::Sampler(e) => Some(e.as_mut()),
            BoxedEntity::TestInstrument(e) => Some(e.as_mut()),
            BoxedEntity::TestSynth(e) => Some(e.as_mut()),
            BoxedEntity::WelshSynth(e) => Some(e.as_mut()),
            _ => None,
        }
    }
}
