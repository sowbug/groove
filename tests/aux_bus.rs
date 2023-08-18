// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove::{
    mini::{register_mini_factory_entities, Key, OrchestratorBuilder},
    EntityFactory,
};
use std::path::PathBuf;

// Demonstrates use of aux buses.
#[test]
fn aux_bus() {
    let mut orchestrator = OrchestratorBuilder::default()
        .title(Some("Auxiliary Buses".to_string()))
        .build()
        .unwrap();
    let factory = register_mini_factory_entities(EntityFactory::default());

    let track_uid_1 = orchestrator.new_midi_track().unwrap();
    let track_uid_2 = orchestrator.new_midi_track().unwrap();
    let aux_track_uid = orchestrator.new_aux_track().unwrap();

    let synth_uid_1 = {
        let track = orchestrator.get_track_mut(&track_uid_1).unwrap();
        track
            .append_thing(factory.new_thing(&Key::from("welsh-synth")).unwrap())
            .unwrap()
    };
    let synth_uid_2 = {
        let track = orchestrator.get_track_mut(&track_uid_2).unwrap();
        track
            .append_thing(factory.new_thing(&Key::from("toy-synth")).unwrap())
            .unwrap()
    };
    let effect_uid_1 = {
        let track = orchestrator.get_track_mut(&aux_track_uid).unwrap();
        track
            .append_thing(factory.new_thing(&Key::from("reverb")).unwrap())
            .unwrap()
    };

    // let _ = orchestrator.send_to_aux(&track_uid_1, &aux_track_uid, Normal::from(1.0));
    // let _ = orchestrator.send_to_aux(&track_uid_2, &aux_track_uid, Normal::from(1.0));

    // https://doc.rust-lang.org/std/path/struct.PathBuf.html example
    let output_path: PathBuf = [env!("CARGO_TARGET_TMPDIR"), "aux-bus.wav"]
        .iter()
        .collect();
    assert!(orchestrator.write_to_file(&output_path).is_ok());
}
