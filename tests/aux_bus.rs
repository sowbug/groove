// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove::{
    mini::{register_factory_entities, Key, OrchestratorBuilder, PatternBuilder},
    EntityFactory,
};
use groove_core::Normal;
use std::path::PathBuf;

// Demonstrates use of aux buses.
#[test]
fn aux_bus() {
    let mut orchestrator = OrchestratorBuilder::default()
        .title(Some("Auxiliary Buses".to_string()))
        .build()
        .unwrap();
    let factory = register_factory_entities(EntityFactory::default());

    let track_uid_1 = orchestrator.new_midi_track().unwrap();
    let track_uid_2 = orchestrator.new_midi_track().unwrap();
    let aux_track_uid = orchestrator.new_aux_track().unwrap();

    let synth_pattern_uid_1 = {
        let mut piano_roll = orchestrator.piano_roll_mut();
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

    let synth_pattern_uid_2 = {
        let mut piano_roll = orchestrator.piano_roll_mut();
        piano_roll.insert(
            PatternBuilder::default()
                .note_sequence(
                    vec![
                        84, 255, 83, 255, 81, 255, 79, 255, 77, 255, 76, 255, 74, 255, 72, 255,
                    ],
                    None,
                )
                .build()
                .unwrap(),
        )
    };

    let _synth_uid_1 = {
        let track = orchestrator.get_track_mut(&track_uid_1).unwrap();
        let sequencer = track.sequencer_mut().unwrap();
        let _ = sequencer.arrange_pattern(&synth_pattern_uid_1, 0);

        // Even though we want the effect to be placed after the instrument in
        // the audio chain, we can add the effect before we add the instrument.
        // This is because the processing order is always controllers,
        // instruments, effects.
        track
            .append_thing(factory.new_thing(&Key::from("gain")).unwrap())
            .unwrap();
        track
            .append_thing(factory.new_thing(&Key::from("welsh-synth")).unwrap())
            .unwrap()
    };
    let _synth_uid_2 = {
        let track = orchestrator.get_track_mut(&track_uid_2).unwrap();
        let sequencer = track.sequencer_mut().unwrap();
        let _ = sequencer.arrange_pattern(&synth_pattern_uid_2, 0);
        track
            .append_thing(factory.new_thing(&Key::from("gain")).unwrap())
            .unwrap();
        track
            .append_thing(factory.new_thing(&Key::from("toy-synth")).unwrap())
            .unwrap()
    };
    let _effect_uid_1 = {
        let track = orchestrator.get_track_mut(&aux_track_uid).unwrap();
        track
            .append_thing(factory.new_thing(&Key::from("gain")).unwrap())
            .unwrap();
        track
            .append_thing(factory.new_thing(&Key::from("reverb")).unwrap())
            .unwrap()
    };

    let _ = orchestrator.send_to_aux(track_uid_1, aux_track_uid, Normal::from(1.0));
    let _ = orchestrator.send_to_aux(track_uid_2, aux_track_uid, Normal::from(1.0));

    // https://doc.rust-lang.org/std/path/struct.PathBuf.html example
    let output_path: PathBuf = [env!("CARGO_TARGET_TMPDIR"), "aux-bus.wav"]
        .iter()
        .collect();
    assert!(orchestrator.write_to_file(&output_path).is_ok());
}
