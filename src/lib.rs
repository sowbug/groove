// Copyright (c) 2023 Mike Tsao. All rights reserved.

#![allow(clippy::box_default)]
#![warn(missing_docs)]
#![warn(rustdoc::missing_doc_code_examples)]

//! An audio engine designed to support a DAW (digital audio workstation).
//!
//! ```
//! # use groove::{Entity, Orchestrator};
//! # use groove_core::{
//! #     generators::{EnvelopeParams, Waveform},
//! #     midi::{MidiChannel, new_note_off, new_note_on},
//! #     time::{
//! #         Clock,
//! #         ClockParams,
//! #         MusicalTime,
//! #         SampleRate,
//! #         TimeSignature,
//! #         TimeSignatureParams
//! #     },
//! #     traits::Configurable,
//! #     Normal,
//! #     SAMPLE_BUFFER_SIZE,
//! #     StereoSample,
//! # };
//! # use groove_entities::{
//! #     controllers::Sequencer,
//! #     controllers::SequencerParams,
//! #     effects::Compressor,
//! #     effects::CompressorParams,
//! # };
//! # use groove_toys::{ToySynth, ToySynthParams};
//! #
//! # const BPM: f64 = 128.0;
//! # const MIDI_0: MidiChannel = MidiChannel(0);
//! #
//! // The system needs a working buffer for audio.
//! let mut buffer = [StereoSample::SILENCE; SAMPLE_BUFFER_SIZE];
//!
//! // ToySynth is a MIDI instrument that makes simple sounds.
//! let synth = ToySynth::new_with(&ToySynthParams::default());
//!
//! // Sequencer sends MIDI commands to the synth.
//! let mut sequencer = Sequencer::new_with(&SequencerParams { bpm: 128.0 });
//!
//! // There are lots of different ways to populate the sequencer with notes.
//! let ts = TimeSignature::default();
//! sequencer.insert(&MusicalTime::new(&ts, 0, 0, 0, 0), MIDI_0, new_note_on(69, 100));
//! sequencer.insert(&MusicalTime::new(&ts, 0, 1, 0, 0), MIDI_0, new_note_off(69, 100));
//!
//! // An effect takes the edge off the synth.
//! let compressor = Compressor::new_with(&CompressorParams {
//!     threshold: Normal::from(0.8),
//!     ratio: 0.5,
//!     attack: 0.05,
//!     release: 0.1,
//! });
//!
//! // Orchestrator understands the relationships among the
//! // instruments, controllers, and effects, and uses them to
//! // produce a song.
//! let mut orchestrator = Orchestrator::new_with(&ClockParams {
//!     bpm: 128.0,
//!     midi_ticks_per_second: 960,
//!     time_signature: TimeSignatureParams { top: 4, bottom: 4 },
//! });
//!
//! // Orchestrator owns the sample rate and propagates it to the devices
//! // that it controls.
//! orchestrator.update_sample_rate(SampleRate::DEFAULT);
//!
//! // Each "entity" has an ID that is used to connect them.
//! let synth_id = orchestrator.add(Entity::ToySynth(Box::new(synth)));
//! let _sequencer_id = orchestrator.add(Entity::Sequencer(Box::new(sequencer)));
//! let compressor_id = orchestrator.add(Entity::Compressor(Box::new(compressor)));
//!
//! // The synth's output goes to the compressor's input, and then the
//! // compressor's output goes to the main mixer.
//! assert!(orchestrator.patch_chain_to_main_mixer(&[synth_id, compressor_id]).is_ok());
//!
//! // Virtual MIDI cables let devices send messages to other devices.
//! orchestrator.connect_midi_downstream(synth_id, MIDI_0);
//!
//! // Once everything is set up, the orchestrator renders an audio stream.
//! if let Ok(samples) = orchestrator.run(&mut buffer) {
//!     println!("Created a stream of {} samples.", samples.len());
//!     assert!(samples
//!         .iter()
//!         .any(|sample| *sample != StereoSample::SILENCE));
//!
//!     // not shown: writing stream to WAV file
//! }
//! ```

pub use groove_orchestration::{Entity, Orchestrator};

/// Widgets for egui
pub mod egui_widgets;

/// Temp home for minidaw research results
pub mod mini;

use groove_core::ParameterType;

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

// https://stackoverflow.com/a/65972328/344467
/// A string that's useful for displaying build information to end users.
pub fn app_version() -> &'static str {
    option_env!("GIT_DESCRIBE")
        .unwrap_or(option_env!("GIT_REV_PARSE").unwrap_or(env!("CARGO_PKG_VERSION")))
}
