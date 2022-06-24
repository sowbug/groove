use std::{cell::RefCell, rc::Rc};

use super::instruments::Sequencer;
use midly::{MidiMessage as MidlyMidiMessage, TrackEventKind};

pub enum MidiMessageType {
    NoteOn = 0x1001,
    _NoteOff = 0x1000,
}
pub struct MidiMessage {
    // status and channel are normally packed into one byte, but for ease of use
    // we're unpacking here.
    pub status: MidiMessageType,
    pub channel: u8,
    pub data1: u8,
    pub data2: u8,
}

impl MidiMessage {
    pub fn to_frequency(&self) -> f32 {
        match self.data1 {
            0 => 0.,
            _ => 2.0_f32.powf((self.data1 as f32 - 69.0) / 12.0) * 440.0,
        }
    }
}

pub struct MidiReader {}

impl MidiReader {
    pub fn load_sequencer(data: &[u8], sequencer: Rc<RefCell<Sequencer>>) {
        let smf = midly::Smf::parse(data).unwrap();

        let mut ticks_per_click: f32 = 0.;
        let mut seconds_per_tick: f32 = 0.;
        for track in smf.tracks.iter() {
            let mut track_time_ticks: u32 = 0; // Each track's relative time references start over at zero
            for t in track.iter() {
                match t.kind {
                    TrackEventKind::Midi {
                        channel: _,
                        message,
                    } => {
                        let delta = t.delta;
                        track_time_ticks += delta.as_int();
                        match message {
                            MidlyMidiMessage::NoteOn { key, vel } => {
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
                            MidlyMidiMessage::NoteOff { key, vel: _ } => {
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
                            let bpm = 60.0 * 4.0 / (1000000.0 / (tempo.as_int() as f32));
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
    }
}
