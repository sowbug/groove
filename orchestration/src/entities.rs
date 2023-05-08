// Copyright (c) 2023 Mike Tsao. All rights reserved.

#[cfg(feature = "iced-framework")]
use groove_core::time::ClockMessage;
use groove_core::{
    time::{Clock, ClockParams},
    traits::{HasUid, Resets},
};

use groove_entities::{
    controllers::{
        Arpeggiator, ArpeggiatorParams, ControlTrip, ControlTripParams, LfoController,
        LfoControllerParams, MidiTickSequencer, MidiTickSequencerParams, PatternManager,
        PatternManagerParams, Sequencer, SequencerParams, SignalPassthroughController,
        SignalPassthroughControllerParams, Timer, TimerParams, Trigger, TriggerParams,
    },
    effects::{
        BiQuadFilterAllPass, BiQuadFilterAllPassParams, BiQuadFilterBandPass,
        BiQuadFilterBandPassParams, BiQuadFilterBandStop, BiQuadFilterBandStopParams,
        BiQuadFilterHighPass, BiQuadFilterHighPassParams, BiQuadFilterHighShelf,
        BiQuadFilterHighShelfParams, BiQuadFilterLowPass12db, BiQuadFilterLowPass12dbParams,
        BiQuadFilterLowPass24db, BiQuadFilterLowPass24dbParams, BiQuadFilterLowShelf,
        BiQuadFilterLowShelfParams, BiQuadFilterNone, BiQuadFilterNoneParams,
        BiQuadFilterPeakingEq, BiQuadFilterPeakingEqParams, Bitcrusher, BitcrusherParams, Chorus,
        ChorusParams, Compressor, CompressorParams, Delay, DelayParams, Gain, GainParams, Limiter,
        LimiterParams, Mixer, MixerParams, Reverb, ReverbParams,
    },
    instruments::{
        Drumkit, DrumkitParams, FmSynth, FmSynthParams, Metronome, MetronomeParams, Sampler,
        SamplerParams, WelshSynth, WelshSynthParams,
    },
    EntityMessage,
};
use groove_proc_macros::Everything;
use groove_toys::{
    DebugSynth, DebugSynthParams, ToyAudioSource, ToyAudioSourceParams, ToyEffect, ToyEffectParams,
    ToyInstrument, ToyInstrumentParams, ToySynth, ToySynthParams,
};

#[cfg(feature = "iced-framework")]
use groove_entities::{
    controllers::{
        ArpeggiatorMessage, ControlTripMessage, LfoControllerMessage, MidiTickSequencerMessage,
        PatternManagerMessage, SequencerMessage, SignalPassthroughControllerMessage, TimerMessage,
        TriggerMessage,
    },
    effects::{
        BiQuadFilterAllPassMessage, BiQuadFilterBandPassMessage, BiQuadFilterBandStopMessage,
        BiQuadFilterHighPassMessage, BiQuadFilterHighShelfMessage, BiQuadFilterLowPass12dbMessage,
        BiQuadFilterLowPass24dbMessage, BiQuadFilterLowShelfMessage, BiQuadFilterNoneMessage,
        BitcrusherMessage, ChorusMessage, CompressorMessage, DelayMessage, GainMessage,
        LimiterMessage, MixerMessage, ReverbMessage,
    },
    instruments::{
        Drumkit, DrumkitMessage, DrumkitParams, FmSynth, FmSynthMessage, FmSynthParams, Metronome,
        MetronomeMessage, MetronomeParams, Sampler, SamplerMessage, SamplerParams, WelshSynth,
        WelshSynthMessage, WelshSynthParams,
    },
    EntityMessage,
};
#[cfg(feature = "iced-framework")]
use groove_toys::{
    DebugSynthMessage, ToyAudioSourceMessage, ToyEffectMessage, ToyInstrumentMessage,
    ToySynthMessage,
};
#[cfg(toy_controller_disabled)]
use groove_toys::{ToyController, ToyControllerMessage, ToyControllerParams};

// PRO TIP: use `cargo expand --lib entities` to see what's being generated

// TODO: where does this docstring go?
// An [Entity] wraps a musical device, giving it the ability to be managed by
// [Orchestrator] and automated by other devices in the system.

