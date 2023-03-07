// Copyright (c) 2023 Mike Tsao. All rights reserved.

#![allow(clippy::box_default)]

//! A DAW (digital audio workstation) engine.
//!
//! ```
//! use groove::{Entity, Orchestrator};
//! use groove_core::{
//!     midi::{MidiChannel, new_note_off, new_note_on},
//!     time::PerfectTimeUnit,
//!     StereoSample,
//! };
//!
//! use groove_entities::{controllers::BeatSequencer, effects::Compressor};
//! use groove_toys::ToySynth;
//!
//! const SAMPLE_RATE: usize = 44100;
//! const BPM: f64 = 128.0;
//! const MIDI_0: MidiChannel = 0;
//!
//! // Provide a working buffer for audio.
//! let mut buffer = [StereoSample::SILENCE; 64];
//!
//! // ToySynth is a MIDI instrument that makes simple sounds.
//! let synth = ToySynth::new_with(SAMPLE_RATE);
//!
//! // Sequencer sends MIDI commands to the synth.
//! let mut sequencer = BeatSequencer::new_with(SAMPLE_RATE, BPM);
//!
//! // There are lots of different ways to populate the sequencer with notes.
//! sequencer.insert(PerfectTimeUnit(0.0), MIDI_0, new_note_on(69, 100));
//! sequencer.insert(PerfectTimeUnit(1.0), MIDI_0, new_note_off(69, 100));
//!
//! // An effect takes the edge off the synth.
//! let compressor = Compressor::new_with(0.8, 0.5, 0.05, 0.1);
//!
//! // Orchestrator understands the relationships among the
//! // instruments, controllers, and effects, and uses them to
//! // produce a song.
//! let mut orchestrator = Orchestrator::new_with(SAMPLE_RATE, BPM);
//!
//! // Tell the orchestrator about everything.
//! let synth_id = orchestrator.add(Entity::ToySynth(Box::new(synth)));
//! let _sequencer_id = orchestrator.add(Entity::BeatSequencer(Box::new(sequencer)));
//! let compressor_id = orchestrator.add(Entity::Compressor(Box::new(compressor)));
//!
//! // Plug in the audio outputs.
//! let _ = orchestrator.patch_chain_to_main_mixer(&[synth_id, compressor_id]);
//!
//! // Connect MIDI.
//! orchestrator.connect_midi_downstream(synth_id, MIDI_0);
//!
//! // Ask the orchestrator to render the performance.
//! if let Ok(samples) = orchestrator.run(&mut buffer) {
//!     println!("{}", samples.len());
//!     assert!(samples
//!         .iter()
//!         .any(|sample| *sample != StereoSample::SILENCE));
//! }
//! ```

pub mod subscriptions;
pub use groove_orchestration::Entity;
pub use groove_orchestration::Orchestrator;

use groove_core::ParameterType;

// TODO: these should be #[cfg(test)] because nobody should be assuming these
// values
pub const DEFAULT_SAMPLE_RATE: usize = 44100;
pub const DEFAULT_BPM: ParameterType = 128.0;
pub const DEFAULT_TIME_SIGNATURE: (usize, usize) = (4, 4);
pub const DEFAULT_MIDI_TICKS_PER_SECOND: usize = 960;

// https://stackoverflow.com/a/65972328/344467
pub fn app_version() -> &'static str {
    option_env!("GIT_DESCRIBE")
        .unwrap_or(option_env!("GIT_REV_PARSE").unwrap_or(env!("CARGO_PKG_VERSION")))
}

#[cfg(test)]
mod tests {
    use groove_core::{util::Paths, StereoSample};
    use groove_orchestration::helpers::IOHelper;
    use groove_settings::SongSettings;
    use std::{fs::File, io::prelude::*, time::Instant};

    #[test]
    fn yaml_loads_and_parses() {
        let mut path = Paths::test_data_path();
        path.push("kitchen-sink.yaml");
        let yaml = std::fs::read_to_string(path)
            .unwrap_or_else(|err| panic!("loading YAML failed: {:?}", err));
        let song_settings = SongSettings::new_from_yaml(yaml.as_str())
            .unwrap_or_else(|err| panic!("parsing settings failed: {:?}", err));
        let mut orchestrator = song_settings
            .instantiate(false)
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
    fn spit_out_perf_data() {
        let mut path = Paths::test_data_path();
        path.push("perf-1.yaml");
        let yaml = std::fs::read_to_string(path)
            .unwrap_or_else(|err| panic!("loading YAML failed: {:?}", err));
        let song_settings = SongSettings::new_from_yaml(yaml.as_str())
            .unwrap_or_else(|err| panic!("parsing settings failed: {:?}", err));
        let mut orchestrator = song_settings
            .instantiate(false)
            .unwrap_or_else(|err| panic!("instantiation failed: {:?}", err));

        let start_instant = Instant::now();
        let mut samples = [StereoSample::SILENCE; 64];
        let performance = orchestrator
            .run_performance(&mut samples, false)
            .unwrap_or_else(|err| panic!("performance failed: {:?}", err));
        let elapsed = start_instant.elapsed();
        let frame_count = performance.worker.len();

        let mut file = File::create("perf-output.txt").unwrap();
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

        let mut path = Paths::out_path();
        path.push("perf-1.wav");
        assert!(IOHelper::send_performance_to_file(&performance, &path).is_ok());
    }

    #[test]
    fn test_patching_to_device_with_no_input_fails_with_proper_error() {
        let mut path = Paths::test_data_path();
        path.push("instruments-have-no-inputs.yaml");
        let yaml = std::fs::read_to_string(path)
            .unwrap_or_else(|err| panic!("loading YAML failed: {:?}", err));
        let song_settings = SongSettings::new_from_yaml(yaml.as_str())
            .unwrap_or_else(|err| panic!("parsing settings failed: {:?}", err));
        let r = song_settings.instantiate(false);
        assert_eq!(
            r.unwrap_err().to_string(),
            "Input device doesn't transform audio and can't be patched from output device"
        );
    }
}
