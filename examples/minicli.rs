// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! The `minicli` example shows how to generate sound files from a serialized
//! [Orchestrator].

use clap::Parser;
use groove::mini::Orchestrator;
use regex::Regex;
use std::{fs::File, io::BufReader, path::PathBuf};

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
                        if let Err(e) = o.write_to_file(&output_path) {
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
