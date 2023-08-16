// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove::{
    mini::{register_mini_factory_entities, Key, OrchestratorBuilder, PatternBuilder, PatternUid},
    EntityFactory, Orchestrator,
};
use groove_core::{time::Tempo, traits::Configurable, Normal};
use std::path::PathBuf;

fn set_up_patterns(o: &mut Orchestrator) -> Vec<PatternUid> {
    let mut piano_roll = o.piano_roll_mut();

    let drum_pattern = PatternBuilder::default()
        .note_sequence(
            vec![
                35, 255, 255, 255, 35, 255, 255, 255, 35, 255, 255, 255, 35, 255, 255, 255,
            ],
            None,
        )
        .note_sequence(
            vec![
                255, 255, 255, 255, 39, 255, 255, 255, 255, 255, 255, 255, 39, 255, 255, 255,
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
        .unwrap();
    let drum_pattern_uid = piano_roll.insert(drum_pattern);

    let scale_pattern = PatternBuilder::default()
        .note_sequence(
            vec![
                60, 255, 62, 255, 64, 255, 65, 255, 67, 255, 69, 255, 71, 255, 72, 255,
            ],
            None,
        )
        .build()
        .unwrap();
    let scale_pattern_uid = piano_roll.insert(scale_pattern);

    vec![drum_pattern_uid, scale_pattern_uid]
}

fn set_up_drum_track(o: &mut Orchestrator, factory: &EntityFactory, kick_pattern: PatternUid) {
    let track_uid = o.new_midi_track().unwrap();
    let track = o.get_track_mut(&track_uid).unwrap();
    let sequencer = track.sequencer_mut().unwrap();
    let _ = sequencer.arrange_pattern(&kick_pattern, 0);

    let _drumkit_uid = track
        .append_thing(factory.new_thing(&Key::from("drumkit")).unwrap())
        .unwrap();

    let effect_uid = track
        .append_thing(factory.new_thing(&Key::from("reverb")).unwrap())
        .unwrap();
    let _ = track.set_humidity(effect_uid, Normal::from(0.2));
}

#[test]
fn drum_beat() {
    let mut orchestrator = OrchestratorBuilder::default()
        .title(Some("Drum Beat".to_string()))
        .build()
        .unwrap();
    orchestrator.update_tempo(Tempo(128.0));

    let mut factory = EntityFactory::default();
    register_mini_factory_entities(&mut factory);
    let factory = factory;

    let pattern_uids = set_up_patterns(&mut orchestrator);
    set_up_drum_track(&mut orchestrator, &factory, pattern_uids[0]);

    // https://doc.rust-lang.org/std/path/struct.PathBuf.html example
    let output_path: PathBuf = [env!("CARGO_TARGET_TMPDIR"), "drum-beat.wav"]
        .iter()
        .collect();
    assert!(orchestrator.write_to_file(&output_path).is_ok());
}
