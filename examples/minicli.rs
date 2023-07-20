// Copyright (c) 2023 Mike Tsao. All rights reserved.

use clap::Parser;
use groove::mini::Orchestrator;
use groove_core::{
    traits::{Controls, Performs},
    StereoSample,
};
use regex::Regex;
use std::{fs::File, io::BufReader, path::PathBuf};

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
    orchestrator.render_debug(&mut buffer);

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

#[derive(Parser, Debug, Default)]
#[clap(author, about, long_about = None)]
struct Args {
    /// Names of files to process. Currently accepts JSON-format projects.
    input: Vec<String>,

    /// Render as WAVE file(s) (file will appear next to source file)
    #[clap(short = 'w', long, value_parser)]
    wav: bool,

    /// Enable debug mode
    #[clap(short = 'd', long, value_parser)]
    debug: bool,

    /// Print version and exit
    #[clap(short = 'v', long, value_parser)]
    version: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    for input_filename in args.input {
        match File::open(input_filename.clone()) {
            Ok(f) => match serde_json::from_reader::<_, Orchestrator>(BufReader::new(f)) {
                Ok(mut o) => {
                    if args.wav {
                        let re = Regex::new(r"\.json$").unwrap();
                        let output_filename = re.replace(&input_filename, ".wav");
                        if input_filename == output_filename {
                            panic!("would overwrite input file; couldn't generate output filename");
                        }
                        let output_path = PathBuf::from(output_filename.to_string());
                        if let Err(e) = write_performance_to_file(&mut o, &output_path) {
                            eprintln!(
                                "error while writing {input_filename} render to {}: {e:?}",
                                output_path.display()
                            );
                            return Err(e);
                        }
                    }
                }
                Err(e) => eprintln!("error while parsing {input_filename}: {e:?}"),
            },
            Err(e) => eprintln!("error while opening {input_filename}: {e:?}"),
        }
    }
    Ok(())
}
