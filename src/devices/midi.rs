use midly::{MidiMessage as MidlyMidiMessage, TrackEventKind};
use std::{cell::RefCell, cmp::Ordering, rc::Rc};

use super::sequencer::Sequencer;

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum MidiMessageType {
    NoteOn = 0x1001,
    NoteOff = 0x1000,
}
#[derive(Debug, Eq, PartialEq, PartialOrd, Ord)]
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
        2.0_f32.powf((self.data1 as f32 - 69.0) / 12.0) * 440.0
    }

    pub fn new_note_on(note: u8, vel: u8) -> MidiMessage {
        MidiMessage {
            status: MidiMessageType::NoteOn,
            channel: 0,
            data1: note,
            data2: vel,
        }
    }

    pub fn new_note_off(note: u8, vel: u8) -> MidiMessage {
        MidiMessage {
            status: MidiMessageType::NoteOff,
            channel: 0,
            data1: note,
            data2: vel,
        }
    }
}

#[derive(Eq, Debug)]
pub struct OrderedMidiMessage {
    pub when: u32,
    pub message: MidiMessage,
}

impl Ord for OrderedMidiMessage {
    fn cmp(&self, other: &Self) -> Ordering {
        self.when.cmp(&other.when)
    }
}

impl PartialOrd for OrderedMidiMessage {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for OrderedMidiMessage {
    fn eq(&self, other: &Self) -> bool {
        self.when == other.when
    }
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;

    use super::*;

    #[test]
    fn test_note_to_frequency() {
        assert_approx_eq!(MidiMessage::new_note_on(60, 0).to_frequency(), 261.625549);
        assert_approx_eq!(MidiMessage::new_note_on(0, 0).to_frequency(), 8.175798);
        assert_approx_eq!(MidiMessage::new_note_on(127, 0).to_frequency(), 12543.855);
    }
}

pub struct MidiReader {}

impl MidiReader {
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
                                        message: MidiMessage {
                                            status: MidiMessageType::NoteOff,
                                            channel: channel.as_int(),
                                            data1: key.as_int(),
                                            data2: vel.as_int(),
                                        },
                                    }
                                } else {
                                    OrderedMidiMessage {
                                        when: track_time_ticks,
                                        message: MidiMessage {
                                            status: MidiMessageType::NoteOn,
                                            channel: channel.as_int(),
                                            data1: key.as_int(),
                                            data2: vel.as_int(),
                                        },
                                    }
                                };
                                sequencer.borrow_mut().add_message(midi_message);
                            }
                            MidlyMidiMessage::NoteOff { key, vel } => {
                                let midi_message = OrderedMidiMessage {
                                    when: track_time_ticks,
                                    message: MidiMessage {
                                        status: MidiMessageType::NoteOff,
                                        channel: channel.as_int(),
                                        data1: key.as_int(),
                                        data2: vel.as_int(),
                                    },
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
                    _ => {}
                }
            }
        }
        println!("Done processing MIDI file");
    }
}
