use anyhow::Ok;
use clap::Parser;
use groove::{
    app_version,
    common::{DEFAULT_BPM, DEFAULT_SAMPLE_RATE},
    helpers::IOHelper,
    Orchestrator,
};
use groove_core::StereoSample;
use regex::Regex;
use std::time::Instant;
//use groove::ScriptEngine;

#[derive(Parser, Debug, Default)]
#[clap(author, about, long_about = None)]
struct Args {
    /// Names of files to process. Can be YAML, MIDI, or scripts.
    input: Vec<String>,

    /// Render as WAVE file(s) (file will appear next to source file)
    #[clap(short = 'w', long, value_parser)]
    wav: bool,

    /// Render as MP3 file(s) (not yet implemented)
    #[clap(short = 'm', long, value_parser)]
    mp3: bool,

    /// Enable debug mode
    #[clap(short = 'd', long, value_parser)]
    debug: bool,

    /// Print perf information
    #[clap(short = 'p', long, value_parser)]
    perf: bool,

    /// Suppress status updates while processing
    #[clap(short = 'q', long, value_parser)]
    quiet: bool,

    /// Print version and exit
    #[clap(short = 'v', long, value_parser)]
    version: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // TODO: output this information into any generated files (WAV, MP3, etc.)
    // so that we can reproduce them when the code later changes.
    if args.version {
        println!("groove-cli {}", app_version());
        return Ok(());
    }

    for input_filename in args.input {
        if input_filename == "-" {
            // This is a separator for cases like
            //
            // `cargo run --bin groove-cli - x.yaml`
            continue;
        }
        let mut orchestrator = if input_filename.ends_with(".nscr") {
            #[cfg(feature = "scripting")]
            let _r = ScriptEngine::new().execute_file(&args.script_in.unwrap());

            // TODO: this is temporary, to return the right type
            #[cfg(not(feature = "scripting"))]
            Box::new(Orchestrator::new_with(DEFAULT_SAMPLE_RATE, DEFAULT_BPM))
        } else if input_filename.ends_with(".yaml")
            || input_filename.ends_with(".yml")
            || input_filename.ends_with(".nsn")
        {
            let start_instant = Instant::now();
            let r = Box::new(
                IOHelper::song_settings_from_yaml_file(input_filename.as_str())?
                    .instantiate(args.debug)?,
            );
            if args.perf {
                println!(
                    "Orchestrator instantiation time: {:.2?}",
                    start_instant.elapsed()
                );
            }
            r
        } else {
            Box::new(Orchestrator::new_with(DEFAULT_SAMPLE_RATE, DEFAULT_BPM))
        };

        orchestrator.set_enable_dev_experiment(args.debug);
        orchestrator.set_should_output_perf(args.perf);

        if !args.quiet {
            print!("Performing to queue ");
        }
        let mut clock_settings = orchestrator.clock_settings().clone();
        clock_settings.set_sample_rate(if args.wav {
            44100
        } else {
            IOHelper::get_output_device_sample_rate()
        });
        let start_instant = Instant::now();
        let mut sample_buffer = [StereoSample::SILENCE; 64];
        let performance = orchestrator.run_performance(&mut sample_buffer, args.quiet)?;
        if args.perf {
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
        }
        if !args.quiet {
            println!("Rendering queue");
        }
        if args.wav {
            let re = Regex::new(r"\.ya?ml$").unwrap();
            let output_filename = re.replace(&input_filename, ".wav");
            if input_filename == output_filename {
                panic!("would overwrite input file; couldn't generate output filename");
            }
            IOHelper::send_performance_to_file(&performance, &output_filename)?;
        } else {
            IOHelper::send_performance_to_output_device(&performance)?;
        }
    }
    Ok(())
}
