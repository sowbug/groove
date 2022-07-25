use midir::{Ignore, MidiInput};
use midly::{live::LiveEvent, MidiMessage as MidlyMidiMessage, TrackEventKind};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Condvar, Mutex},
};

use crate::{
    common::{MidiMessage, OrderedMidiMessage},
    primitives::clock::Clock,
};

use super::{sequencer::Sequencer, traits::DeviceTrait};

pub struct MidiControllerReader {
    sinks: Vec<Rc<RefCell<dyn DeviceTrait>>>,
}

impl MidiControllerReader {
    pub fn new() -> Self {
        Self { sinks: Vec::new() }
    }

    pub fn connect(&mut self) {
        let mut midi_in = match MidiInput::new(std::any::type_name::<Self>()) {
            Ok(t) => t,
            Err(e) => {
                panic!("{:?}", e)
            }
        };
        midi_in.ignore(Ignore::None);
        let in_ports = midi_in.ports();
        let in_port = match in_ports.len() {
            0 => panic!("no input port found"),
            1 => {
                println!(
                    "Choosing the only available input port: {}",
                    midi_in.port_name(&in_ports[0]).unwrap()
                );
                &in_ports[0]
            }
            _ => {
                println!("\nAvailable input ports:");
                for (i, p) in in_ports.iter().enumerate() {
                    println!("{}: {}", i, midi_in.port_name(p).unwrap());
                }
                print!("Choosing second...");
                in_ports
                    .get(1)
                    .ok_or("invalid input port selected")
                    .unwrap()
            }
        };
        let pair = Arc::new((Mutex::new(false), Condvar::new()));
        let pair2 = Arc::clone(&pair);
        let _ = midi_in.connect(
            in_port,
            "hoo hah",
            move |timestamp, message, _| {
                println!("{}: {:?} (len = {})", timestamp, message, message.len());
                let event = LiveEvent::parse(message).unwrap();
                match event {
                    LiveEvent::Midi { channel, message } => match message {
                        #[allow(unused_variables)]
                        MidlyMidiMessage::NoteOn { key, vel } => {
                            println!("hit note {} on channel {}", key, channel);
                            // self.handle_midi_message(
                            //     &MidiMessage::new_note_on(0, u8::from(key), u8::from(vel)),
                            //     &Clock::new(44100, 4, 4, 128.0),
                            // );
                            if key == 60 {
                                let (lock, cvar) = &*pair2;
                                let mut started = lock.lock().unwrap();
                                *started = true;
                                // We notify the condvar that the value has changed.
                                cvar.notify_one();
                            }
                        }
                        _ => {
                            println!("midi message other");
                        }
                    },
                    _ => {
                        println!("other message other");
                    }
                }
            },
            (),
        );
        let (lock, cvar) = &*pair;
        let mut started = lock.lock().unwrap();
        while !*started {
            started = cvar.wait(started).unwrap();
        }
        //                std::thread::sleep(std::time::Duration::from_millis(5000));
    }
}

#[allow(unused_variables)]
impl DeviceTrait for MidiControllerReader {
    fn sources_midi(&self) -> bool {
        true
    }
    fn connect_midi_sink(&mut self, device: Rc<RefCell<dyn DeviceTrait>>) {
        self.sinks.push(device);
    }
    fn handle_midi_message(&mut self, message: &MidiMessage, clock: &Clock) {}

    fn tick(&mut self, clock: &Clock) -> bool {
        false
    }
}

pub struct MidiSmfReader {}

