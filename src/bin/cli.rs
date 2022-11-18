use anyhow::Ok;
use clap::Parser;
use groove::{IOHelper, Orchestrator};
//use groove::ScriptEngine;

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
        //let _r = ScriptEngine::new().execute_file(&args.script_in.unwrap());
        Ok(())
    } else {
        let mut orchestrator = if args.midi_in.is_some() {
            IOHelper::orchestrator_from_midi_file(args.midi_in.unwrap().as_str())
        } else if args.yaml_in.is_some() {
            IOHelper::song_settings_from_yaml_file(args.yaml_in.unwrap().as_str())?.instantiate()?
        } else {
            Orchestrator::new()
        };

        print!("Performing to queue ");
        let performance = orchestrator.perform()?;

        println!("Rendering queue");
        if let Some(output_filename) = args.wav_out {
            IOHelper::send_performance_to_file(performance, &output_filename)
        } else {
            IOHelper::send_performance_to_output_device(performance)
        }
    }
}
