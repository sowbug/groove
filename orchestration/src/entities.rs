// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    midi::HandlesMidi,
    traits::{Controllable, HasUid, IsController, IsEffect, IsInstrument},
};
use groove_entities::{
    controllers::{
        Arpeggiator, ArpeggiatorParams, ArpeggiatorParamsMessage, ControlTrip, ControlTripParams,
        ControlTripParamsMessage, LfoController, LfoControllerParams, LfoControllerParamsMessage,
        MidiTickSequencer, MidiTickSequencerParams, MidiTickSequencerParamsMessage, PatternManager,
        PatternManagerParams, PatternManagerParamsMessage, Sequencer, SequencerParams,
        SequencerParamsMessage, SignalPassthroughController, SignalPassthroughControllerParams,
        SignalPassthroughControllerParamsMessage, Timer, TimerParams, TimerParamsMessage,
    },
    effects::{
        BiQuadFilter, BiQuadFilterParams, BiQuadFilterParamsMessage, Bitcrusher, BitcrusherParams,
        BitcrusherParamsMessage, Chorus, ChorusParams, ChorusParamsMessage, Compressor,
        CompressorParams, CompressorParamsMessage, Delay, DelayParams, DelayParamsMessage, Gain,
        GainParams, GainParamsMessage, Limiter, LimiterParams, LimiterParamsMessage, Mixer,
        MixerParams, MixerParamsMessage, Reverb, ReverbParams, ReverbParamsMessage,
    },
    instruments::{
        Drumkit, DrumkitParams, DrumkitParamsMessage, FmSynth, FmSynthParams, FmSynthParamsMessage,
        Sampler, SamplerParams, SamplerParamsMessage, WelshSynth, WelshSynthParams,
        WelshSynthParamsMessage,
    },
    EntityMessage,
};
use groove_macros::{
    all_entities, boxed_entity_enum_and_common_crackers, controllable_crackers,
    controller_crackers, effect_crackers, handles_midi_crackers, instrument_crackers,
    register_impl,
};
use groove_toys::{
    ToyAudioSource, ToyAudioSourceParams, ToyAudioSourceParamsMessage, ToyController,
    ToyControllerParams, ToyControllerParamsMessage, ToyEffect, ToyEffectParams,
    ToyEffectParamsMessage, ToyInstrument, ToyInstrumentParams, ToyInstrumentParamsMessage,
    ToySynth, ToySynthParams, ToySynthParamsMessage,
};

// PRO TIP: use `cargo expand --lib entities` to see what's being generated

// TODO: where does this docstring go?
// An [Entity] wraps a musical device, giving it the ability to be managed by
// [Orchestrator] and automated by other devices in the system.

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
    FmSynth: FmSynth,
    Sampler: Sampler,
    ToyAudioSource: ToyAudioSource,
    ToyInstrument: ToyInstrument,
    ToySynth: ToySynth,
    WelshSynth: WelshSynth,
}

controllable_crackers! {
    Arpeggiator,
    BiQuadFilter,
    Bitcrusher,
    Chorus,
    Compressor,
    Delay,
    FmSynth,
    Gain,
    Limiter,
    Reverb,
    ToyEffect,
    ToyInstrument,
    ToySynth,
    WelshSynth,
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

instrument_crackers! {
    ToyAudioSource,
    Drumkit,
    FmSynth,
    Sampler,
    ToyInstrument,
    ToySynth,
    WelshSynth,
}

handles_midi_crackers! {
    Arpeggiator,
    ToyAudioSource,
    Sequencer,
    ControlTrip,
    Drumkit,
    FmSynth,
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

all_entities! {
    // struct; params; message; is_controller; is_controllable,

    // Controllers
    Arpeggiator; ArpeggiatorParams; ArpeggiatorParamsMessage; true; true,
    ControlTrip; ControlTripParams; ControlTripParamsMessage; true; false,
    LfoController; LfoControllerParams; LfoControllerParamsMessage; true; false,
    MidiTickSequencer; MidiTickSequencerParams; MidiTickSequencerParamsMessage; true; false,
    PatternManager; PatternManagerParams; PatternManagerParamsMessage; true; false,
    Sequencer; SequencerParams; SequencerParamsMessage; true; false,
    SignalPassthroughController; SignalPassthroughControllerParams; SignalPassthroughControllerParamsMessage; true; false,
    Timer; TimerParams; TimerParamsMessage; true; false,
    ToyController; ToyControllerParams; ToyControllerParamsMessage; true; false,

    // Effects
    BiQuadFilter; BiQuadFilterParams; BiQuadFilterParamsMessage; false; false,
    Bitcrusher; BitcrusherParams; BitcrusherParamsMessage; false; true,
    Chorus; ChorusParams; ChorusParamsMessage; false; false,
    Compressor; CompressorParams; CompressorParamsMessage; false; false,
    Delay; DelayParams; DelayParamsMessage; false; false,
    Gain; GainParams; GainParamsMessage; false; true,
    Limiter; LimiterParams; LimiterParamsMessage; false; false,
    Mixer; MixerParams; MixerParamsMessage; false; true,
    Reverb; ReverbParams; ReverbParamsMessage; false; true,
// both controller and effect...    SignalPassthroughController; SignalPassthroughControllerParams; SignalPassthroughControllerParamsMessage; true; false,
    ToyEffect; ToyEffectParams; ToyEffectParamsMessage; false; false,

    // Instruments
    Drumkit; DrumkitParams; DrumkitParamsMessage; false; false,
    FmSynth; FmSynthParams; FmSynthParamsMessage; false; false,
    Sampler; SamplerParams; SamplerParamsMessage; false; false,
    ToyAudioSource; ToyAudioSourceParams; ToyAudioSourceParamsMessage; false; false,
    ToyInstrument; ToyInstrumentParams; ToyInstrumentParamsMessage; false; false,
    ToySynth; ToySynthParams; ToySynthParamsMessage; false; false,
    WelshSynth; WelshSynthParams; WelshSynthParamsMessage; false; true,
}

enum Everything {
    Arpeggiator,
    BiQuadFilter,
    Bitcrusher,
    Chorus,
    Compressor,
    ControlTrip,
    Delay,
    Drumkit,
    FmSynthesizer,
    Gain,
    LfoController,
    Limiter,
    MidiTickSequencer,
    Mixer,
    PatternManager,
    Reverb,
    Sampler,
    Sequencer,
    SignalPassthroughController,
    Timer,
    ToyAudioSource,
    ToyController,
    ToyEffect,
    ToyInstrument,
    ToySynth,
    WelshSynth,
}
