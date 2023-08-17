// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove::{
    mini::{register_mini_factory_entities, Key, OrchestratorBuilder, PatternBuilder},
    EntityFactory,
};
use groove_core::{
    generators::Waveform,
    time::Tempo,
    traits::{Configurable, HasUid},
    FrequencyHz,
};
use groove_entities::controllers::{LfoController, LfoControllerParams};
use std::path::PathBuf;

// Demonstrates the control (automation) system.
#[test]
fn demo_automation() {
    let mut orchestrator = OrchestratorBuilder::default()
        .title(Some("Automation".to_string()))
        .build()
        .unwrap();
    orchestrator.update_tempo(Tempo(128.0));

    let mut factory = EntityFactory::default();
    register_mini_factory_entities(&mut factory);
    let factory = factory;

    // Add the lead pattern to the PianoRoll.
    let scale_pattern_uid = {
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

    // Arrange the lead pattern in a new MIDI track's Sequencer.
    let track_uid = orchestrator.new_midi_track().unwrap();
    let track = orchestrator.get_track_mut(&track_uid).unwrap();
    let sequencer = track.sequencer_mut().unwrap();
    let _ = sequencer.arrange_pattern(&scale_pattern_uid, 0);

    // Add a synth to play the pattern.
    let synth_uid = track
        .append_thing(factory.new_thing(&Key::from("toy-synth")).unwrap())
        .unwrap();

    // Add an LFO that will control a synth parameter.
    let lfo_uid = {
        let mut lfo = Box::new(LfoController::new_with(&LfoControllerParams {
            frequency: FrequencyHz(2.0),
            waveform: Waveform::Sine,
        }));
        lfo.set_uid(factory.mint_uid());
        track.append_thing(lfo).unwrap()
    };

    let pan_param_index = {
        // This would have been a little easier if Orchestrator or Track had a
        // way to query param names, but I'm not sure how often that will
        // happen.
        factory
            .new_thing(&Key::from("toy-synth"))
            .unwrap()
            .as_controllable()
            .unwrap()
            .control_index_for_name("dca-pan")
            .unwrap()
    };

    // Link the LFO to the synth's pan.
    track
        .control_router_mut()
        .link_control(lfo_uid, synth_uid, pan_param_index);

    // https://doc.rust-lang.org/std/path/struct.PathBuf.html example
    let output_path: PathBuf = [env!("CARGO_TARGET_TMPDIR"), "automation.wav"]
        .iter()
        .collect();
    assert!(orchestrator.write_to_file(&output_path).is_ok());
}
