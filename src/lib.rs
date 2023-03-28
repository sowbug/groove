// Copyright (c) 2023 Mike Tsao. All rights reserved.

#![allow(clippy::box_default)]
#![warn(missing_docs)]
#![warn(missing_doc_code_examples)]

//! An audio engine designed for making a DAW (digital audio workstation).
//!
//! ```
//! # use groove::{Entity, Orchestrator};
//! # use groove_core::{
//! #     midi::{MidiChannel, new_note_off, new_note_on},
//! #     time::PerfectTimeUnit,
//! #     Normal,
//! #     StereoSample,
//! # };
//! # use groove_entities::{
//! #     controllers::Sequencer,
//! #     controllers::SequencerParams,
//! #     effects::Compressor,
//! #     effects::CompressorParams,
//! # };
//! # use groove_toys::ToySynth;
//! #
//! # const SAMPLE_RATE: usize = 44100;
//! # const BPM: f64 = 128.0;
//! # const MIDI_0: MidiChannel = 0;
//! #
//! // The system needs a working buffer for audio.
//! let mut buffer = [StereoSample::SILENCE; 64];
//!
//! // ToySynth is a MIDI instrument that makes simple sounds.
//! let synth = ToySynth::new_with(SAMPLE_RATE);
//!
//! // Sequencer sends MIDI commands to the synth.
//! let mut sequencer = Sequencer::new_with(SAMPLE_RATE, SequencerParams { bpm: 128.0 });
//!
//! // There are lots of different ways to populate the sequencer with notes.
//! sequencer.insert(PerfectTimeUnit(0.0), MIDI_0, new_note_on(69, 100));
//! sequencer.insert(PerfectTimeUnit(1.0), MIDI_0, new_note_off(69, 100));
//!
//! // An effect takes the edge off the synth.
//! let compressor = Compressor::new_with(CompressorParams {
//!     threshold: Normal::from(0.8),
//!     ratio: 0.5,
//!     attack: 0.05,
//!     release: 0.1,
//! });
//!
//! // Orchestrator understands the relationships among the
//! // instruments, controllers, and effects, and uses them to
//! // produce a song.
//! let mut orchestrator = Orchestrator::new_with(SAMPLE_RATE, BPM);
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

/// Helps send audio to the outside world.
pub mod audio;
/// Contains Iced [Subscriptions](iced_native::subscription::Subscription) for
/// working with this crate.
pub mod subscriptions;
pub use groove_orchestration::{Entity, Orchestrator};
/// Contains path-building utilities.
pub mod util;

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

#[cfg(test)]
mod tests {
    use crate::util::{PathType, Paths};
    use groove_core::{util::tests::TestOnlyPaths, StereoSample};
    use groove_orchestration::helpers::IOHelper;
    use groove_settings::SongSettings;
    use std::{fs::File, io::prelude::*, path::PathBuf, time::Instant};

    #[ignore = "Figure out how to tell Paths to use cwd as the installation directory"]
    #[test]
    fn yaml_loads_and_parses() {
        let yaml = std::fs::read_to_string(PathBuf::from("test-data/kitchen-sink.yaml"))
            .unwrap_or_else(|err| panic!("loading YAML failed: {:?}", err));
        let song_settings = SongSettings::new_from_yaml(yaml.as_str())
            .unwrap_or_else(|err| panic!("parsing settings failed: {:?}", err));
        let mut orchestrator = song_settings
            .instantiate(&Paths::assets_path(PathType::Dev), false)
            .unwrap_or_else(|err| panic!("instantiation failed: {:?}", err));
        let mut sample_buffer = [StereoSample::SILENCE; 64];
        if let Ok(samples) = orchestrator.run(&mut sample_buffer) {
            assert!(
                !samples.is_empty(),
                "Orchestrator reported successful performance, but performance is empty."
            );

            assert!(
                samples
                    .iter()
                    .any(|sample| { *sample != StereoSample::SILENCE }),
                "Performance contains only silence."
            );
        } else {
            panic!("run failed")
        }
    }

    #[test]
    #[ignore = "orchestrator - control_message_for_index is incomplete. re-enable when macroized"]
    fn spit_out_perf_data() {
        let yaml = std::fs::read_to_string(PathBuf::from("test-data/perf-1.yaml"))
            .unwrap_or_else(|err| panic!("loading YAML failed: {:?}", err));
        let song_settings = SongSettings::new_from_yaml(yaml.as_str())
            .unwrap_or_else(|err| panic!("parsing settings failed: {:?}", err));
        let mut orchestrator = song_settings
            .instantiate(&Paths::assets_path(PathType::Dev), false)
            .unwrap_or_else(|err| panic!("instantiation failed: {:?}", err));

        let start_instant = Instant::now();
        let mut samples = [StereoSample::SILENCE; 64];
        let performance = orchestrator
            .run_performance(&mut samples, false)
            .unwrap_or_else(|err| panic!("performance failed: {:?}", err));
        let elapsed = start_instant.elapsed();
        let frame_count = performance.worker.len();

        let mut out_path = TestOnlyPaths::writable_out_path();
        out_path.push("perf-output.txt");
        let mut file = File::create(out_path).unwrap();
        let output = format!(
            "Elapsed    : {:0.3}s\n\
Frames     : {}\n\
Frames/msec: {:.2?} (goal >{:.2?})\n\
usec/frame : {:.2?} (goal <{:.2?})",
            elapsed.as_secs_f32(),
            frame_count,
            frame_count as f32 / start_instant.elapsed().as_millis() as f32,
            performance.sample_rate as f32 / 1000.0,
            start_instant.elapsed().as_micros() as f32 / frame_count as f32,
            1000000.0 / performance.sample_rate as f32
        );
        let _ = file.write(output.as_bytes());

        let mut path = TestOnlyPaths::test_data_path();
        path.push("perf-1.wav");
        assert!(IOHelper::send_performance_to_file(&performance, &path).is_ok());
    }

    #[test]
    fn patching_to_device_with_no_input_fails_with_proper_error() {
        let path = TestOnlyPaths::test_data_path().join("instruments-have-no-inputs.yaml");
        let yaml = std::fs::read_to_string(path)
            .unwrap_or_else(|err| panic!("loading YAML failed: {:?}", err));
        let song_settings = SongSettings::new_from_yaml(yaml.as_str())
            .unwrap_or_else(|err| panic!("parsing settings failed: {:?}", err));
        let r = song_settings.instantiate(&Paths::assets_path(PathType::Dev), false);
        assert_eq!(
            r.unwrap_err().to_string(),
            "Input device doesn't transform audio and can't be patched from output device"
        );
    }
}
