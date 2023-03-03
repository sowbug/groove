use groove_core::ParameterType;

// TODO: these should be #[cfg(test)] because nobody should be assuming these
// values
pub const DEFAULT_SAMPLE_RATE: usize = 44100;
pub const DEFAULT_BPM: ParameterType = 128.0;
pub const DEFAULT_TIME_SIGNATURE: (usize, usize) = (4, 4);
pub const DEFAULT_MIDI_TICKS_PER_SECOND: usize = 960;
