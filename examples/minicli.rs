// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove::mini::{MiniOrchestrator, MiniSequencer, MiniSequencerParams, TrackIndex};
use groove_core::{
    midi::MidiChannel,
    traits::{Controls, Performs},
    StereoSample,
};
use std::path::PathBuf;

fn write_performance_to_file(orchestrator: &mut MiniOrchestrator) -> anyhow::Result<()> {
    let spec = hound::WavSpec {
        channels: orchestrator.channels(),
        sample_rate: orchestrator.sample_rate().into(),
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let path = PathBuf::from("minicli.wav");
    let mut writer = hound::WavWriter::create(path, spec).unwrap();

    let mut buffer = [StereoSample::SILENCE; 64];
    orchestrator.debug_sample_buffer(&mut buffer);

    orchestrator.play();
    loop {
        if orchestrator.is_finished() {
            break;
        }
        orchestrator.generate_next_samples(&mut buffer);
        for sample in buffer {
            let (left, right) = sample.into_i16();
            let _ = writer.write_sample(left);
            let _ = writer.write_sample(right);
        }
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let mut o = MiniOrchestrator::default();

    let _uid = o
        .add_controller(
            Box::new(MiniSequencer::new_with(
                &MiniSequencerParams::default(),
                MidiChannel(0),
            )),
            TrackIndex(0),
        )
        .unwrap();

    write_performance_to_file(&mut o)
}
