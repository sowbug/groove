mod backend;

use std::cell::RefCell;
use std::rc::Rc;

use crate::backend::orchestrator::Orchestrator;

use backend::{
    devices::DeviceTrait,
    effects::Quietener,
    instruments::{Oscillator, Waveform, Sequencer},
};
use cpal::{
    traits::{HostTrait, StreamTrait},
    StreamConfig,
};

// TODO: Controller?

// fn old_main() -> anyhow::Result<()> {
//     let mut orchestrator: Orchestrator = Orchestrator::new();
//     orchestrator.tmp_add_some_notes();

//     let host = cpal::default_host();
//     let device = host
//         .default_output_device()
//         .expect("no output device available");

//     let mut supported_configs_range = device
//         .supported_output_configs()
//         .expect("error while querying configs");
//     let supported_config = supported_configs_range
//         .next()
//         .expect("no supported config?!")
//         .with_max_sample_rate();

//     let err_fn = |err| eprintln!("an error occurred on the output audio stream: {}", err);
//     let sample_format = supported_config.sample_format();
//     let config: StreamConfig = supported_config.into();

//     orchestrator.clock.sample_rate = config.sample_rate.0 as f32;
//     orchestrator.clock.sample_clock = 0f32;

//     let stream = match sample_format {
//         cpal::SampleFormat::F32 => device.build_output_stream(
//             &config,
//             move |data, output_callback_info| {
//                 orchestrator.write_sample_data::<f32>(data, output_callback_info)
//             },
//             err_fn,
//         ),
//         cpal::SampleFormat::I16 => device.build_output_stream(
//             &config,
//             move |data, output_callback_info| {
//                 orchestrator.write_sample_data::<i16>(data, output_callback_info)
//             },
//             err_fn,
//         ),
//         cpal::SampleFormat::U16 => device.build_output_stream(
//             &config,
//             move |data, output_callback_info| {
//                 orchestrator.write_sample_data::<u16>(data, output_callback_info)
//             },
//             err_fn,
//         ),
//     }
//     .unwrap();

//     stream.play()?;
//     std::thread::sleep(std::time::Duration::from_millis(3000));
//     Ok(())
// }

fn main() -> anyhow::Result<()> {
    let mut orchestrator = Orchestrator::new();

    let square_oscillator: Rc<RefCell<_>> =
        Rc::new(RefCell::new(Oscillator::new(Waveform::Square)));
    orchestrator.add_device(square_oscillator.clone());

    let sine_oscillator: Rc<RefCell<_>> = Rc::new(RefCell::new(Oscillator::new(Waveform::Sine)));
    orchestrator.add_device(sine_oscillator.clone());

    let quietener: Rc<RefCell<_>> =
        Rc::new(RefCell::new(Quietener::new(square_oscillator.clone())));
    orchestrator.add_device(quietener.clone());

    let midi_instrument: Rc<RefCell<_>> = Rc::new(RefCell::new(Sequencer::new()));

    orchestrator.add_device(midi_instrument.clone());

    quietener
        .borrow_mut()
        .add_audio_source(square_oscillator.clone());

    {
        let mut mixer = orchestrator.master_mixer.borrow_mut();
        mixer.add_audio_source(quietener);
        mixer.add_audio_source(sine_oscillator.clone());
    }

    midi_instrument
        .borrow_mut()
        .connect_midi_sink(square_oscillator.clone());
    midi_instrument
        .borrow_mut()
        .connect_midi_sink(sine_oscillator.clone());

    orchestrator.play();
    Ok(())
}
