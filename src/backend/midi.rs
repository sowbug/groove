use super::instruments::Sequencer;
use midly::{MidiMessage as MidlyMidiMessage, TrackEventKind};
use std::{cell::RefCell, rc::Rc};

#[derive(Debug)]
pub enum MidiMessageType {
    NoteOn = 0x1001,
    NoteOff = 0x1000,
}
#[derive(Debug)]
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
        let smf = midly::Smf::parse(data).unwrap();

        struct MetaInfo {
            // Pulses per quarter-note
            ppq: usize,

            // Microseconds per quarter-note
            tempo: usize,

            time_signature_numerator: u8,
            time_signature_denominator_exp: u8,
        }
        let mut meta_info = MetaInfo {
            ppq: match smf.header.timing {
                midly::Timing::Metrical(ticks_per_beat) => ticks_per_beat.as_int() as usize,
                _ => 0,
            },
            tempo: 0,

            // https://en.wikipedia.org/wiki/Time_signature
            time_signature_numerator: 0,
            time_signature_denominator_exp: 0,
        };
        for track in smf.tracks.iter() {
            let mut track_time_ticks: usize = 0; // The relative time references start over at zero with each track.
            for t in track.iter() {
                match t.kind {
                    TrackEventKind::Midi {
                        channel: u4,
                        message,
                    } => {
                        let delta = t.delta;
                        track_time_ticks += delta.as_int() as usize;
                        match message {
                            MidlyMidiMessage::NoteOn { key, vel } => {
                                let midi_message = if vel == 0 {
                                    MidiMessage {
                                        status: MidiMessageType::NoteOff,
                                        channel: u4.as_int(),
                                        data1: key.as_int(),
                                        data2: vel.as_int(),
                                    }
                                } else {
                                    MidiMessage {
                                        status: MidiMessageType::NoteOn,
                                        channel: u4.as_int(),
                                        data1: key.as_int(),
                                        data2: vel.as_int(),
                                    }
                                };
                                sequencer
                                    .borrow_mut()
                                    .add_message(track_time_ticks, midi_message);
                            }
                            MidlyMidiMessage::NoteOff { key, vel } => {
                                let midi_message = MidiMessage {
                                    status: MidiMessageType::NoteOff,
                                    channel: u4.as_int(),
                                    data1: key.as_int(),
                                    data2: vel.as_int(),
                                };
                                sequencer
                                    .borrow_mut()
                                    .add_message(track_time_ticks, midi_message);
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
                            meta_info.tempo = tempo.as_int() as usize;
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
                        midly::MetaMessage::EndOfTrack => {
                            let time_signature: (usize, usize) = (
                                meta_info.time_signature_numerator.into(),
                                2_usize.pow(meta_info.time_signature_denominator_exp.into()),
                            );
                            println!(
                                "Time signature is {}/{}",
                                time_signature.0, time_signature.1
                            );

                            let ticks_per_quarter_note: f32 = meta_info.ppq as f32;
                            let seconds_per_quarter_note: f32 = meta_info.tempo as f32 / 1000000.0;
                            let ticks_per_second =
                                ticks_per_quarter_note / seconds_per_quarter_note;

                            let bpm: f32 = (60.0 * 1000000.0) / (meta_info.tempo as f32);

                            // https://stackoverflow.com/a/2038364/344467
                            //                            let ticks_per_second: usize = (bpm * meta_info.tempo) / 60000;
                            println!("MIDI ticks per second: {}", ticks_per_second);
                            println!("BPM: {}", bpm);
                            sequencer
                                .borrow_mut()
                                .set_time_signature(time_signature.0, time_signature.1);
                            sequencer
                                .borrow_mut()
                                .set_midi_ticks_per_second(ticks_per_second as usize);
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
