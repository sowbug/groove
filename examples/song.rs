// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! This example shows how to use the public API to create a song with Rust
//! code.

use groove::{
    mini::{
        register_mini_factory_entities, Key, Orchestrator, OrchestratorBuilder, PatternBuilder,
        PatternUid,
    },
    EntityFactory,
};
use groove_core::{
    time::Tempo,
    traits::{Configurable, Controls},
    StereoSample,
};
use std::path::PathBuf;

// TODO: it would be nice to put this somewhere reusable, but I'm having trouble
// finding the right place without changing lots of dependencies.
fn write_performance_to_file(
    orchestrator: &mut Orchestrator,
    path: &PathBuf,
) -> anyhow::Result<()> {
    let spec = hound::WavSpec {
        channels: orchestrator.channels(),
        sample_rate: orchestrator.sample_rate().into(),
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).unwrap();

    let mut buffer = [StereoSample::SILENCE; 64];
    let mut batches_processed = 0;
    orchestrator.play();
    loop {
        if orchestrator.is_finished() {
            break;
        }
        buffer.fill(StereoSample::SILENCE);
        orchestrator.render(&mut buffer);
        for sample in buffer {
            let (left, right) = sample.into_i16();
            let _ = writer.write_sample(left);
            let _ = writer.write_sample(right);
        }
        batches_processed += 1;
    }

    eprintln!(
        "Processed {batches_processed} batches of {} samples each",
        buffer.len()
    );

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let mut o = OrchestratorBuilder::default()
        .title(Some("My Song".to_string()))
        .build()
        .unwrap();

    // TODO: is this needed after a new?
    //    o.after_deser();

    o.update_tempo(Tempo(128.0));

    set_up_song(&mut o);

    let output_path = PathBuf::from("output.wav");
    if let Err(e) = write_performance_to_file(&mut o, &output_path) {
        eprintln!(
            "error while writing render results to {}: {e:?}",
            output_path.display()
        );
        return Err(e);
    }
    Ok(())
}

fn set_up_kick_track(o: &mut Orchestrator, factory: &EntityFactory, kick_pattern: PatternUid) {
    let track_uid = o.new_midi_track().unwrap();
    let track = o.get_track_mut(&track_uid).unwrap();
    let sequencer = track.sequencer_mut().unwrap();
    let _ = sequencer.arrange_pattern(&kick_pattern, 0);

    let _drumkit_uid = o
        .add_thing(
            factory.new_thing(&Key::from("drumkit")).unwrap(),
            &track_uid,
        )
        .unwrap();
}

fn set_up_song(o: &mut Orchestrator) {
    let mut factory = EntityFactory::default();
    register_mini_factory_entities(&mut factory);
    let factory = factory;

    let pattern_uids = set_up_patterns(o);
    set_up_kick_track(o, &factory, pattern_uids[0]);
}

fn set_up_patterns(o: &mut Orchestrator) -> Vec<PatternUid> {
    let mut piano_roll = o.piano_roll_mut();

    let drum_pattern = PatternBuilder::default()
        //        .note_sequence(vec![255, 255, 255, 255, 60], None)
        // .note_sequence(
        //     vec![
        //         35, 255, 60, 255, 69
        //     ],
        //     None,
        // )
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
