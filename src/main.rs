extern crate anyhow;
extern crate cpal;

mod backend;

use crate::backend::orchestrator::Orchestrator;

use backend::{
    devices::DeviceTrait as GrooveDeviceTrait,
    effects::Quietener,
    instruments::{Oscillator, Sequencer, Waveform},
};
use clap::Parser;

use midly::{MidiMessage, TrackEventKind};
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

// fn perform_to_output_device(orchestrator: Orchestrator) -> anyhow::Result<()> {
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
    let args = Args::parse();

    let output_filename = args.out.unwrap_or_default();
    let should_write_output = if output_filename.is_empty() {
        println!("will output to speaker");
        false
    } else {
        println!("will output to {}", output_filename);
        true
    };

    let mut orchestrator = Orchestrator::new(44100); // TODO: get this from cpal

    let square_oscillator: Rc<RefCell<_>> =
        Rc::new(RefCell::new(Oscillator::new(Waveform::Square)));
    orchestrator.add_device(square_oscillator.clone());

    let sine_oscillator: Rc<RefCell<_>> = Rc::new(RefCell::new(Oscillator::new(Waveform::Sine)));
    orchestrator.add_device(sine_oscillator.clone());

    let quietener: Rc<RefCell<_>> =
        Rc::new(RefCell::new(Quietener::new(square_oscillator.clone())));
    orchestrator.add_device(quietener.clone());

    let sequencer: Rc<RefCell<_>> = Rc::new(RefCell::new(Sequencer::new()));

    let data = std::fs::read("jingle_bells.mid").unwrap();
    let smf = midly::Smf::parse(&data).unwrap();

    // Use the information
    println!("midi file has {} tracks!", smf.tracks.len());

    let mut bpm: f32 = 0.;
    let mut ticks_per_click: f32 = 0.;
    let mut seconds_per_tick: f32 = 0.;
    for track in smf.tracks.iter() {
        let mut track_time_ticks: u32 = 0; // Each track's relative time references start over at zero
        for t in track.iter() {
            match t.kind {
                TrackEventKind::Midi { channel: _, message } => {
                    let delta = t.delta;
                    track_time_ticks += delta.as_int();
                    match message {
                        MidiMessage::NoteOn { key, vel } => {
                            if vel == 0 {
                                sequencer.borrow_mut().add_note_off(
                                    key.as_int(),
                                    track_time_ticks as f32 * seconds_per_tick,
                                );
                                // println!("note {} DE FACTO OFF at time {}", key, time);
                            } else {
                                sequencer.borrow_mut().add_note_on(
                                    key.as_int(),
                                    track_time_ticks as f32 * seconds_per_tick,
                                );
                                // println!("note {} ON at time {}", key, time);
                            }
                        }
                        MidiMessage::NoteOff { key, vel: _ } => {
                            println!("note {} OFF at time {}", key, track_time_ticks);
                            sequencer.borrow_mut().add_note_off(
                                key.as_int(),
                                track_time_ticks as f32 * seconds_per_tick,
                            );
                        }
                        _ => {
                            // println!("skipping {:?}", message);
                        }
                    }
                }
                TrackEventKind::Meta(meta_message) => match meta_message {
                    midly::MetaMessage::TimeSignature(
                        _numerator,
                        _denominator_exp,
                        cc,
                        _bb,
                    ) => {
                        ticks_per_click = cc as f32;
                        println!("ticks per click {}", ticks_per_click);
                    }
                    midly::MetaMessage::Tempo(tempo) => {
                        println!("microseconds per beat: {}", tempo);
                        // TODO: handle time signatures
                        bpm = 60.0 * 4.0 / (1000000.0 / (tempo.as_int() as f32));
                        seconds_per_tick = 1. / (ticks_per_click * 4. * 2.);
                        println!("BPM: {}. seconds per tick: {}", bpm, seconds_per_tick);
                    }
                    midly::MetaMessage::TrackNumber(track_opt) => {
                        if track_opt.is_none() {
                            continue;
                        }
                        let track_number = track_opt.unwrap();
                        if track_number > 2 {
                            continue;
                        }
                    }
                    _ => {}
                },
                _ => {
                    //        println!("skipping {:?}", t.kind);
                }
            }
            //println!("track event {:?}", t.kind);
        }
    }

    orchestrator.add_device(sequencer.clone());

    quietener
        .borrow_mut()
        .add_audio_source(square_oscillator.clone());

    {
        let mut mixer = orchestrator.master_mixer.borrow_mut();
        mixer.add_audio_source(quietener);
        mixer.add_audio_source(sine_oscillator.clone());
    }

    sequencer
        .borrow_mut()
        .connect_midi_sink(square_oscillator);
    sequencer
        .borrow_mut()
        .connect_midi_sink(sine_oscillator);

    if should_write_output {
        orchestrator.perform_to_file(&output_filename)
    } else {
        Ok(()) // TODO
               //perform_to_output_device(orchestrator)
    }
}
