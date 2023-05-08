mod entities {
    use groove_core::{
        time::{Clock, ClockParams},
        traits::{HasUid, Resets},
    };
    use groove_entities::{
        controllers::{
            Arpeggiator, ArpeggiatorParams, ControlTrip, ControlTripParams,
            LfoController, LfoControllerParams, MidiTickSequencer,
            MidiTickSequencerParams, PatternManager, PatternManagerParams, Sequencer,
            SequencerParams, SignalPassthroughController,
            SignalPassthroughControllerParams, Timer, TimerParams, Trigger, TriggerParams,
        },
        effects::{
            BiQuadFilterAllPass, BiQuadFilterAllPassParams, BiQuadFilterBandPass,
            BiQuadFilterBandPassParams, BiQuadFilterBandStop, BiQuadFilterBandStopParams,
            BiQuadFilterHighPass, BiQuadFilterHighPassParams, BiQuadFilterHighShelf,
            BiQuadFilterHighShelfParams, BiQuadFilterLowPass12db,
            BiQuadFilterLowPass12dbParams, BiQuadFilterLowPass24db,
            BiQuadFilterLowPass24dbParams, BiQuadFilterLowShelf,
            BiQuadFilterLowShelfParams, BiQuadFilterNone, BiQuadFilterNoneParams,
            BiQuadFilterPeakingEq, BiQuadFilterPeakingEqParams, Bitcrusher,
            BitcrusherParams, Chorus, ChorusParams, Compressor, CompressorParams, Delay,
            DelayParams, Gain, GainParams, Limiter, LimiterParams, Mixer, MixerParams,
            Reverb, ReverbParams,
        },
        instruments::{
            Drumkit, DrumkitParams, FmSynth, FmSynthParams, Metronome, MetronomeParams,
            Sampler, SamplerParams, WelshSynth, WelshSynthParams,
        },
        EntityMessage,
    };
    use groove_proc_macros::Everything;
    use groove_toys::{
        DebugSynth, DebugSynthParams, ToyAudioSource, ToyAudioSourceParams, ToyEffect,
        ToyEffectParams, ToyInstrument, ToyInstrumentParams, ToySynth, ToySynthParams,
    };
    /// The #[derive] macro uses [Everything] to generate a lot of boilerplate code.
    /// The enum itself is otherwise unused.
    #[cfg(not(feature = "iced-framework"))]
    #[allow(dead_code)]
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
    #[automatically_derived]
    pub enum Entity {
        Arpeggiator(Box<Arpeggiator>),
        BiQuadFilterAllPass(Box<BiQuadFilterAllPass>),
        BiQuadFilterBandPass(Box<BiQuadFilterBandPass>),
        BiQuadFilterBandStop(Box<BiQuadFilterBandStop>),
        BiQuadFilterHighPass(Box<BiQuadFilterHighPass>),
        BiQuadFilterHighShelf(Box<BiQuadFilterHighShelf>),
        BiQuadFilterLowPass12db(Box<BiQuadFilterLowPass12db>),
        BiQuadFilterLowPass24db(Box<BiQuadFilterLowPass24db>),
        BiQuadFilterLowShelf(Box<BiQuadFilterLowShelf>),
        BiQuadFilterNone(Box<BiQuadFilterNone>),
        BiQuadFilterPeakingEq(Box<BiQuadFilterPeakingEq>),
        Bitcrusher(Box<Bitcrusher>),
        Chorus(Box<Chorus>),
        Clock(Box<Clock>),
        Compressor(Box<Compressor>),
        ControlTrip(Box<ControlTrip>),
        DebugSynth(Box<DebugSynth>),
        Delay(Box<Delay>),
        Drumkit(Box<Drumkit>),
        FmSynth(Box<FmSynth>),
        Gain(Box<Gain>),
        LfoController(Box<LfoController>),
        Limiter(Box<Limiter>),
        Metronome(Box<Metronome>),
        MidiTickSequencer(Box<MidiTickSequencer>),
        Mixer(Box<Mixer>),
        PatternManager(Box<PatternManager>),
        Reverb(Box<Reverb>),
        Sampler(Box<Sampler>),
        Sequencer(Box<Sequencer>),
        SignalPassthroughController(Box<SignalPassthroughController>),
        Timer(Box<Timer>),
        ToyAudioSource(Box<ToyAudioSource>),
        ToyEffect(Box<ToyEffect>),
        ToyInstrument(Box<ToyInstrument>),
        ToySynth(Box<ToySynth>),
        Trigger(Box<Trigger>),
        WelshSynth(Box<WelshSynth>),
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Entity {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                Entity::Arpeggiator(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Arpeggiator",
                        &__self_0,
                    )
                }
                Entity::BiQuadFilterAllPass(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "BiQuadFilterAllPass",
                        &__self_0,
                    )
                }
                Entity::BiQuadFilterBandPass(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "BiQuadFilterBandPass",
                        &__self_0,
                    )
                }
                Entity::BiQuadFilterBandStop(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "BiQuadFilterBandStop",
                        &__self_0,
                    )
                }
                Entity::BiQuadFilterHighPass(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "BiQuadFilterHighPass",
                        &__self_0,
                    )
                }
                Entity::BiQuadFilterHighShelf(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "BiQuadFilterHighShelf",
                        &__self_0,
                    )
                }
                Entity::BiQuadFilterLowPass12db(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "BiQuadFilterLowPass12db",
                        &__self_0,
                    )
                }
                Entity::BiQuadFilterLowPass24db(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "BiQuadFilterLowPass24db",
                        &__self_0,
                    )
                }
                Entity::BiQuadFilterLowShelf(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "BiQuadFilterLowShelf",
                        &__self_0,
                    )
                }
                Entity::BiQuadFilterNone(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "BiQuadFilterNone",
                        &__self_0,
                    )
                }
                Entity::BiQuadFilterPeakingEq(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "BiQuadFilterPeakingEq",
                        &__self_0,
                    )
                }
                Entity::Bitcrusher(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Bitcrusher",
                        &__self_0,
                    )
                }
                Entity::Chorus(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Chorus",
                        &__self_0,
                    )
                }
                Entity::Clock(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Clock",
                        &__self_0,
                    )
                }
                Entity::Compressor(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Compressor",
                        &__self_0,
                    )
                }
                Entity::ControlTrip(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "ControlTrip",
                        &__self_0,
                    )
                }
                Entity::DebugSynth(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "DebugSynth",
                        &__self_0,
                    )
                }
                Entity::Delay(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Delay",
                        &__self_0,
                    )
                }
                Entity::Drumkit(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Drumkit",
                        &__self_0,
                    )
                }
                Entity::FmSynth(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "FmSynth",
                        &__self_0,
                    )
                }
                Entity::Gain(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Gain",
                        &__self_0,
                    )
                }
                Entity::LfoController(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "LfoController",
                        &__self_0,
                    )
                }
                Entity::Limiter(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Limiter",
                        &__self_0,
                    )
                }
                Entity::Metronome(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Metronome",
                        &__self_0,
                    )
                }
                Entity::MidiTickSequencer(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "MidiTickSequencer",
                        &__self_0,
                    )
                }
                Entity::Mixer(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Mixer",
                        &__self_0,
                    )
                }
                Entity::PatternManager(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "PatternManager",
                        &__self_0,
                    )
                }
                Entity::Reverb(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Reverb",
                        &__self_0,
                    )
                }
                Entity::Sampler(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Sampler",
                        &__self_0,
                    )
                }
                Entity::Sequencer(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Sequencer",
                        &__self_0,
                    )
                }
                Entity::SignalPassthroughController(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "SignalPassthroughController",
                        &__self_0,
                    )
                }
                Entity::Timer(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Timer",
                        &__self_0,
                    )
                }
                Entity::ToyAudioSource(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "ToyAudioSource",
                        &__self_0,
                    )
                }
                Entity::ToyEffect(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "ToyEffect",
                        &__self_0,
                    )
                }
                Entity::ToyInstrument(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "ToyInstrument",
                        &__self_0,
                    )
                }
                Entity::ToySynth(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "ToySynth",
                        &__self_0,
                    )
                }
                Entity::Trigger(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Trigger",
                        &__self_0,
                    )
                }
                Entity::WelshSynth(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "WelshSynth",
                        &__self_0,
                    )
                }
            }
        }
    }
    pub enum EntityParams {
        Arpeggiator(Box<ArpeggiatorParams>),
        BiQuadFilterAllPass(Box<BiQuadFilterAllPassParams>),
        BiQuadFilterBandPass(Box<BiQuadFilterBandPassParams>),
        BiQuadFilterBandStop(Box<BiQuadFilterBandStopParams>),
        BiQuadFilterHighPass(Box<BiQuadFilterHighPassParams>),
        BiQuadFilterHighShelf(Box<BiQuadFilterHighShelfParams>),
        BiQuadFilterLowPass12db(Box<BiQuadFilterLowPass12dbParams>),
        BiQuadFilterLowPass24db(Box<BiQuadFilterLowPass24dbParams>),
        BiQuadFilterLowShelf(Box<BiQuadFilterLowShelfParams>),
        BiQuadFilterNone(Box<BiQuadFilterNoneParams>),
        BiQuadFilterPeakingEq(Box<BiQuadFilterPeakingEqParams>),
        Bitcrusher(Box<BitcrusherParams>),
        Chorus(Box<ChorusParams>),
        Clock(Box<ClockParams>),
        Compressor(Box<CompressorParams>),
        ControlTrip(Box<ControlTripParams>),
        DebugSynth(Box<DebugSynthParams>),
        Delay(Box<DelayParams>),
        Drumkit(Box<DrumkitParams>),
        FmSynth(Box<FmSynthParams>),
        Gain(Box<GainParams>),
        LfoController(Box<LfoControllerParams>),
        Limiter(Box<LimiterParams>),
        Metronome(Box<MetronomeParams>),
        MidiTickSequencer(Box<MidiTickSequencerParams>),
        Mixer(Box<MixerParams>),
        PatternManager(Box<PatternManagerParams>),
        Reverb(Box<ReverbParams>),
        Sampler(Box<SamplerParams>),
        Sequencer(Box<SequencerParams>),
        SignalPassthroughController(Box<SignalPassthroughControllerParams>),
        Timer(Box<TimerParams>),
        ToyAudioSource(Box<ToyAudioSourceParams>),
        ToyEffect(Box<ToyEffectParams>),
        ToyInstrument(Box<ToyInstrumentParams>),
        ToySynth(Box<ToySynthParams>),
        Trigger(Box<TriggerParams>),
        WelshSynth(Box<WelshSynthParams>),
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for EntityParams {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                EntityParams::Arpeggiator(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Arpeggiator",
                        &__self_0,
                    )
                }
                EntityParams::BiQuadFilterAllPass(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "BiQuadFilterAllPass",
                        &__self_0,
                    )
                }
                EntityParams::BiQuadFilterBandPass(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "BiQuadFilterBandPass",
                        &__self_0,
                    )
                }
                EntityParams::BiQuadFilterBandStop(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "BiQuadFilterBandStop",
                        &__self_0,
                    )
                }
                EntityParams::BiQuadFilterHighPass(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "BiQuadFilterHighPass",
                        &__self_0,
                    )
                }
                EntityParams::BiQuadFilterHighShelf(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "BiQuadFilterHighShelf",
                        &__self_0,
                    )
                }
                EntityParams::BiQuadFilterLowPass12db(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "BiQuadFilterLowPass12db",
                        &__self_0,
                    )
                }
                EntityParams::BiQuadFilterLowPass24db(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "BiQuadFilterLowPass24db",
                        &__self_0,
                    )
                }
                EntityParams::BiQuadFilterLowShelf(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "BiQuadFilterLowShelf",
                        &__self_0,
                    )
                }
                EntityParams::BiQuadFilterNone(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "BiQuadFilterNone",
                        &__self_0,
                    )
                }
                EntityParams::BiQuadFilterPeakingEq(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "BiQuadFilterPeakingEq",
                        &__self_0,
                    )
                }
                EntityParams::Bitcrusher(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Bitcrusher",
                        &__self_0,
                    )
                }
                EntityParams::Chorus(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Chorus",
                        &__self_0,
                    )
                }
                EntityParams::Clock(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Clock",
                        &__self_0,
                    )
                }
                EntityParams::Compressor(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Compressor",
                        &__self_0,
                    )
                }
                EntityParams::ControlTrip(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "ControlTrip",
                        &__self_0,
                    )
                }
                EntityParams::DebugSynth(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "DebugSynth",
                        &__self_0,
                    )
                }
                EntityParams::Delay(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Delay",
                        &__self_0,
                    )
                }
                EntityParams::Drumkit(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Drumkit",
                        &__self_0,
                    )
                }
                EntityParams::FmSynth(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "FmSynth",
                        &__self_0,
                    )
                }
                EntityParams::Gain(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Gain",
                        &__self_0,
                    )
                }
                EntityParams::LfoController(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "LfoController",
                        &__self_0,
                    )
                }
                EntityParams::Limiter(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Limiter",
                        &__self_0,
                    )
                }
                EntityParams::Metronome(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Metronome",
                        &__self_0,
                    )
                }
                EntityParams::MidiTickSequencer(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "MidiTickSequencer",
                        &__self_0,
                    )
                }
                EntityParams::Mixer(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Mixer",
                        &__self_0,
                    )
                }
                EntityParams::PatternManager(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "PatternManager",
                        &__self_0,
                    )
                }
                EntityParams::Reverb(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Reverb",
                        &__self_0,
                    )
                }
                EntityParams::Sampler(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Sampler",
                        &__self_0,
                    )
                }
                EntityParams::Sequencer(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Sequencer",
                        &__self_0,
                    )
                }
                EntityParams::SignalPassthroughController(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "SignalPassthroughController",
                        &__self_0,
                    )
                }
                EntityParams::Timer(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Timer",
                        &__self_0,
                    )
                }
                EntityParams::ToyAudioSource(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "ToyAudioSource",
                        &__self_0,
                    )
                }
                EntityParams::ToyEffect(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "ToyEffect",
                        &__self_0,
                    )
                }
                EntityParams::ToyInstrument(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "ToyInstrument",
                        &__self_0,
                    )
                }
                EntityParams::ToySynth(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "ToySynth",
                        &__self_0,
                    )
                }
                EntityParams::Trigger(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Trigger",
                        &__self_0,
                    )
                }
                EntityParams::WelshSynth(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "WelshSynth",
                        &__self_0,
                    )
                }
            }
        }
    }
    #[automatically_derived]
    impl Entity {
        pub fn name(&self) -> &str {
            match self {
                Entity::Arpeggiator(e) => e.name(),
                Entity::BiQuadFilterAllPass(e) => e.name(),
                Entity::BiQuadFilterBandPass(e) => e.name(),
                Entity::BiQuadFilterBandStop(e) => e.name(),
                Entity::BiQuadFilterHighPass(e) => e.name(),
                Entity::BiQuadFilterHighShelf(e) => e.name(),
                Entity::BiQuadFilterLowPass12db(e) => e.name(),
                Entity::BiQuadFilterLowPass24db(e) => e.name(),
                Entity::BiQuadFilterLowShelf(e) => e.name(),
                Entity::BiQuadFilterNone(e) => e.name(),
                Entity::BiQuadFilterPeakingEq(e) => e.name(),
                Entity::Bitcrusher(e) => e.name(),
                Entity::Chorus(e) => e.name(),
                Entity::Clock(e) => e.name(),
                Entity::Compressor(e) => e.name(),
                Entity::ControlTrip(e) => e.name(),
                Entity::DebugSynth(e) => e.name(),
                Entity::Delay(e) => e.name(),
                Entity::Drumkit(e) => e.name(),
                Entity::FmSynth(e) => e.name(),
                Entity::Gain(e) => e.name(),
                Entity::LfoController(e) => e.name(),
                Entity::Limiter(e) => e.name(),
                Entity::Metronome(e) => e.name(),
                Entity::MidiTickSequencer(e) => e.name(),
                Entity::Mixer(e) => e.name(),
                Entity::PatternManager(e) => e.name(),
                Entity::Reverb(e) => e.name(),
                Entity::Sampler(e) => e.name(),
                Entity::Sequencer(e) => e.name(),
                Entity::SignalPassthroughController(e) => e.name(),
                Entity::Timer(e) => e.name(),
                Entity::ToyAudioSource(e) => e.name(),
                Entity::ToyEffect(e) => e.name(),
                Entity::ToyInstrument(e) => e.name(),
                Entity::ToySynth(e) => e.name(),
                Entity::Trigger(e) => e.name(),
                Entity::WelshSynth(e) => e.name(),
            }
        }
        pub fn as_has_uid(&self) -> &dyn HasUid {
            match self {
                Entity::Arpeggiator(e) => e.as_ref(),
                Entity::BiQuadFilterAllPass(e) => e.as_ref(),
                Entity::BiQuadFilterBandPass(e) => e.as_ref(),
                Entity::BiQuadFilterBandStop(e) => e.as_ref(),
                Entity::BiQuadFilterHighPass(e) => e.as_ref(),
                Entity::BiQuadFilterHighShelf(e) => e.as_ref(),
                Entity::BiQuadFilterLowPass12db(e) => e.as_ref(),
                Entity::BiQuadFilterLowPass24db(e) => e.as_ref(),
                Entity::BiQuadFilterLowShelf(e) => e.as_ref(),
                Entity::BiQuadFilterNone(e) => e.as_ref(),
                Entity::BiQuadFilterPeakingEq(e) => e.as_ref(),
                Entity::Bitcrusher(e) => e.as_ref(),
                Entity::Chorus(e) => e.as_ref(),
                Entity::Clock(e) => e.as_ref(),
                Entity::Compressor(e) => e.as_ref(),
                Entity::ControlTrip(e) => e.as_ref(),
                Entity::DebugSynth(e) => e.as_ref(),
                Entity::Delay(e) => e.as_ref(),
                Entity::Drumkit(e) => e.as_ref(),
                Entity::FmSynth(e) => e.as_ref(),
                Entity::Gain(e) => e.as_ref(),
                Entity::LfoController(e) => e.as_ref(),
                Entity::Limiter(e) => e.as_ref(),
                Entity::Metronome(e) => e.as_ref(),
                Entity::MidiTickSequencer(e) => e.as_ref(),
                Entity::Mixer(e) => e.as_ref(),
                Entity::PatternManager(e) => e.as_ref(),
                Entity::Reverb(e) => e.as_ref(),
                Entity::Sampler(e) => e.as_ref(),
                Entity::Sequencer(e) => e.as_ref(),
                Entity::SignalPassthroughController(e) => e.as_ref(),
                Entity::Timer(e) => e.as_ref(),
                Entity::ToyAudioSource(e) => e.as_ref(),
                Entity::ToyEffect(e) => e.as_ref(),
                Entity::ToyInstrument(e) => e.as_ref(),
                Entity::ToySynth(e) => e.as_ref(),
                Entity::Trigger(e) => e.as_ref(),
                Entity::WelshSynth(e) => e.as_ref(),
            }
        }
        pub fn as_has_uid_mut(&mut self) -> &mut dyn HasUid {
            match self {
                Entity::Arpeggiator(e) => e.as_mut(),
                Entity::BiQuadFilterAllPass(e) => e.as_mut(),
                Entity::BiQuadFilterBandPass(e) => e.as_mut(),
                Entity::BiQuadFilterBandStop(e) => e.as_mut(),
                Entity::BiQuadFilterHighPass(e) => e.as_mut(),
                Entity::BiQuadFilterHighShelf(e) => e.as_mut(),
                Entity::BiQuadFilterLowPass12db(e) => e.as_mut(),
                Entity::BiQuadFilterLowPass24db(e) => e.as_mut(),
                Entity::BiQuadFilterLowShelf(e) => e.as_mut(),
                Entity::BiQuadFilterNone(e) => e.as_mut(),
                Entity::BiQuadFilterPeakingEq(e) => e.as_mut(),
                Entity::Bitcrusher(e) => e.as_mut(),
                Entity::Chorus(e) => e.as_mut(),
                Entity::Clock(e) => e.as_mut(),
                Entity::Compressor(e) => e.as_mut(),
                Entity::ControlTrip(e) => e.as_mut(),
                Entity::DebugSynth(e) => e.as_mut(),
                Entity::Delay(e) => e.as_mut(),
                Entity::Drumkit(e) => e.as_mut(),
                Entity::FmSynth(e) => e.as_mut(),
                Entity::Gain(e) => e.as_mut(),
                Entity::LfoController(e) => e.as_mut(),
                Entity::Limiter(e) => e.as_mut(),
                Entity::Metronome(e) => e.as_mut(),
                Entity::MidiTickSequencer(e) => e.as_mut(),
                Entity::Mixer(e) => e.as_mut(),
                Entity::PatternManager(e) => e.as_mut(),
                Entity::Reverb(e) => e.as_mut(),
                Entity::Sampler(e) => e.as_mut(),
                Entity::Sequencer(e) => e.as_mut(),
                Entity::SignalPassthroughController(e) => e.as_mut(),
                Entity::Timer(e) => e.as_mut(),
                Entity::ToyAudioSource(e) => e.as_mut(),
                Entity::ToyEffect(e) => e.as_mut(),
                Entity::ToyInstrument(e) => e.as_mut(),
                Entity::ToySynth(e) => e.as_mut(),
                Entity::Trigger(e) => e.as_mut(),
                Entity::WelshSynth(e) => e.as_mut(),
            }
        }
        pub fn as_resets_mut(&mut self) -> &mut dyn Resets {
            match self {
                Entity::Arpeggiator(e) => e.as_mut(),
                Entity::BiQuadFilterAllPass(e) => e.as_mut(),
                Entity::BiQuadFilterBandPass(e) => e.as_mut(),
                Entity::BiQuadFilterBandStop(e) => e.as_mut(),
                Entity::BiQuadFilterHighPass(e) => e.as_mut(),
                Entity::BiQuadFilterHighShelf(e) => e.as_mut(),
                Entity::BiQuadFilterLowPass12db(e) => e.as_mut(),
                Entity::BiQuadFilterLowPass24db(e) => e.as_mut(),
                Entity::BiQuadFilterLowShelf(e) => e.as_mut(),
                Entity::BiQuadFilterNone(e) => e.as_mut(),
                Entity::BiQuadFilterPeakingEq(e) => e.as_mut(),
                Entity::Bitcrusher(e) => e.as_mut(),
                Entity::Chorus(e) => e.as_mut(),
                Entity::Clock(e) => e.as_mut(),
                Entity::Compressor(e) => e.as_mut(),
                Entity::ControlTrip(e) => e.as_mut(),
                Entity::DebugSynth(e) => e.as_mut(),
                Entity::Delay(e) => e.as_mut(),
                Entity::Drumkit(e) => e.as_mut(),
                Entity::FmSynth(e) => e.as_mut(),
                Entity::Gain(e) => e.as_mut(),
                Entity::LfoController(e) => e.as_mut(),
                Entity::Limiter(e) => e.as_mut(),
                Entity::Metronome(e) => e.as_mut(),
                Entity::MidiTickSequencer(e) => e.as_mut(),
                Entity::Mixer(e) => e.as_mut(),
                Entity::PatternManager(e) => e.as_mut(),
                Entity::Reverb(e) => e.as_mut(),
                Entity::Sampler(e) => e.as_mut(),
                Entity::Sequencer(e) => e.as_mut(),
                Entity::SignalPassthroughController(e) => e.as_mut(),
                Entity::Timer(e) => e.as_mut(),
                Entity::ToyAudioSource(e) => e.as_mut(),
                Entity::ToyEffect(e) => e.as_mut(),
                Entity::ToyInstrument(e) => e.as_mut(),
                Entity::ToySynth(e) => e.as_mut(),
                Entity::Trigger(e) => e.as_mut(),
                Entity::WelshSynth(e) => e.as_mut(),
            }
        }
    }
    #[automatically_derived]
    impl Entity {
        pub fn is_controller(&self) -> bool {
            match self {
                Entity::Arpeggiator(_) => true,
                Entity::ControlTrip(_) => true,
                Entity::LfoController(_) => true,
                Entity::MidiTickSequencer(_) => true,
                Entity::PatternManager(_) => true,
                Entity::Sequencer(_) => true,
                Entity::SignalPassthroughController(_) => true,
                Entity::Timer(_) => true,
                Entity::Trigger(_) => true,
                _ => false,
            }
        }
        pub fn as_is_controller(
            &self,
        ) -> Option<&dyn groove_core::traits::IsController<Message = MsgType>> {
            match self {
                Entity::Arpeggiator(e) => Some(e.as_ref()),
                Entity::ControlTrip(e) => Some(e.as_ref()),
                Entity::LfoController(e) => Some(e.as_ref()),
                Entity::MidiTickSequencer(e) => Some(e.as_ref()),
                Entity::PatternManager(e) => Some(e.as_ref()),
                Entity::Sequencer(e) => Some(e.as_ref()),
                Entity::SignalPassthroughController(e) => Some(e.as_ref()),
                Entity::Timer(e) => Some(e.as_ref()),
                Entity::Trigger(e) => Some(e.as_ref()),
                _ => None,
            }
        }
        pub fn as_is_controller_mut(
            &mut self,
        ) -> Option<&mut dyn groove_core::traits::IsController<Message = MsgType>> {
            match self {
                Entity::Arpeggiator(e) => Some(e.as_mut()),
                Entity::ControlTrip(e) => Some(e.as_mut()),
                Entity::LfoController(e) => Some(e.as_mut()),
                Entity::MidiTickSequencer(e) => Some(e.as_mut()),
                Entity::PatternManager(e) => Some(e.as_mut()),
                Entity::Sequencer(e) => Some(e.as_mut()),
                Entity::SignalPassthroughController(e) => Some(e.as_mut()),
                Entity::Timer(e) => Some(e.as_mut()),
                Entity::Trigger(e) => Some(e.as_mut()),
                _ => None,
            }
        }
    }
    #[automatically_derived]
    impl Entity {
        pub fn as_is_effect(&self) -> Option<&dyn groove_core::traits::IsEffect> {
            match self {
                Entity::BiQuadFilterAllPass(e) => Some(e.as_ref()),
                Entity::BiQuadFilterBandPass(e) => Some(e.as_ref()),
                Entity::BiQuadFilterBandStop(e) => Some(e.as_ref()),
                Entity::BiQuadFilterHighPass(e) => Some(e.as_ref()),
                Entity::BiQuadFilterHighShelf(e) => Some(e.as_ref()),
                Entity::BiQuadFilterLowPass12db(e) => Some(e.as_ref()),
                Entity::BiQuadFilterLowPass24db(e) => Some(e.as_ref()),
                Entity::BiQuadFilterLowShelf(e) => Some(e.as_ref()),
                Entity::BiQuadFilterNone(e) => Some(e.as_ref()),
                Entity::BiQuadFilterPeakingEq(e) => Some(e.as_ref()),
                Entity::Bitcrusher(e) => Some(e.as_ref()),
                Entity::Chorus(e) => Some(e.as_ref()),
                Entity::Compressor(e) => Some(e.as_ref()),
                Entity::Delay(e) => Some(e.as_ref()),
                Entity::Gain(e) => Some(e.as_ref()),
                Entity::Limiter(e) => Some(e.as_ref()),
                Entity::Mixer(e) => Some(e.as_ref()),
                Entity::Reverb(e) => Some(e.as_ref()),
                Entity::SignalPassthroughController(e) => Some(e.as_ref()),
                Entity::ToyEffect(e) => Some(e.as_ref()),
                _ => None,
            }
        }
        pub fn as_is_effect_mut(
            &mut self,
        ) -> Option<&mut dyn groove_core::traits::IsEffect> {
            match self {
                Entity::BiQuadFilterAllPass(e) => Some(e.as_mut()),
                Entity::BiQuadFilterBandPass(e) => Some(e.as_mut()),
                Entity::BiQuadFilterBandStop(e) => Some(e.as_mut()),
                Entity::BiQuadFilterHighPass(e) => Some(e.as_mut()),
                Entity::BiQuadFilterHighShelf(e) => Some(e.as_mut()),
                Entity::BiQuadFilterLowPass12db(e) => Some(e.as_mut()),
                Entity::BiQuadFilterLowPass24db(e) => Some(e.as_mut()),
                Entity::BiQuadFilterLowShelf(e) => Some(e.as_mut()),
                Entity::BiQuadFilterNone(e) => Some(e.as_mut()),
                Entity::BiQuadFilterPeakingEq(e) => Some(e.as_mut()),
                Entity::Bitcrusher(e) => Some(e.as_mut()),
                Entity::Chorus(e) => Some(e.as_mut()),
                Entity::Compressor(e) => Some(e.as_mut()),
                Entity::Delay(e) => Some(e.as_mut()),
                Entity::Gain(e) => Some(e.as_mut()),
                Entity::Limiter(e) => Some(e.as_mut()),
                Entity::Mixer(e) => Some(e.as_mut()),
                Entity::Reverb(e) => Some(e.as_mut()),
                Entity::SignalPassthroughController(e) => Some(e.as_mut()),
                Entity::ToyEffect(e) => Some(e.as_mut()),
                _ => None,
            }
        }
    }
    #[automatically_derived]
    impl Entity {
        pub fn as_is_instrument(
            &self,
        ) -> Option<&dyn groove_core::traits::IsInstrument> {
            match self {
                Entity::DebugSynth(e) => Some(e.as_ref()),
                Entity::Drumkit(e) => Some(e.as_ref()),
                Entity::FmSynth(e) => Some(e.as_ref()),
                Entity::Metronome(e) => Some(e.as_ref()),
                Entity::Sampler(e) => Some(e.as_ref()),
                Entity::ToyAudioSource(e) => Some(e.as_ref()),
                Entity::ToyInstrument(e) => Some(e.as_ref()),
                Entity::ToySynth(e) => Some(e.as_ref()),
                Entity::WelshSynth(e) => Some(e.as_ref()),
                _ => None,
            }
        }
        pub fn as_is_instrument_mut(
            &mut self,
        ) -> Option<&mut dyn groove_core::traits::IsInstrument> {
            match self {
                Entity::DebugSynth(e) => Some(e.as_mut()),
                Entity::Drumkit(e) => Some(e.as_mut()),
                Entity::FmSynth(e) => Some(e.as_mut()),
                Entity::Metronome(e) => Some(e.as_mut()),
                Entity::Sampler(e) => Some(e.as_mut()),
                Entity::ToyAudioSource(e) => Some(e.as_mut()),
                Entity::ToyInstrument(e) => Some(e.as_mut()),
                Entity::ToySynth(e) => Some(e.as_mut()),
                Entity::WelshSynth(e) => Some(e.as_mut()),
                _ => None,
            }
        }
    }
    #[automatically_derived]
    impl Entity {
        pub fn is_controllable(&self) -> bool {
            match self {
                Entity::Arpeggiator(_) => true,
                Entity::BiQuadFilterAllPass(_) => true,
                Entity::BiQuadFilterBandPass(_) => true,
                Entity::BiQuadFilterBandStop(_) => true,
                Entity::BiQuadFilterHighPass(_) => true,
                Entity::BiQuadFilterHighShelf(_) => true,
                Entity::BiQuadFilterLowPass12db(_) => true,
                Entity::BiQuadFilterLowPass24db(_) => true,
                Entity::BiQuadFilterLowShelf(_) => true,
                Entity::BiQuadFilterNone(_) => true,
                Entity::BiQuadFilterPeakingEq(_) => true,
                Entity::Bitcrusher(_) => true,
                Entity::Chorus(_) => true,
                Entity::Compressor(_) => true,
                Entity::DebugSynth(_) => true,
                Entity::Delay(_) => true,
                Entity::FmSynth(_) => true,
                Entity::Gain(_) => true,
                Entity::Limiter(_) => true,
                Entity::Metronome(_) => true,
                Entity::Reverb(_) => true,
                Entity::ToyEffect(_) => true,
                Entity::ToyInstrument(_) => true,
                Entity::ToySynth(_) => true,
                Entity::WelshSynth(_) => true,
                _ => false,
            }
        }
        pub fn as_controllable(&self) -> Option<&dyn groove_core::traits::Controllable> {
            match self {
                Entity::Arpeggiator(e) => Some(e.as_ref()),
                Entity::BiQuadFilterAllPass(e) => Some(e.as_ref()),
                Entity::BiQuadFilterBandPass(e) => Some(e.as_ref()),
                Entity::BiQuadFilterBandStop(e) => Some(e.as_ref()),
                Entity::BiQuadFilterHighPass(e) => Some(e.as_ref()),
                Entity::BiQuadFilterHighShelf(e) => Some(e.as_ref()),
                Entity::BiQuadFilterLowPass12db(e) => Some(e.as_ref()),
                Entity::BiQuadFilterLowPass24db(e) => Some(e.as_ref()),
                Entity::BiQuadFilterLowShelf(e) => Some(e.as_ref()),
                Entity::BiQuadFilterNone(e) => Some(e.as_ref()),
                Entity::BiQuadFilterPeakingEq(e) => Some(e.as_ref()),
                Entity::Bitcrusher(e) => Some(e.as_ref()),
                Entity::Chorus(e) => Some(e.as_ref()),
                Entity::Compressor(e) => Some(e.as_ref()),
                Entity::DebugSynth(e) => Some(e.as_ref()),
                Entity::Delay(e) => Some(e.as_ref()),
                Entity::FmSynth(e) => Some(e.as_ref()),
                Entity::Gain(e) => Some(e.as_ref()),
                Entity::Limiter(e) => Some(e.as_ref()),
                Entity::Metronome(e) => Some(e.as_ref()),
                Entity::Reverb(e) => Some(e.as_ref()),
                Entity::ToyEffect(e) => Some(e.as_ref()),
                Entity::ToyInstrument(e) => Some(e.as_ref()),
                Entity::ToySynth(e) => Some(e.as_ref()),
                Entity::WelshSynth(e) => Some(e.as_ref()),
                _ => None,
            }
        }
        pub fn as_controllable_mut(
            &mut self,
        ) -> Option<&mut dyn groove_core::traits::Controllable> {
            match self {
                Entity::Arpeggiator(e) => Some(e.as_mut()),
                Entity::BiQuadFilterAllPass(e) => Some(e.as_mut()),
                Entity::BiQuadFilterBandPass(e) => Some(e.as_mut()),
                Entity::BiQuadFilterBandStop(e) => Some(e.as_mut()),
                Entity::BiQuadFilterHighPass(e) => Some(e.as_mut()),
                Entity::BiQuadFilterHighShelf(e) => Some(e.as_mut()),
                Entity::BiQuadFilterLowPass12db(e) => Some(e.as_mut()),
                Entity::BiQuadFilterLowPass24db(e) => Some(e.as_mut()),
                Entity::BiQuadFilterLowShelf(e) => Some(e.as_mut()),
                Entity::BiQuadFilterNone(e) => Some(e.as_mut()),
                Entity::BiQuadFilterPeakingEq(e) => Some(e.as_mut()),
                Entity::Bitcrusher(e) => Some(e.as_mut()),
                Entity::Chorus(e) => Some(e.as_mut()),
                Entity::Compressor(e) => Some(e.as_mut()),
                Entity::DebugSynth(e) => Some(e.as_mut()),
                Entity::Delay(e) => Some(e.as_mut()),
                Entity::FmSynth(e) => Some(e.as_mut()),
                Entity::Gain(e) => Some(e.as_mut()),
                Entity::Limiter(e) => Some(e.as_mut()),
                Entity::Metronome(e) => Some(e.as_mut()),
                Entity::Reverb(e) => Some(e.as_mut()),
                Entity::ToyEffect(e) => Some(e.as_mut()),
                Entity::ToyInstrument(e) => Some(e.as_mut()),
                Entity::ToySynth(e) => Some(e.as_mut()),
                Entity::WelshSynth(e) => Some(e.as_mut()),
                _ => None,
            }
        }
    }
    #[automatically_derived]
    impl Entity {
        pub fn as_handles_midi(&self) -> Option<&dyn groove_core::traits::HandlesMidi> {
            match self {
                Entity::Arpeggiator(e) => Some(e.as_ref()),
                Entity::ControlTrip(e) => Some(e.as_ref()),
                Entity::DebugSynth(e) => Some(e.as_ref()),
                Entity::Drumkit(e) => Some(e.as_ref()),
                Entity::FmSynth(e) => Some(e.as_ref()),
                Entity::LfoController(e) => Some(e.as_ref()),
                Entity::Metronome(e) => Some(e.as_ref()),
                Entity::MidiTickSequencer(e) => Some(e.as_ref()),
                Entity::PatternManager(e) => Some(e.as_ref()),
                Entity::Sampler(e) => Some(e.as_ref()),
                Entity::Sequencer(e) => Some(e.as_ref()),
                Entity::SignalPassthroughController(e) => Some(e.as_ref()),
                Entity::Timer(e) => Some(e.as_ref()),
                Entity::ToyAudioSource(e) => Some(e.as_ref()),
                Entity::ToyInstrument(e) => Some(e.as_ref()),
                Entity::ToySynth(e) => Some(e.as_ref()),
                Entity::WelshSynth(e) => Some(e.as_ref()),
                _ => None,
            }
        }
        pub fn as_handles_midi_mut(
            &mut self,
        ) -> Option<&mut dyn groove_core::traits::HandlesMidi> {
            match self {
                Entity::Arpeggiator(e) => Some(e.as_mut()),
                Entity::ControlTrip(e) => Some(e.as_mut()),
                Entity::DebugSynth(e) => Some(e.as_mut()),
                Entity::Drumkit(e) => Some(e.as_mut()),
                Entity::FmSynth(e) => Some(e.as_mut()),
                Entity::LfoController(e) => Some(e.as_mut()),
                Entity::Metronome(e) => Some(e.as_mut()),
                Entity::MidiTickSequencer(e) => Some(e.as_mut()),
                Entity::PatternManager(e) => Some(e.as_mut()),
                Entity::Sampler(e) => Some(e.as_mut()),
                Entity::Sequencer(e) => Some(e.as_mut()),
                Entity::SignalPassthroughController(e) => Some(e.as_mut()),
                Entity::Timer(e) => Some(e.as_mut()),
                Entity::ToyAudioSource(e) => Some(e.as_mut()),
                Entity::ToyInstrument(e) => Some(e.as_mut()),
                Entity::ToySynth(e) => Some(e.as_mut()),
                Entity::WelshSynth(e) => Some(e.as_mut()),
                _ => None,
            }
        }
    }
}