/// The #[derive] macro uses [Everything] to generate a lot of boilerplate code.
/// The enum itself is otherwise unused.
#[cfg(not(feature = "iced-framework"))]
#[allow(dead_code)]
#[derive(Everything)]
enum Everything {
    #[everything(controller, midi, controllable)]
    Arpeggiator(Arpeggiator),

    #[everything(effect, controllable)]
    BiQuadFilterAllPass(BiQuadFilterAllPass),

    #[everything(effect, controllable)]
    BiQuadFilterBandPass(BiQuadFilterBandPass),

    #[everything(effect, controllable)]
    BiQuadFilterBandStop(BiQuadFilterBandStop),

    #[everything(effect, controllable)]
    BiQuadFilterHighPass(BiQuadFilterHighPass),

    #[everything(effect, controllable)]
    BiQuadFilterHighShelf(BiQuadFilterHighShelf),

    #[everything(effect, controllable)]
    BiQuadFilterLowPass12db(BiQuadFilterLowPass12db),

    #[everything(effect, controllable)]
    BiQuadFilterLowPass24db(BiQuadFilterLowPass24db),

    #[everything(effect, controllable)]
    BiQuadFilterLowShelf(BiQuadFilterLowShelf),

    #[everything(effect, controllable)]
    BiQuadFilterNone(BiQuadFilterNone),

    #[everything(effect, controllable)]
    BiQuadFilterPeakingEq(BiQuadFilterPeakingEq),

    #[everything(effect, controllable)]
    Bitcrusher(Bitcrusher),

    #[everything(effect, controllable)]
    Chorus(Chorus),

    #[everything()]
    Clock(Clock),

    #[everything(effect, controllable)]
    Compressor(Compressor),

    #[everything(controller, midi)]
    ControlTrip(ControlTrip),

    #[everything(instrument, midi, controllable)]
    DebugSynth(DebugSynth),

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

    #[everything(controllable, instrument, midi)]
    Metronome(Metronome),

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

    #[cfg(toy_controller_disabled)]
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

#[cfg(feature = "iced-framework")]
type MsgType = EntityMessage;

/// The #[derive] macro uses [Everything] to generate a lot of boilerplate code.
/// The enum itself is otherwise unused.
#[cfg(feature = "iced-framework")]
#[allow(dead_code)]
#[derive(NanoEntities)]
enum Everything {
    #[everything(controller, midi, controllable)]
    Arpeggiator(Arpeggiator),

    #[everything(effect, controllable)]
    BiQuadFilterAllPass(BiQuadFilterAllPass),

    #[everything(effect, controllable)]
    BiQuadFilterBandPass(BiQuadFilterBandPass),

    #[everything(effect, controllable)]
    BiQuadFilterBandStop(BiQuadFilterBandStop),

    #[everything(effect, controllable)]
    BiQuadFilterHighPass(BiQuadFilterHighPass),

    #[everything(effect, controllable)]
    BiQuadFilterHighShelf(BiQuadFilterHighShelf),

    #[everything(effect, controllable)]
    BiQuadFilterLowPass12db(BiQuadFilterLowPass12db),

    #[everything(effect, controllable)]
    BiQuadFilterLowPass24db(BiQuadFilterLowPass24db),

    #[everything(effect, controllable)]
    BiQuadFilterLowShelf(BiQuadFilterLowShelf),

    #[everything(effect, controllable)]
    BiQuadFilterNone(BiQuadFilterNone),

    #[everything(effect, controllable)]
    BiQuadFilterPeakingEq(BiQuadFilterPeakingEq),

    #[everything(effect, controllable)]
    Bitcrusher(Bitcrusher),

    #[everything(effect, controllable)]
    Chorus(Chorus),

    #[everything()]
    Clock(Clock),

    #[everything(effect, controllable)]
    Compressor(Compressor),

    #[everything(controller, midi)]
    ControlTrip(ControlTrip),

    #[everything(instrument, midi, controllable)]
    DebugSynth(DebugSynth),

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

    #[everything(controllable, instrument, midi)]
    Metronome(Metronome),

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

    #[cfg(toy_controller_disabled)]
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
