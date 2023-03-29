// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::traits::HasUid;
use groove_entities::{
    controllers::{
        Arpeggiator, ArpeggiatorMessage, ControlTrip, ControlTripMessage, LfoController,
        LfoControllerMessage, MidiTickSequencer, MidiTickSequencerMessage, NanoArpeggiator,
        NanoControlTrip, NanoLfoController, NanoMidiTickSequencer, NanoPatternManager,
        NanoSequencer, NanoSignalPassthroughController, NanoTimer, NanoTrigger, PatternManager,
        PatternManagerMessage, Sequencer, SequencerMessage, SignalPassthroughController,
        SignalPassthroughControllerMessage, Timer, TimerMessage, Trigger, TriggerMessage,
    },
    effects::{
        BiQuadFilter, BiQuadFilterMessage, Bitcrusher, BitcrusherMessage, Chorus, ChorusMessage,
        Compressor, CompressorMessage, Delay, DelayMessage, Gain, GainMessage, Limiter,
        LimiterMessage, Mixer, MixerMessage, NanoBiQuadFilter, NanoBitcrusher, NanoChorus,
        NanoCompressor, NanoDelay, NanoGain, NanoLimiter, NanoMixer, NanoReverb, Reverb,
        ReverbMessage,
    },
    instruments::{
        Drumkit, DrumkitMessage, FmSynth, FmSynthMessage, NanoDrumkit, NanoFmSynth, NanoSampler,
        NanoWelshSynth, Sampler, SamplerMessage, WelshSynth, WelshSynthMessage,
    },
    EntityMessage,
};
use groove_proc_macros::Everything;
use groove_toys::{
    NanoToyAudioSource, NanoToyController, NanoToyEffect, NanoToyInstrument, NanoToySynth,
    ToyAudioSource, ToyAudioSourceMessage, ToyController, ToyControllerMessage, ToyEffect,
    ToyEffectMessage, ToyInstrument, ToyInstrumentMessage, ToySynth, ToySynthMessage,
};

// PRO TIP: use `cargo expand --lib entities` to see what's being generated

// TODO: where does this docstring go?
// An [Entity] wraps a musical device, giving it the ability to be managed by
// [Orchestrator] and automated by other devices in the system.

type MsgType = EntityMessage;

/// The #[derive] macro uses [Everything] to generate a lot of boilerplate code.
/// The enum itself is otherwise unused.
#[allow(dead_code)]
#[derive(Everything)]
enum Everything {
    #[everything(controller, midi, controllable)]
    Arpeggiator(Arpeggiator),

    #[everything(effect, controllable)]
    BiQuadFilter(BiQuadFilter),

    #[everything(effect, controllable)]
    Bitcrusher(Bitcrusher),

    #[everything(effect, controllable)]
    Chorus(Chorus),

    #[everything(effect, controllable)]
    Compressor(Compressor),

    #[everything(controller, midi)]
    ControlTrip(ControlTrip),

    #[everything(effect, controllable)]
    Delay(Delay),

    #[everything(instrument, midi)]
    Drumkit(Drumkit),

    #[everything(instrument, midi, controllable)]
    FmSynth(FmSynth),

    #[everything(effect, controllable)]
    Gain(Gain),

    #[everything(controller, midi)]
    LfoController(LfoController),

    #[everything(effect, controllable)]
    Limiter(Limiter),

    #[everything(controller, midi)]
    MidiTickSequencer(MidiTickSequencer),

    #[everything(effect)]
    Mixer(Mixer),

    #[everything(controller, midi)]
    PatternManager(PatternManager),

    #[everything(effect, controllable)]
    Reverb(Reverb),

    #[everything(instrument, midi)]
    Sampler(Sampler),

    #[everything(controller, midi)]
    Sequencer(Sequencer),

    #[everything(controller, effect, midi)]
    SignalPassthroughController(SignalPassthroughController),

    #[everything(controller, midi)]
    Timer(Timer),

    #[everything(instrument, midi)]
    ToyAudioSource(ToyAudioSource),

    #[everything(controller, midi)]
    ToyController(ToyController<EntityMessage>),

    #[everything(effect, controllable)]
    ToyEffect(ToyEffect),

    #[everything(instrument, midi, controllable)]
    ToyInstrument(ToyInstrument),

    #[everything(instrument, midi, controllable)]
    ToySynth(ToySynth),

    #[everything(controller)]
    Trigger(Trigger),

    #[everything(instrument, midi, controllable)]
    WelshSynth(WelshSynth),
}
