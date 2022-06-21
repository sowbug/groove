extern crate anyhow;
extern crate cpal;

pub mod clock;
pub mod instruments;
pub mod midi;
pub mod orchestrator;
pub mod sequencer;

use crate::orchestrator::Orchestrator;

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    StreamConfig,
};

// TODO: Controller?

fn main() -> anyhow::Result<()> {
    let mut orchestrator: Orchestrator = Orchestrator::new();
    orchestrator.tmp_add_some_notes();

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
        .with_max_sample_rate();

    let err_fn = |err| eprintln!("an error occurred on the output audio stream: {}", err);
    let sample_format = supported_config.sample_format();
    let config: StreamConfig = supported_config.into();

    orchestrator.clock.sample_rate = config.sample_rate.0 as f32;
    orchestrator.clock.sample_clock = 0f32;

    let stream = match sample_format {
        cpal::SampleFormat::F32 => device.build_output_stream(
            &config,
            move |data, output_callback_info| {
                orchestrator.write_sample_data::<f32>(data, output_callback_info)
            },
            err_fn,
        ),
        cpal::SampleFormat::I16 => device.build_output_stream(
            &config,
            move |data, output_callback_info| {
                orchestrator.write_sample_data::<i16>(data, output_callback_info)
            },
            err_fn,
        ),
        cpal::SampleFormat::U16 => device.build_output_stream(
            &config,
            move |data, output_callback_info| {
                orchestrator.write_sample_data::<u16>(data, output_callback_info)
            },
            err_fn,
        ),
    }
    .unwrap();

    stream.play()?;
    std::thread::sleep(std::time::Duration::from_millis(3000));
    Ok(())
}
