// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    time::{Clock, ClockMessage, ClockNano},
    traits::{HasUid, Resets},
};
use groove_entities::{
    controllers::{
        Arpeggiator, ArpeggiatorMessage, ArpeggiatorNano, ControlTrip, ControlTripMessage,
        ControlTripNano, LfoController, LfoControllerMessage, LfoControllerNano, MidiTickSequencer,
        MidiTickSequencerMessage, MidiTickSequencerNano, PatternManager, PatternManagerMessage,
        PatternManagerNano, Sequencer, SequencerMessage, SequencerNano,
        SignalPassthroughController, SignalPassthroughControllerMessage,
        SignalPassthroughControllerNano, Timer, TimerMessage, TimerNano, Trigger, TriggerMessage,
        TriggerNano,
    },
    effects::{
        BiQuadFilterAllPass, BiQuadFilterAllPassMessage, BiQuadFilterAllPassNano,
        BiQuadFilterBandPass, BiQuadFilterBandPassMessage, BiQuadFilterBandPassNano,
        BiQuadFilterBandStop, BiQuadFilterBandStopMessage, BiQuadFilterBandStopNano,
        BiQuadFilterHighPass, BiQuadFilterHighPassMessage, BiQuadFilterHighPassNano,
        BiQuadFilterHighShelf, BiQuadFilterHighShelfMessage, BiQuadFilterHighShelfNano,
        BiQuadFilterLowPass12db, BiQuadFilterLowPass12dbMessage, BiQuadFilterLowPass12dbNano,
        BiQuadFilterLowPass24db, BiQuadFilterLowPass24dbMessage, BiQuadFilterLowPass24dbNano,
        BiQuadFilterLowShelf, BiQuadFilterLowShelfMessage, BiQuadFilterLowShelfNano,
        BiQuadFilterNone, BiQuadFilterNoneMessage, BiQuadFilterNoneNano, BiQuadFilterPeakingEq,
        BiQuadFilterPeakingEqMessage, BiQuadFilterPeakingEqNano, Bitcrusher, BitcrusherMessage,
        BitcrusherNano, Chorus, ChorusMessage, ChorusNano, Compressor, CompressorMessage,
        CompressorNano, Delay, DelayMessage, DelayNano, Gain, GainMessage, GainNano, Limiter,
        LimiterMessage, LimiterNano, Mixer, MixerMessage, MixerNano, Reverb, ReverbMessage,
        ReverbNano,
    },
    instruments::{
        Drumkit, DrumkitMessage, DrumkitNano, FmSynth, FmSynthMessage, FmSynthNano, Metronome,
        MetronomeMessage, MetronomeNano, Sampler, SamplerMessage, SamplerNano, WelshSynth,
        WelshSynthMessage, WelshSynthNano,
    },
    EntityMessage,
};
use groove_proc_macros::Everything;
use groove_toys::{
    DebugSynth, DebugSynthMessage, DebugSynthNano, ToyAudioSource, ToyAudioSourceMessage,
    ToyAudioSourceNano, ToyController, ToyControllerMessage, ToyControllerNano, ToyEffect,
    ToyEffectMessage, ToyEffectNano, ToyInstrument, ToyInstrumentMessage, ToyInstrumentNano,
    ToySynth, ToySynthMessage, ToySynthNano,
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
