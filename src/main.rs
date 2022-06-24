extern crate anyhow;
extern crate cpal;

mod backend;

use crate::backend::orchestrator::Orchestrator;

use backend::{
    devices::DeviceTrait,
    effects::Quietener,
    instruments::{Oscillator, Sequencer, Waveform},
    midi::MidiReader,
};
use clap::Parser;
use cpal::{
    traits::{DeviceTrait as CpalDeviceTrait, HostTrait, StreamTrait},
    SampleRate, StreamConfig,
};
use crossbeam::deque::{Stealer, Worker};

use std::cell::RefCell;

use std::rc::Rc;

// TODO: Controller?

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Output filename
    #[clap(short, long, value_parser)]
    out: Option<String>,
}

pub fn get_sample_from_queue<T: cpal::Sample>(
    stealer: &Stealer<f32>,
    data: &mut [T],
    _info: &cpal::OutputCallbackInfo,
) {
    for next_sample in data.iter_mut() {
        let sample_option = stealer.steal();
        let sample: f32 = if sample_option.is_success() {
            sample_option.success().unwrap_or_default()
        } else {
            0.
        };
        *next_sample = cpal::Sample::from(&sample);
    }
}

fn send_performance_to_output_device(sample_rate: u32, worker: &Worker<f32>) -> anyhow::Result<()> {
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

    let stream = match sample_format {
        cpal::SampleFormat::F32 => device.build_output_stream(
            &config,
            move |data, output_callback_info| {
                get_sample_from_queue::<f32>(&stealer, data, output_callback_info)
            },
            err_fn,
        ),
        cpal::SampleFormat::I16 => device.build_output_stream(
            &config,
            move |data, output_callback_info| {
                get_sample_from_queue::<i16>(&stealer, data, output_callback_info)
            },
            err_fn,
        ),
        cpal::SampleFormat::U16 => device.build_output_stream(
            &config,
            move |data, output_callback_info| {
                get_sample_from_queue::<u16>(&stealer, data, output_callback_info)
            },
            err_fn,
        ),
    }
    .unwrap();

    stream.play()?;
    while !worker.is_empty() {
        // TODO(miket): learn how thread sync primitives work in Rust
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    Ok(())
}

pub fn send_performance_to_file(
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

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let output_filename = args.out.unwrap_or_default();
    let should_write_output = if output_filename.is_empty() {
        println!("will output to speaker");
        false
    } else {
        println!("will output to {}", output_filename);
        true
    };

    // TODO: get this from cpal. Today cpal gets it from this hardcoded value.
    let mut orchestrator = Orchestrator::new(44100);

    let sine_oscillator: Rc<RefCell<_>> = Rc::new(RefCell::new(Oscillator::new(Waveform::Sine)));
    orchestrator.add_device(sine_oscillator.clone());

    let square_oscillator: Rc<RefCell<_>> =
        Rc::new(RefCell::new(Oscillator::new(Waveform::Square)));
    orchestrator.add_device(square_oscillator.clone());

    let triangle_oscillator: Rc<RefCell<_>> =
        Rc::new(RefCell::new(Oscillator::new(Waveform::Triangle)));
    orchestrator.add_device(triangle_oscillator.clone());

    let sawtooth_oscillator: Rc<RefCell<_>> =
        Rc::new(RefCell::new(Oscillator::new(Waveform::Sawtooth)));
    orchestrator.add_device(sawtooth_oscillator.clone());

    // let quietener: Rc<RefCell<_>> =
    //     Rc::new(RefCell::new(Quietener::new(square_oscillator.clone())));
    // orchestrator.add_device(quietener.clone());
    // quietener
    //     .borrow_mut()
    //     .add_audio_source(square_oscillator.clone());

    let sequencer: Rc<RefCell<_>> = Rc::new(RefCell::new(Sequencer::new()));

    let data = std::fs::read("sound_of_music.mid").unwrap();
    MidiReader::load_sequencer(&data, sequencer.clone());

    orchestrator.add_device(sequencer.clone());
    {
        let mut mixer = orchestrator.master_mixer.borrow_mut();
        // mixer.add_audio_source(quietener);
        mixer.add_audio_source(sine_oscillator.clone());
        mixer.add_audio_source(square_oscillator.clone());
        mixer.add_audio_source(triangle_oscillator.clone());
        mixer.add_audio_source(sawtooth_oscillator.clone());
    }

    sequencer.borrow_mut().connect_midi_sink(sine_oscillator);
    sequencer.borrow_mut().connect_midi_sink(square_oscillator);
    sequencer
        .borrow_mut()
        .connect_midi_sink(triangle_oscillator);
    sequencer
        .borrow_mut()
        .connect_midi_sink(sawtooth_oscillator);

    let worker = Worker::<f32>::new_fifo();
    let sample_rate = orchestrator.clock.sample_rate as u32;
    let result = orchestrator.perform_to_queue(&worker);
    if result.is_err() {
        return result;
    }

    if should_write_output {
        send_performance_to_file(sample_rate, &output_filename, &worker)
    } else {
        send_performance_to_output_device(sample_rate, &worker)
    }
}
