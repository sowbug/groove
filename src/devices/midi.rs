use midly::{MidiMessage as MidlyMidiMessage, TrackEventKind};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use crate::{
    common::{MidiChannel, MidiMessage, OrderedMidiMessage, MIDI_CHANNEL_RECEIVE_ALL},
    traits::{SinksMidi, SourcesMidi},
};

use super::sequencer::MidiSequencer;

pub struct MidiSmfReader {}

impl MidiSmfReader {
    pub fn load_sequencer(data: &[u8], sequencer: Rc<RefCell<MidiSequencer>>) {
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
        for (track_number, track) in parse_result.tracks.iter().enumerate() {
            println!("Processing track {}", track_number);
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

#[derive(Debug, Default)]
pub(crate) struct MidiBus {
    channels_to_sink_vecs: HashMap<MidiChannel, Vec<Weak<RefCell<dyn SinksMidi>>>>,
}
impl SinksMidi for MidiBus {
    fn midi_channel(&self) -> MidiChannel {
        MIDI_CHANNEL_RECEIVE_ALL
    }

    fn set_midi_channel(&mut self, _midi_channel: MidiChannel) {}

    fn handle_midi_for_channel(
        &mut self,
        clock: &crate::primitives::clock::Clock,
        message: &MidiMessage,
    ) {
        // send to everyone EXCEPT whoever sent it!
        self.issue_midi(clock, message);
    }
}
impl SourcesMidi for MidiBus {
    fn midi_sinks(&self) -> &HashMap<MidiChannel, Vec<Weak<RefCell<dyn SinksMidi>>>> {
        &self.channels_to_sink_vecs
    }

    fn midi_sinks_mut(&mut self) -> &mut HashMap<MidiChannel, Vec<Weak<RefCell<dyn SinksMidi>>>> {
        &mut self.channels_to_sink_vecs
    }
}
impl MidiBus {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}
