// Copyright (c) 2023 Mike Tsao. All rights reserved.

#![allow(clippy::box_default)]
#![warn(missing_docs)]
#![warn(rustdoc::missing_doc_code_examples)]

//! An audio engine designed to support a DAW (digital audio workstation).
//!
//! ```
//! use ensnare::{midi::prelude::*, prelude::*, traits::prelude::*};
//! use groove::mini::{Note, Orchestrator};
//! use groove_entities::{effects::Compressor, effects::CompressorParams};
//! use groove_toys::{ToySynth, ToySynthParams};
//! use std::path::PathBuf;
//!
//! const BPM: f64 = 128.0;
//! const MIDI_0: MidiChannel = MidiChannel(0);
//!
//! // The system needs a working buffer for audio.
//! let mut buffer = [StereoSample::SILENCE; 64];
//!
//! // ToySynth is a MIDI instrument that makes simple sounds.
//! let mut synth = ToySynth::new_with(&ToySynthParams::default());
//! synth.set_uid(Uid(2001));
//!
//! // An effect takes the edge off the synth.
//! let mut compressor = Compressor::new_with(&CompressorParams {
//!     threshold: Normal::from(0.8),
//!     ratio: 0.5,
//!     attack: 0.05,
//!     release: 0.1,
//! });
//! compressor.set_uid(Uid(2002));
//!
//! // Orchestrator understands the relationships among the instruments,
//! // controllers, and effects, and uses them to produce a song.
//! let mut orchestrator = Orchestrator::default();
//!
//! // Orchestrator owns the sample rate and propagates it to the devices
//! // that it controls.
//! orchestrator.update_sample_rate(SampleRate::DEFAULT);
//!
//! // An Orchestrator manages a set of Tracks, which are what actually contains
//! // musical devices.
//! let track_uid = orchestrator.new_midi_track().unwrap();
//! let track = orchestrator.get_track_mut(&track_uid).unwrap();
//!
//! // The sequencer sends MIDI commands to the synth. Each MIDI track
//! // automatically includes one. There are lots of different ways to populate
//! // the sequencer with notes.
//! let mut sequencer = track.sequencer_mut();
//!
//! // TODO - not working yet!
//! // sequencer.append_note(&Note::new_with_midi_note(
//! //     MidiNote::A4,
//! //     MusicalTime::START,
//! //     MusicalTime::DURATION_QUARTER,
//! // ));
//!
//! // Adding an entity to a track forms a chain that sends MIDI, control, and
//! // audio data appropriately.
//! let synth_id = track.append_entity(Box::new(synth)).unwrap();
//! let compressor_id = track.append_entity(Box::new(compressor)).unwrap();
//!
//! // Once everything is set up, the orchestrator renders an audio stream.
//! let _ = orchestrator.write_to_file(&PathBuf::from("output.wav"));
//! ```

// #[deprecated]
// pub use groove_orchestration::EntityObsolete;

pub use mini::EntityFactory;
pub use mini::Orchestrator;

/// Widgets for egui
pub mod panels;

/// Temp home for minidaw research results
pub mod mini;

/// Recommended imports for first-time users.
pub mod prelude {
    pub use super::mini::Orchestrator;
    pub use ensnare::core::StereoSample;
}

// TODO: these should be #[cfg(test)] because nobody should be assuming these
// values

#[doc(hidden)]
/// A typical sample rate.
pub const DEFAULT_SAMPLE_RATE: usize = 44100;
#[doc(hidden)]
/// A typical BPM (beats per minute) for EDM.
pub const DEFAULT_BPM: ensnare::core::ParameterType = 128.0;
#[doc(hidden)]
/// The most common time signature
pub const DEFAULT_TIME_SIGNATURE: (usize, usize) = (4, 4);
#[doc(hidden)]
/// A typical tick-per-second rate for a MIDI file.
pub const DEFAULT_MIDI_TICKS_PER_SECOND: usize = 960;

// https://stackoverflow.com/a/65972328/344467
/// A string that's useful for displaying build information to end users.
pub fn app_version() -> &'static str {
    option_env!("GIT_DESCRIBE")
        .unwrap_or(option_env!("GIT_REV_PARSE").unwrap_or(env!("CARGO_PKG_VERSION")))
}
