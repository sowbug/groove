// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! This example shows how to use the public API to create a song with Rust
//! code.

use groove::mini::{Orchestrator, OrchestratorBuilder};
use groove_core::{traits::Controls, StereoSample};
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
