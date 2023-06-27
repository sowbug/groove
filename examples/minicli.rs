// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove::mini::MiniOrchestrator;
use groove_core::StereoSample;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let mut o = MiniOrchestrator::default();

    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: o.sample_rate().value() as u32,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let path = PathBuf::from("minicli.wav");
    let mut writer = hound::WavWriter::create(path, spec).unwrap();

    let mut buffer = [StereoSample::SILENCE; 64];
    o.debug_sample_buffer(&mut buffer);
    loop {
        if o.is_performing() {
            o.generate_next_samples(&mut buffer);
            for sample in buffer {
                let (left, right) = sample.into_i16();
                let _ = writer.write_sample(left);
                let _ = writer.write_sample(right);
            }
        } else {
            break;
        }
    }

    Ok(())
}
