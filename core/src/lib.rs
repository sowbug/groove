// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! Fundamental structs and traits.

/// This struct doesn't do anything. It exists only to let the doc system know
/// what the name of the project is.
pub struct Groove;

/// Knows about [MIDI](https://en.wikipedia.org/wiki/MIDI).
pub mod midi;
/// Handles digital-audio, wall-clock, and musical time.
pub mod time;
/// Contains various helper functions that keep different parts of the system
/// consistent.
pub mod util;

pub const SAMPLE_BUFFER_SIZE: usize = 64;
