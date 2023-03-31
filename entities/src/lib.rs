// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! The suite of instruments, effects, and controllers supplied with Groove.

pub use messages::EntityMessage;
pub use messages::ToyMessageMaker;

/// Controllers implement the [IsController](groove_core::traits::IsController)
/// trait, which means that they control other devices. An example of a
/// controller is a [Sequencer](groove_entities::controllers::Sequencer), which
/// produces MIDI messages.
///
/// Generally, controllers produce only control signals, and not audio. But
/// adapters exist that change one kind of signal into another, such as
/// [SignalPassthroughController], which is used in
/// [sidechaining](https://en.wikipedia.org/wiki/Dynamic_range_compression#Side-chaining).
/// In theory, a similar adapter could be used to change a control signal into
/// an audio signal.
pub mod controllers;
/// Effects implement the [IsEffect](groove_core::traits::IsEffect) trait, which
/// means that they transform audio. They don't produce their own audio, and
/// while they don't produce control signals, most of them do respond to
/// controls. Examples of effects are [Compressor](crate::effects::Compressor),
/// [BiQuadFilter](crate::effects::filter::BiQuadFilter), and
/// [Reverb](crate::effects::Reverb).
pub mod effects;
/// Instruments play sounds. They implement the
/// [IsInstrument](groove_core::traits::IsInstrument) trait, which means that
/// they respond to MIDI and produce [StereoSamples](groove_core::StereoSample).
/// Examples of instruments are [Sampler](crate::instruments::Sampler) and
/// [WelshSynth](crate::instruments::WelshSynth).
pub mod instruments;
mod messages;

#[cfg(test)]
mod tests {
    use groove_core::ParameterType;

    pub(crate) const DEFAULT_SAMPLE_RATE: usize = 44100;
    pub(crate) const DEFAULT_BPM: ParameterType = 128.0;
    #[allow(dead_code)]
    pub(crate) const DEFAULT_TIME_SIGNATURE: (usize, usize) = (4, 4);
    pub(crate) const DEFAULT_MIDI_TICKS_PER_SECOND: usize = 960;
}
