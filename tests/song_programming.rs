// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove::{
    mini::{register_mini_factory_entities, Key, OrchestratorBuilder, PatternBuilder},
    EntityFactory, Orchestrator,
};
use groove_core::{time::Tempo, traits::Configurable, Normal};
use std::path::PathBuf;

fn set_up_drum_track(o: &mut Orchestrator, factory: &EntityFactory) {
    // Add the drum pattern to the PianoRoll.
    // We need to scope piano_roll to satisfy the borrow checker.
    let drum_pattern_uid = {
        let mut piano_roll = o.piano_roll_mut();
        piano_roll.insert(
            PatternBuilder::default()
                .note_sequence(
                    vec![
                        35, 255, 255, 255, 35, 255, 255, 255, 35, 255, 255, 255, 35, 255, 255, 255,
                    ],
                    None,
                )
                .note_sequence(
                    vec![
                        255, 255, 255, 255, 39, 255, 255, 255, 255, 255, 255, 255, 39, 255, 255,
                        255,
                    ],
                    None,
                )
                .note_sequence(
                    vec![
                        // Bug: if we do note on every 16th, we get only the first one
                        42, 255, 42, 255, 42, 255, 42, 255, 42, 255, 42, 255, 42, 255, 42, 255,
                    ],
                    None,
                )
                .build()
                .unwrap(),
        )
    };

    // Arrange the drum pattern in a new MIDI track's Sequencer. By default, the
    // Sequencer emits events on MIDI channel 0.
    let track_uid = o.new_midi_track().unwrap();
    let track = o.get_track_mut(&track_uid).unwrap();
    let sequencer = track.sequencer_mut().unwrap();
    let _ = sequencer.arrange_pattern(&drum_pattern_uid, 0);

    // Add the drumkit instrument to the track. By default, it listens on MIDI channel 0.
    let _drumkit_uid = track
        .append_thing(factory.new_thing(&Key::from("drumkit")).unwrap())
        .unwrap();

    // Add an effect to the track's effect chain.
    let filter_uid = track
        .append_thing(
            factory
                .new_thing(&Key::from("filter-low-pass-24db"))
                .unwrap(),
        )
        .unwrap();
    let _ = track.set_humidity(filter_uid, Normal::from(0.2));
}

fn set_up_lead_track(o: &mut Orchestrator, factory: &EntityFactory) {
    // Add the lead pattern to the PianoRoll.
    let scale_pattern_uid = {
        let mut piano_roll = o.piano_roll_mut();
        piano_roll.insert(
            PatternBuilder::default()
                .note_sequence(
                    vec![
                        60, 255, 62, 255, 64, 255, 65, 255, 67, 255, 69, 255, 71, 255, 72, 255,
                    ],
                    None,
                )
                .build()
                .unwrap(),
        )
    };

    // Arrange the lead pattern in a new MIDI track's Sequencer.
    let track_uid = o.new_midi_track().unwrap();
    let track = o.get_track_mut(&track_uid).unwrap();
    let sequencer = track.sequencer_mut().unwrap();
    let _ = sequencer.arrange_pattern(&scale_pattern_uid, 0);

    // Add a synth to play the pattern.
    let _synth_uid = track
        .append_thing(factory.new_thing(&Key::from("welsh-synth")).unwrap())
        .unwrap();

    // Make the synth sound better.
    let reverb_uid = track
        .append_thing(factory.new_thing(&Key::from("reverb")).unwrap())
        .unwrap();
    let _ = track.set_humidity(reverb_uid, Normal::from(0.2));
}

// Demonstrates making a song in Rust. We assume that we knew what the song is
// from the start, so there is no editing -- just programming. Compare the
// edit_song() test, which demonstrates adding elements, changing them, and
// removing them, as you'd expect a GUI DAW to do.
#[test]
fn program_song() {
    let mut orchestrator = OrchestratorBuilder::default()
        .title(Some("Simple Song".to_string()))
        .build()
        .unwrap();
    orchestrator.update_tempo(Tempo(128.0));
    let factory = register_mini_factory_entities(EntityFactory::default());

    set_up_drum_track(&mut orchestrator, &factory);
    set_up_lead_track(&mut orchestrator, &factory);

    // https://doc.rust-lang.org/std/path/struct.PathBuf.html example
    let output_path: PathBuf = [env!("CARGO_TARGET_TMPDIR"), "simple-song.wav"]
        .iter()
        .collect();
    assert!(orchestrator.write_to_file(&output_path).is_ok());
}
