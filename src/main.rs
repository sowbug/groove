extern crate anyhow;
extern crate cpal;

mod backend;
mod effects;
mod primitives;

use crate::{
    backend::{
        instruments::{CelloSynth2, SimpleSynth},
        orchestrator::Orchestrator,
    },
    primitives::{lfos::Lfo, oscillators::Waveform},
};
use backend::{devices::DeviceTrait, instruments::Sequencer, midi::MidiReader};
use clap::Parser;
use cpal::{
    traits::{DeviceTrait as CpalDeviceTrait, HostTrait, StreamTrait},
    SampleRate, StreamConfig,
};
use crossbeam::deque::{Stealer, Worker};
use std::{
    cell::RefCell,
    sync::{Arc, Condvar, Mutex},
};
use std::{ops::Rem, rc::Rc};

struct ClDaw {
    orchestrator: Orchestrator,
}

impl ClDaw {
    pub fn new(sample_rate: u32) -> ClDaw {
        ClDaw {
            orchestrator: Orchestrator::new(sample_rate),
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

    fn send_performance_to_output_device(
        &self,
        sample_rate: u32,
        worker: &Worker<f32>,
    ) -> anyhow::Result<()> {
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
            .with_sample_rate(SampleRate(sample_rate));

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
        sample_rate: u32,
        output_filename: &str,
        worker: &Worker<f32>,
    ) -> anyhow::Result<()> {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(output_filename, spec).unwrap();
        let amplitude = i16::MAX as f32;

        while !worker.is_empty() {
            let sample = worker.pop().unwrap_or_default();
            writer.write_sample((sample * amplitude) as i16).unwrap();
        }
        Ok(())
    }

    pub fn perform(
        &mut self,
        midi_in: Option<String>,
        wav_out: Option<String>,
    ) -> anyhow::Result<()> {
        if midi_in.is_some() {
            let sequencer = Rc::new(RefCell::new(Sequencer::new()));
            self.orchestrator.add_device(sequencer.clone());

            let data = std::fs::read(midi_in.unwrap()).unwrap();
            MidiReader::load_sequencer(&data, sequencer.clone());

            let lfo = Rc::new(RefCell::new(Lfo::new(3.0)));
            self.orchestrator.add_device(lfo.clone());

            //             // TODO: for now, we'll create one synth per MIDI channel. Later
            //             // I guess we'll analyze the sequencer content and figure out which are needed.
            //             for channel in 0..16 {
            //                 let waveform = match channel.rem(4) {
            //                     0 => Waveform::Sine,
            //                     1 => Waveform::Square,
            //                     2 => Waveform::Triangle,
            //                     3 => Waveform::Sawtooth,
            //                     4_u32..=u32::MAX => todo!(), // TODO: is this a problem with the compiler's smarts?
            //                 };
            // //                let simple_synth = Rc::new(RefCell::new(SimpleSynth::new(waveform, channel)));
            //                 let simple_synth = Rc::new(RefCell::new(CelloSynth2::new()));
            //                 self.orchestrator.add_device(simple_synth.clone());

            //                 // if channel == 0 {
            //                 //     let s2 = simple_synth.clone();
            //                 //     let target = move |value: f32| -> () {
            //                 //         let frequency = 440f32;
            //                 //         s2.borrow_mut()
            //                 //             .temp_set_oscillator_frequency(frequency + frequency * value * 0.25);
            //                 //     };
            //                 //     lfo.borrow_mut().connect_automation_sink(target);
            //                 // }

            //                 self.orchestrator
            //                     .master_mixer
            //                     .borrow_mut()
            //                     .add_audio_source(simple_synth.clone());

            //                 sequencer
            //                     .borrow_mut()
            //                     .connect_midi_sink_for_channel(simple_synth, channel);
            //             }

            let simple_synth = Rc::new(RefCell::new(CelloSynth2::new()));
            self.orchestrator.add_device(simple_synth.clone());

            self.orchestrator
                .master_mixer
                .borrow_mut()
                .add_audio_source(simple_synth.clone());

            sequencer
                .borrow_mut()
                .connect_midi_sink_for_channel(simple_synth, 0);
        }

        println!("Performing to queue");
        let worker = Worker::<f32>::new_fifo();
        let result = self.orchestrator.perform_to_queue(&worker);
        if result.is_err() {
            return result;
        }

        println!("Rendering queue");
        let sample_rate = self.orchestrator.clock.sample_rate() as u32;
        if let Some(output_filename) = wav_out {
            self.send_performance_to_file(sample_rate, &output_filename, &worker)
        } else {
            self.send_performance_to_output_device(sample_rate, &worker)
        }
    }
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// MIDI filename
    #[clap(short, long, value_parser)]
    midi_in: Option<String>,
    /// Output filename
    #[clap(short, long, value_parser)]
    wav_out: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let mut command_line_daw = ClDaw::new(44100);

    command_line_daw.perform(args.midi_in, args.wav_out)
}
