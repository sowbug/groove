// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove::{
    mini::{register_factory_entities, Key, Note, OrchestratorBuilder, PatternBuilder},
    EntityFactory,
};
use groove_core::{midi::MidiNote, time::MusicalTime};
use std::path::PathBuf;

// Demonstrates sidechaining (which could be considered a kind of automation,
// but it's important enough to put top-level and make sure it's a good
// experience and not merely possible).
#[test]
fn demo_sidechaining() {
    let mut orchestrator = OrchestratorBuilder::default()
        .title(Some("Sidechaining".to_string()))
        .build()
        .unwrap();

    let factory = register_factory_entities(EntityFactory::default());

    // Add the sidechain source track.
    let sidechain_pattern_uid = {
        let mut piano_roll = orchestrator.piano_roll_mut();
        piano_roll.insert(
            PatternBuilder::default()
                .note_sequence(
                    vec![
                        35, 255, 255, 255, 35, 255, 255, 255, 35, 255, 255, 255, 35, 255, 255, 255,
                    ],
                    None,
                )
                .build()
                .unwrap(),
        )
    };
    let sidechain_track_uid = orchestrator.new_midi_track().unwrap();
    let track = orchestrator.get_track_mut(&sidechain_track_uid).unwrap();
    let _ = track
        .sequencer_mut()
        .arrange_pattern(&sidechain_pattern_uid, 0);
    let drumkit_uid = track
        .append_thing(factory.new_thing(&Key::from("drumkit")).unwrap())
        .unwrap();
    // This turns the chain's audio output into Control events.
    let signal_passthrough_uid = track
        .append_thing(
            factory
                .new_thing(&Key::from("signal-amplitude-inverted-passthrough"))
                .unwrap(),
        )
        .unwrap();
    // In this demo, we don't want to hear the kick track.
    let mute_uid = track
        .append_thing(factory.new_thing(&Key::from("mute")).unwrap())
        .unwrap();

    // Add the lead track that we want to duck.
    let lead_pattern_uid = {
        let mut piano_roll = orchestrator.piano_roll_mut();
        piano_roll.insert(
            PatternBuilder::default()
                .note(Note {
                    key: MidiNote::C4 as u8,
                    range: MusicalTime::START..MusicalTime::new_with_beats(4),
                })
                .build()
                .unwrap(),
        )
    };
    let lead_track_uid = orchestrator.new_midi_track().unwrap();
    let track = orchestrator.get_track_mut(&lead_track_uid).unwrap();
    let _ = track.sequencer_mut().arrange_pattern(&lead_pattern_uid, 0);
    let synth_uid = track
        .append_thing(factory.new_thing(&Key::from("toy-synth")).unwrap())
        .unwrap();
    let gain_uid = track
        .append_thing(factory.new_thing(&Key::from("gain")).unwrap())
        .unwrap();

    let gain_ceiling_param_index = {
        factory
            .new_thing(&Key::from("gain"))
            .unwrap()
            .as_controllable()
            .unwrap()
            .control_index_for_name("ceiling")
            .unwrap()
    };

    // Link the sidechain control to the synth's gain. Note that the track with
    // the controllable device, not the track with the controlling device, is
    // the right one to contain the link.
    let track = orchestrator.get_track_mut(&lead_track_uid).unwrap();
    track.control_router_mut().link_control(
        signal_passthrough_uid,
        gain_uid,
        gain_ceiling_param_index,
    );

    // https://doc.rust-lang.org/std/path/struct.PathBuf.html example
    let output_path: PathBuf = [env!("CARGO_TARGET_TMPDIR"), "sidechaining.wav"]
        .iter()
        .collect();
    assert!(orchestrator.write_to_file(&output_path).is_ok());
}
