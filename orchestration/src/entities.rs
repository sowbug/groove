// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! An [EntityObsolete] wraps a musical device, giving it the ability to be managed by
//! [crate::Orchestrator] and automated by other devices in the system.

use groove_entities::{
    controllers::{
        Arpeggiator, Calculator, LfoController, PatternManager, Sequencer,
        SignalPassthroughController, Timer, ToyController, Trigger,
    },
    effects::{
        BiQuadFilterAllPass, BiQuadFilterBandPass, BiQuadFilterBandStop, BiQuadFilterHighPass,
        BiQuadFilterHighShelf, BiQuadFilterLowPass12db, BiQuadFilterLowPass24db,
        BiQuadFilterLowShelf, BiQuadFilterNone, BiQuadFilterPeakingEq, Bitcrusher, Chorus,
        Compressor, Delay, Gain, Limiter, Mixer, Reverb,
    },
    instruments::{Drumkit, FmSynth, Sampler, WelshSynth},
};
//use groove_proc_macros::Everything;
use groove_toys::{DebugSynth, ToyAudioSource, ToyEffect, ToyInstrument, ToySynth};

// PRO TIP: use `cargo expand --lib entities` to see what's being generated

/// The #[derive] macro uses [Everything] to generate a lot of boilerplate code.
/// The enum itself is otherwise unused.
#[allow(dead_code)]
//#[derive(Everything)]
enum Everything {
    //#[everything(controller, midi, controllable)]
    Arpeggiator(Arpeggiator),

    //#[everything(effect, controllable)]
    BiQuadFilterAllPass(BiQuadFilterAllPass),

    //#[everything(effect, controllable)]
    BiQuadFilterBandPass(BiQuadFilterBandPass),

    //#[everything(effect, controllable)]
    BiQuadFilterBandStop(BiQuadFilterBandStop),

    //#[everything(effect, controllable)]
    BiQuadFilterHighPass(BiQuadFilterHighPass),

    //#[everything(effect, controllable)]
    BiQuadFilterHighShelf(BiQuadFilterHighShelf),

    //#[everything(effect, controllable)]
    BiQuadFilterLowPass12db(BiQuadFilterLowPass12db),

    //#[everything(effect, controllable)]
    BiQuadFilterLowPass24db(BiQuadFilterLowPass24db),

    //#[everything(effect, controllable)]
    BiQuadFilterLowShelf(BiQuadFilterLowShelf),

    //#[everything(effect, controllable)]
    BiQuadFilterNone(BiQuadFilterNone),

    //#[everything(effect, controllable)]
    BiQuadFilterPeakingEq(BiQuadFilterPeakingEq),

    //#[everything(effect, controllable)]
    Bitcrusher(Bitcrusher),

    //#[everything(effect, controllable)]
    Chorus(Chorus),

    //#[everything(effect, controllable)]
    Compressor(Compressor),

    // //#[everything(controller, midi)]
    // ControlTrip(ControlTrip),
    //#[everything(instrument, midi, controllable)]
    DebugSynth(DebugSynth),

    //#[everything(effect, controllable)]
    Delay(Delay),

    //#[everything(instrument, midi)]
    Drumkit(Drumkit),

    //#[everything(instrument, midi, controllable)]
    FmSynth(FmSynth),

    //#[everything(effect, controllable)]
    Gain(Gain),

    //#[everything(instrument, controller)]
    Integrated(Calculator),

    //#[everything(controller, midi)]
    LfoController(LfoController),

    //#[everything(effect, controllable)]
    Limiter(Limiter),

    // //#[everything(controllable, instrument, midi)]
    // Metronome(Metronome),
    //#[everything(effect)]
    Mixer(Mixer),

    //#[everything(controller, midi)]
    PatternManager(PatternManager),

    //#[everything(effect, controllable)]
    Reverb(Reverb),

    //#[everything(instrument, midi)]
    Sampler(Sampler),

    //#[everything(controller, midi)]
    Sequencer(Sequencer),

    //#[everything(controller, effect, midi)]
    SignalPassthroughController(SignalPassthroughController),

    //#[everything(controller, midi)]
    Timer(Timer),

    //#[everything(instrument, midi)]
    ToyAudioSource(ToyAudioSource),

    //#[everything(controller, midi)]
    ToyController(ToyController),

    //#[everything(effect, controllable)]
    ToyEffect(ToyEffect),

    //#[everything(instrument, midi, controllable)]
    ToyInstrument(ToyInstrument),

    //#[everything(instrument, midi, controllable)]
    ToySynth(ToySynth),

    //#[everything(controller)]
    Trigger(Trigger),

    //#[everything(instrument, midi, controllable)]
    WelshSynth(WelshSynth),
}
