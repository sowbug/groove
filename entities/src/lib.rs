// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! The suite of instruments, effects, and controllers supplied with Groove.

pub mod effects;

use groove_core::ParameterType;

// TODO: these should be #[cfg(test)] because nobody should be assuming these
// values
const DEFAULT_SAMPLE_RATE: usize = 44100;
const DEFAULT_BPM: ParameterType = 128.0;
const DEFAULT_TIME_SIGNATURE: (usize, usize) = (4, 4);
const DEFAULT_MIDI_TICKS_PER_SECOND: usize = 960;
