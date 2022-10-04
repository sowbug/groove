#![feature(trait_upcasting)]
#![allow(incomplete_features)]

use libgroove::{
    devices::orchestrator::Orchestrator,
    helpers::{self, IOHelper},
};

use anyhow::Ok;
use clap::Parser;

#[derive(Parser, Debug, Default)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// MIDI filename
    #[clap(short, long, value_parser)]
    midi_in: Option<String>,

    /// Script to execute
    #[clap(short, long, value_parser)]
    script_in: Option<String>,

    /// YAML to execute
    #[clap(short, long, value_parser)]
    yaml_in: Option<String>,

    /// Whether to use an external MIDI controller
    #[clap(short, long, parse(from_flag))]
    use_midi_controller: bool,

    /// Output filename
    #[clap(short, long, value_parser)]
    wav_out: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.script_in.is_some() {
        //      ScriptEngine::new().execute_file(&args.script_in.unwrap())
        Ok(())
    } else {
        let mut orchestrator = if args.midi_in.is_some() {
            IOHelper::orchestrator_from_midi_file(args.midi_in.unwrap())
        } else if args.yaml_in.is_some() {
            IOHelper::orchestrator_from_yaml_file(args.yaml_in.unwrap())
        } else {
            Orchestrator::new_defaults()
        };

        print!("Performing to queue ");
        let performance = orchestrator.perform()?;

        println!("Rendering queue");
        if let Some(output_filename) = args.wav_out {
            helpers::IOHelper::send_performance_to_file(performance, &output_filename)
        } else {
            helpers::IOHelper::send_performance_to_output_device(performance)
        }
    }
}