impl MidiSmfReader {
    pub fn load_sequencer(data: &[u8], sequencer: Rc<RefCell<Sequencer>>) {
        let parse_result = midly::Smf::parse(data).unwrap();

        struct MetaInfo {
            // Pulses per quarter-note
            ppq: u32,

            // Microseconds per quarter-note
            tempo: u32,

            time_signature_numerator: u8,
            time_signature_denominator_exp: u8,
        }
        let mut meta_info = MetaInfo {
            ppq: match parse_result.header.timing {
                midly::Timing::Metrical(ticks_per_beat) => ticks_per_beat.as_int() as u32,
                _ => 0,
            },
            tempo: 0,

            // https://en.wikipedia.org/wiki/Time_signature
            time_signature_numerator: 0,
            time_signature_denominator_exp: 0,
        };
        let mut track_number: u32 = 0;
        for track in parse_result.tracks.iter() {
            println!("Processing track {}", track_number);
            track_number += 1;
            let mut track_time_ticks: u32 = 0; // The relative time references start over at zero with each track.

            for t in track.iter() {
                match t.kind {
                    TrackEventKind::Midi { channel, message } => {
                        let delta = t.delta;
                        track_time_ticks += delta.as_int();
                        match message {
                            MidlyMidiMessage::NoteOn { key, vel } => {
                                let midi_message = if vel == 0 {
                                    OrderedMidiMessage {
                                        when: track_time_ticks,
                                        message: MidiMessage::new_note_off(
                                            channel.as_int(),
                                            key.as_int(),
                                            vel.as_int(),
                                        ),
                                    }
                                } else {
                                    OrderedMidiMessage {
                                        when: track_time_ticks,
                                        message: MidiMessage::new_note_on(
                                            channel.as_int(),
                                            key.as_int(),
                                            vel.as_int(),
                                        ),
                                    }
                                };
                                sequencer.borrow_mut().add_message(midi_message);
                            }
                            MidlyMidiMessage::NoteOff { key, vel } => {
                                let midi_message = OrderedMidiMessage {
                                    when: track_time_ticks,
                                    message: MidiMessage::new_note_off(
                                        channel.as_int(),
                                        key.as_int(),
                                        vel.as_int(),
                                    ),
                                };
                                sequencer.borrow_mut().add_message(midi_message);
                            }
                            MidlyMidiMessage::ProgramChange { program } => {
                                let midi_message = OrderedMidiMessage {
                                    when: track_time_ticks,
                                    message: MidiMessage::new_program_change(
                                        channel.as_int(),
                                        program.as_int(),
                                    ),
                                };
                                sequencer.borrow_mut().add_message(midi_message);
                            }
                            _ => {
                                // println!("skipping {:?}", message);
                            }
                        }
                    }

                    TrackEventKind::Meta(meta_message) => match meta_message {
                        midly::MetaMessage::TimeSignature(numerator, denominator_exp, _cc, _bb) => {
                            meta_info.time_signature_numerator = numerator;
                            meta_info.time_signature_denominator_exp = denominator_exp;
                            //meta_info.ppq = cc; WHA???
                        }
                        midly::MetaMessage::Tempo(tempo) => {
                            meta_info.tempo = tempo.as_int();
                        }
                        midly::MetaMessage::TrackNumber(track_opt) => {
                            if track_opt.is_none() {
                                continue;
                            }
                        }
                        midly::MetaMessage::EndOfTrack => {
                            let _time_signature: (u32, u32) = (
                                meta_info.time_signature_numerator.into(),
                                2_u32.pow(meta_info.time_signature_denominator_exp.into()),
                            );
                            let ticks_per_quarter_note: f32 = meta_info.ppq as f32;
                            let seconds_per_quarter_note: f32 = meta_info.tempo as f32 / 1000000.0;
                            let ticks_per_second =
                                ticks_per_quarter_note / seconds_per_quarter_note;

                            let _bpm: f32 = (60.0 * 1000000.0) / (meta_info.tempo as f32);

                            sequencer
                                .borrow_mut()
                                .set_midi_ticks_per_second(ticks_per_second as u32);
                        }
                        _ => {}
                    },
                    TrackEventKind::SysEx(_data) => { // TODO
                    }
                    TrackEventKind::Escape(_data) => { // TODO
                    }
                }
            }
        }
        println!("Done processing MIDI file");
    }
}
