use anyhow::Ok;
use clap::Parser;
use groove::{Clock, GrooveRunner, IOHelper, Orchestrator};
use std::time::Instant;
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

    /// Whether to run the current debug/dev experiment
    #[clap(short, long, value_parser)]
    experiment: bool,
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
            let start_instant = Instant::now();
            let r = IOHelper::song_settings_from_yaml_file(args.yaml_in.unwrap().as_str())?
                .instantiate(args.experiment)?;
            println!(
                "Orchestrator instantiation time: {:.2?}",
                start_instant.elapsed()
            );
            r
        } else {
            Box::new(Orchestrator::default())
        };

        orchestrator.set_enable_dev_experiment(args.experiment);

        print!("Performing to queue ");
        let mut r = GrooveRunner::default();
        let mut clock = Clock::new_with(orchestrator.clock_settings());
        let start_instant = Instant::now();
        let performance = r.run_performance(&mut orchestrator, &mut clock)?;
        println!(
            "\n Orchestrator performance time: {:.2?}",
            start_instant.elapsed()
        );
        println!(" Sample count: {:?}", performance.worker.len());
        println!(
            " Samples per msec: {:.2?} (goal >{:.2?})",
            performance.worker.len() as f32 / start_instant.elapsed().as_millis() as f32,
            performance.sample_rate as f32 / 1000.0
        );
        println!(
            " usec per sample: {:.2?} (goal <{:.2?})",
            start_instant.elapsed().as_micros() as f32 / performance.worker.len() as f32,
            1000000.0 / performance.sample_rate as f32
        );

        println!("Rendering queue");
        if let Some(output_filename) = args.wav_out {
            IOHelper::send_performance_to_file(performance, &output_filename)
        } else {
            IOHelper::send_performance_to_output_device(performance)
        }
    }
}
