// Copyright (c) 2023 Mike Tsao. All rights reserved.

use ensnare::core::FrequencyHz;
use groove::{
    mini::{
        register_factory_entities, ControlStepBuilder, ControlTripBuilder, ControlTripPath, Key,
        OrchestratorBuilder, PatternBuilder,
    },
    EntityFactory,
};
use groove_core::{
    control::ControlValue,
    generators::Waveform,
    time::{MusicalTime, Tempo},
    traits::{Configurable, HasUid},
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

    let factory = register_factory_entities(EntityFactory::default());

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
    let _ = track.sequencer_mut().arrange_pattern(&scale_pattern_uid, 0);

    // Add a synth to play the pattern.
    let synth_uid = track
        .append_entity(factory.new_entity(&Key::from("toy-synth")).unwrap())
        .unwrap();

    // Add an LFO that will control a synth parameter.
    let lfo_uid = {
        let mut lfo = Box::new(LfoController::new_with(&LfoControllerParams {
            frequency: FrequencyHz(2.0),
            waveform: Waveform::Sine,
        }));
        lfo.set_uid(factory.mint_uid());
        track.append_entity(lfo).unwrap()
    };

    let pan_param_index = {
        // This would have been a little easier if Orchestrator or Track had a
        // way to query param names, but I'm not sure how often that will
        // happen.
        factory
            .new_entity(&Key::from("toy-synth"))
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

#[test]
fn demo_control_trips() {
    let mut orchestrator = OrchestratorBuilder::default()
        .title(Some("Automation".to_string()))
        .build()
        .unwrap();
    orchestrator.update_tempo(Tempo(128.0));

    let factory = register_factory_entities(EntityFactory::default());

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
    let _ = track.sequencer_mut().arrange_pattern(&scale_pattern_uid, 0);

    // Add a synth to play the pattern.
    let synth_uid = track
        .append_entity(factory.new_entity(&Key::from("toy-synth")).unwrap())
        .unwrap();

    // Figure how out to identify the parameter we want to control.
    let pan_param_index = {
        factory
            .new_entity(&Key::from("toy-synth"))
            .unwrap()
            .as_controllable()
            .unwrap()
            .control_index_for_name("dca-pan")
            .unwrap()
    };

    // Add a ControlTrip that ramps from zero to max over the desired amount of time.
    let control_atlas = track.control_atlas_mut();
    let mut trip = ControlTripBuilder::default()
        .step(
            ControlStepBuilder::default()
                .value(ControlValue::MIN)
                .time(MusicalTime::START)
                .path(ControlTripPath::Linear)
                .build()
                .unwrap(),
        )
        .step(
            ControlStepBuilder::default()
                .value(ControlValue::MAX)
                .time(MusicalTime::new_with_beats(4))
                .path(ControlTripPath::Flat)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let trip_uid = factory.mint_uid();
    trip.set_uid(trip_uid);
    control_atlas.add_trip(trip);

    // Hook up that ControlTrip to the pan parameter.
    track
        .control_router_mut()
        .link_control(trip_uid, synth_uid, pan_param_index);

    // https://doc.rust-lang.org/std/path/struct.PathBuf.html example
    let output_path: PathBuf = [env!("CARGO_TARGET_TMPDIR"), "control-trips.wav"]
        .iter()
        .collect();
    assert!(orchestrator.write_to_file(&output_path).is_ok());
}
