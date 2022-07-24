#[macro_use]
extern crate num_derive;

#[macro_use]
extern crate anyhow;

mod common;
mod devices;
mod general_midi;
mod preset;
mod primitives;
mod scripting;
mod synthesizers;

use crate::{
    devices::{
        midi::MidiControllerReader, orchestrator::Orchestrator, sequencer::Sequencer,
        traits::DeviceTrait,
    },
    synthesizers::drumkit_sampler::Sampler as DrumKitSampler,
};
use clap::Parser;
use cpal::{
    traits::{DeviceTrait as CpalDeviceTrait, HostTrait, StreamTrait},
    SampleRate, StreamConfig,
};
use crossbeam::deque::{Stealer, Worker};
use devices::{midi::MidiSmfReader, orchestrator::OrchestratorSettings};
use scripting::ScriptEngine;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Condvar, Mutex},
};
use synthesizers::welsh::*;

#[derive(Default)]
struct ClDaw {
    orchestrator: Orchestrator,
}

impl ClDaw {
    pub fn new() -> Self {
        Self {
            orchestrator: Orchestrator::new(OrchestratorSettings::new_dev()),
        }
    }

    fn get_sample_from_queue<T: cpal::Sample>(
        stealer: &Stealer<f32>,
        sync_pair: &Arc<(Mutex<bool>, Condvar)>,
        data: &mut [T],
        _info: &cpal::OutputCallbackInfo,
    ) {
        let lock = &(*sync_pair).0;
        let cvar = &(*sync_pair).1;
        let mut finished = lock.lock().unwrap();

        for next_sample in data.iter_mut() {
            let sample_option = stealer.steal();
            let sample: f32 = if sample_option.is_success() {
                sample_option.success().unwrap_or_default()
            } else {
                // TODO(miket): this isn't great, because we don't know whether
                // the steal failure was because of a spurious error (buffer underrun)
                // or complete processing.
                *finished = true;
                cvar.notify_one();
                0.
            };
            *next_sample = cpal::Sample::from(&sample);
        }
    }

    fn send_performance_to_output_device(&self, worker: &Worker<f32>) -> anyhow::Result<()> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("no output device available");

        let mut supported_configs_range = device
            .supported_output_configs()
            .expect("error while querying configs");
        let supported_config = supported_configs_range
            .next()
            .expect("no supported config?!")
            .with_sample_rate(SampleRate(self.orchestrator.settings().clock.sample_rate()));

        let err_fn = |err| eprintln!("an error occurred on the output audio stream: {}", err);
        let sample_format = supported_config.sample_format();
        let config: StreamConfig = supported_config.into();

        let stealer = worker.stealer();

        let sync_pair = Arc::new((Mutex::new(false), Condvar::new()));
        let sync_pair_clone = Arc::clone(&sync_pair);
        let stream = match sample_format {
            cpal::SampleFormat::F32 => device.build_output_stream(
                &config,
                move |data, output_callback_info| {
                    ClDaw::get_sample_from_queue::<f32>(
                        &stealer,
                        &sync_pair_clone,
                        data,
                        output_callback_info,
                    )
                },
                err_fn,
            ),
            cpal::SampleFormat::I16 => device.build_output_stream(
                &config,
                move |data, output_callback_info| {
                    ClDaw::get_sample_from_queue::<i16>(
                        &stealer,
                        &sync_pair_clone,
                        data,
                        output_callback_info,
                    )
                },
                err_fn,
            ),
            cpal::SampleFormat::U16 => device.build_output_stream(
                &config,
                move |data, output_callback_info| {
                    ClDaw::get_sample_from_queue::<u16>(
                        &stealer,
                        &sync_pair_clone,
                        data,
                        output_callback_info,
                    )
                },
                err_fn,
            ),
        }
        .unwrap();

        stream.play()?;

        // See https://doc.rust-lang.org/stable/std/sync/struct.Condvar.html for origin of this
        // code.
        let &(ref lock, ref cvar) = &*sync_pair;
        let mut finished = lock.lock().unwrap();
        while !*finished {
            finished = cvar.wait(finished).unwrap();
        }
        Ok(())
    }

    fn send_performance_to_file(
        &self,
        output_filename: &str,
        worker: &Worker<f32>,
    ) -> anyhow::Result<()> {
        const AMPLITUDE: f32 = i16::MAX as f32;
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: self.orchestrator.settings().clock.sample_rate(),
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(output_filename, spec).unwrap();

        while !worker.is_empty() {
            let sample = worker.pop().unwrap_or_default();
            writer.write_sample((sample * AMPLITUDE) as i16).unwrap();
        }
        Ok(())
    }

    pub fn perform(
        &mut self,
        midi_in: Option<String>,
        use_midi_controller: bool,
        wav_out: Option<String>,
    ) -> anyhow::Result<()> {
        println!("it is \n{}", serde_yaml::to_string(&self.orchestrator.settings()).unwrap());

        if midi_in.is_some() {
            let sequencer = Rc::new(RefCell::new(Sequencer::new()));
            self.orchestrator.add_device(sequencer.clone());

            let data = std::fs::read(midi_in.unwrap()).unwrap();
            MidiSmfReader::load_sequencer(&data, sequencer.clone());

            for channel_number in 0..Sequencer::connected_channel_count() {
                let synth: Rc<RefCell<dyn DeviceTrait>> = if channel_number == 9 {
                    Rc::new(RefCell::new(DrumKitSampler::new_from_files()))
                } else {
                    Rc::new(RefCell::new(Synth::new(
                        self.orchestrator.settings().clock.sample_rate(),
                        SynthPreset::by_name(&PresetName::Piano),
                    )))
                };
                self.orchestrator.add_device(synth.clone());
                self.orchestrator.add_master_mixer_source(synth.clone());

                sequencer
                    .borrow_mut()
                    .connect_midi_sink_for_channel(synth, channel_number);
            }
        }
        if use_midi_controller {
            panic!("sorry, this is horribly broken.");
            let synth = Rc::new(RefCell::new(Synth::new(
                self.orchestrator.settings().clock.sample_rate(),
                SynthPreset::by_name(&PresetName::Piano),
            )));
            self.orchestrator.add_device(synth.clone());
            self.orchestrator.add_master_mixer_source(synth.clone());
            let midi_input = Rc::new(RefCell::new(MidiControllerReader::new()));
            midi_input.borrow_mut().connect_midi_sink(synth.clone());
            self.orchestrator.add_device(midi_input.clone());
            midi_input.borrow_mut().connect();
        }
        println!("Performing to queue");
        let worker = Worker::<f32>::new_fifo();
        let result = self.orchestrator.perform_to_queue(&worker);
        if result.is_err() {
            return result;
        }

        println!("Rendering queue");
        if let Some(output_filename) = wav_out {
            self.send_performance_to_file(&output_filename, &worker)
        } else {
            self.send_performance_to_output_device(&worker)
        }
    }
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// MIDI filename
    #[clap(short, long, value_parser)]
    midi_in: Option<String>,

    /// Script to execute
    #[clap(short, long, value_parser)]
    script_in: Option<String>,

    /// Whether to use an external MIDI controller
    #[clap(short, long, parse(from_flag))]
    use_midi_controller: bool,

    /// Output filename
    #[clap(short, long, value_parser)]
    wav_out: Option<String>,
}

extern crate midir;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.script_in.is_some() {
        ScriptEngine::new().execute_file(&args.script_in.unwrap())
    } else {
        let mut command_line_daw = ClDaw::new();

        command_line_daw.perform(args.midi_in, args.use_midi_controller, args.wav_out)
    }
}
