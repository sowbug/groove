#![feature(trait_upcasting)]
#![allow(incomplete_features)]

use libgroove::{
    common::MonoSample,
    devices::{midi::MidiSmfReader, orchestrator::Orchestrator, sequencer::MidiSequencer},
    helpers::{send_performance_to_file, send_performance_to_output_device},
    primitives::IsMidiInstrument,
    settings::song::SongSettings,
    synthesizers::{
        drumkit_sampler::Sampler,
        welsh::{PresetName, Synth, SynthPreset},
    },
};

use anyhow::Ok;
use clap::Parser;
use crossbeam::deque::Worker;
use std::{cell::RefCell, rc::Rc};

#[derive(Debug, Default)]
struct ClDaw {
    orchestrator: Orchestrator,
}

impl ClDaw {
    pub fn new() -> Self {
        Self {
            orchestrator: Orchestrator::new_defaults(),
        }
    }

    pub fn new_from_yaml_file(filename: String) -> Self {
        let yaml = std::fs::read_to_string(filename).unwrap();
        let settings = SongSettings::new_from_yaml(yaml.as_str());
        Self {
            orchestrator: Orchestrator::new(settings.unwrap()),
        }
    }

    pub fn new_from_midi_file(filename: String) -> Self {
        let data = std::fs::read(filename).unwrap();
        let mut result = Self {
            orchestrator: Orchestrator::new_defaults(),
        };
        MidiSmfReader::load_sequencer(&data, result.orchestrator.midi_sequencer());

        for channel in 0..MidiSequencer::connected_channel_count() {
            let synth: Rc<RefCell<dyn IsMidiInstrument>> = if channel == 9 {
                Rc::new(RefCell::new(Sampler::new_from_files(channel)))
            } else {
                Rc::new(RefCell::new(Synth::new(
                    channel,
                    result.orchestrator.settings().clock.sample_rate(),
                    SynthPreset::by_name(&PresetName::Piano),
                )))
            };
            // We make up IDs here, as we know that MIDI won't be referencing them.
            let instrument = Rc::clone(&synth);
            result
                .orchestrator
                .add_instrument_by_id(format!("instrument-{}", channel), instrument);
            let sink = Rc::downgrade(&synth);
            result
                .orchestrator
                .connect_to_downstream_midi_bus(channel, sink);
            result.orchestrator.add_main_mixer_source(synth);
        }
        result
    }

    pub fn perform(
        orchestrator: &mut Orchestrator,
        sample_rate: usize,
        wav_out: Option<String>,
    ) -> anyhow::Result<()> {
        print!("Performing to queue ");
        let worker = Worker::<MonoSample>::new_fifo();
        orchestrator.perform_to_queue(&worker)?;

        println!("Rendering queue");
        if let Some(output_filename) = wav_out {
            send_performance_to_file(sample_rate, &output_filename, &worker)
        } else {
            send_performance_to_output_device(sample_rate, &worker)
        }
    }
}

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
        let mut command_line_daw = if args.midi_in.is_some() {
            ClDaw::new_from_midi_file(args.midi_in.unwrap())
        } else if args.yaml_in.is_some() {
            ClDaw::new_from_yaml_file(args.yaml_in.unwrap())
        } else {
            ClDaw::new()
        };

        command_line_daw.perform(args.wav_out)
    }
}
