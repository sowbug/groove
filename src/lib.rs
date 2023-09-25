// Copyright (c) 2023 Mike Tsao. All rights reserved.

#![allow(clippy::box_default)]
#![warn(missing_docs)]
#![warn(rustdoc::missing_doc_code_examples)]

//! An audio engine designed to support a DAW (digital audio workstation).

// #[deprecated]
// pub use groove_orchestration::EntityObsolete;

use ensnare_core::prelude::ParameterType;

/// Widgets for egui
pub mod panels;

/// Temp home for minidaw research results
pub mod mini;

/// Recommended imports for first-time users.
pub mod prelude {
    pub use ensnare_core::core::StereoSample;
}

// TODO: these should be #[cfg(test)] because nobody should be assuming these
// values

#[doc(hidden)]
/// A typical sample rate.
pub const DEFAULT_SAMPLE_RATE: usize = 44100;
#[doc(hidden)]
/// A typical BPM (beats per minute) for EDM.
pub const DEFAULT_BPM: ParameterType = 128.0;
#[doc(hidden)]
/// The most common time signature
pub const DEFAULT_TIME_SIGNATURE: (usize, usize) = (4, 4);
#[doc(hidden)]
/// A typical tick-per-second rate for a MIDI file.
pub const DEFAULT_MIDI_TICKS_PER_SECOND: usize = 960;
