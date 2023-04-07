// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::sequencers::Sequencer;
use crate::EntityMessage;
use groove_core::{
    midi::{new_note_off, new_note_on, HandlesMidi, MidiChannel, MidiMessage},
    time::PerfectTimeUnit,
    traits::{IsController, Performs, Resets, TicksWithMessages},
    ParameterType,
};
use groove_proc_macros::{Nano, Uid};
use std::str::FromStr;
use strum::EnumCount;
use strum_macros::{Display, EnumCount as EnumCountMacro, EnumString, FromRepr, IntoStaticStr};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

/// [Arpeggiator] creates [arpeggios](https://en.wikipedia.org/wiki/Arpeggio),
/// which "is a type of broken chord in which the notes that compose a chord are
/// individually and quickly sounded in a progressive rising or descending
/// order." You can also think of it as a hybrid MIDI instrument and MIDI
/// controller; you play it with MIDI, but instead of producing audio, it
/// produces more MIDI.
#[derive(Debug, Nano, Uid)]
pub struct Arpeggiator {
    uid: usize,
    midi_channel_out: MidiChannel,
    sequencer: Sequencer,

    #[nano]
    bpm: ParameterType,

    // A poor-man's semaphore that allows note-off events to overlap with the
    // current note without causing it to shut off. Example is a legato
    // playing-style of the MIDI instrument that controls the arpeggiator. If we
    // turned on and off solely by the last note-on/off we received, then the
    // arpeggiator would frequently get clipped.
    note_semaphore: i16,
}
impl IsController for Arpeggiator {}
impl Resets for Arpeggiator {}
impl TicksWithMessages for Arpeggiator {
    type Message = EntityMessage;

    fn tick(&mut self, tick_count: usize) -> (std::option::Option<Vec<Self::Message>>, usize) {
        self.sequencer.tick(tick_count)
    }
}
impl HandlesMidi for Arpeggiator {
    fn handle_midi_message(
        &mut self,
        message: &MidiMessage,
    ) -> Option<Vec<(MidiChannel, MidiMessage)>> {
        match message {
            MidiMessage::NoteOff { key: _, vel: _ } => {
                self.note_semaphore -= 1;
                if self.note_semaphore < 0 {
                    self.note_semaphore = 0;
                }
                self.sequencer.enable(self.note_semaphore > 0);
            }
            MidiMessage::NoteOn { key, vel } => {
                self.note_semaphore += 1;
                self.rebuild_sequence(key.as_int(), vel.as_int());
                self.sequencer.enable(true);

                // TODO: this scratches the itch of needing to respond
                // to a note-down with a note *during this slice*, but
                // it also has an edge condition where we need to cancel
                // a different note that was might have been supposed to
                // be sent instead during this slice, or at least
                // immediately shut it off. This seems to require a
                // two-phase Tick handler (one to decide what we're
                // going to send, and another to send it), and an
                // internal memory of which notes we've asked the
                // downstream to play. TODO TODO TODO
                return self.sequencer.generate_midi_messages_for_current_frame();
            }
            MidiMessage::Aftertouch { key: _, vel: _ } => todo!(),
            MidiMessage::Controller {
                controller: _,
                value: _,
            } => todo!(),
            MidiMessage::ProgramChange { program: _ } => todo!(),
            MidiMessage::ChannelAftertouch { vel: _ } => todo!(),
            MidiMessage::PitchBend { bend: _ } => todo!(),
        }
        None
    }
}
impl Performs for Arpeggiator {
    fn play(&mut self) {
        self.sequencer.play();
    }

    fn stop(&mut self) {
        self.sequencer.stop();
    }

    fn skip_to_start(&mut self) {
        self.sequencer.skip_to_start();
    }
}

impl Arpeggiator {
    pub fn new_with(midi_channel_out: MidiChannel, params: ArpeggiatorNano) -> Self {
        Self {
            uid: Default::default(),
            midi_channel_out,
            bpm: params.bpm,
            sequencer: Sequencer::new_with(super::SequencerNano { bpm: params.bpm() }),
            note_semaphore: Default::default(),
        }
    }

    fn insert_one_note(
        &mut self,
        when: PerfectTimeUnit,
        duration: PerfectTimeUnit,
        key: u8,
        vel: u8,
    ) {
        self.sequencer
            .insert(when, self.midi_channel_out, new_note_on(key, vel));
        self.sequencer
            .insert(when + duration, self.midi_channel_out, new_note_off(key, 0));
    }

    fn rebuild_sequence(&mut self, key: u8, vel: u8) {
        self.sequencer.clear();

        // TODO: this is a good place to start pulling the f32 time thread --
        // remove that ".into()" and deal with it
        let start_beat = PerfectTimeUnit(self.sequencer.cursor_in_beats());
        self.insert_one_note(
            start_beat + PerfectTimeUnit(0.25 * 0.0),
            PerfectTimeUnit(0.25),
            key,
            vel,
        );
        self.insert_one_note(
            start_beat + PerfectTimeUnit(0.25 * 1.0),
            PerfectTimeUnit(0.25),
            key + 2,
            vel,
        );
        self.insert_one_note(
            start_beat + PerfectTimeUnit(0.25 * 2.0),
            PerfectTimeUnit(0.25),
            key + 4,
            vel,
        );
        self.insert_one_note(
            start_beat + PerfectTimeUnit(0.25 * 3.0),
            PerfectTimeUnit(0.25),
            key + 5,
            vel,
        );
        self.insert_one_note(
            start_beat + PerfectTimeUnit(0.25 * 4.0),
            PerfectTimeUnit(0.25),
            key + 7,
            vel,
        );
        self.insert_one_note(
            start_beat + PerfectTimeUnit(0.25 * 5.0),
            PerfectTimeUnit(0.25),
            key + 9,
            vel,
        );
        self.insert_one_note(
            start_beat + PerfectTimeUnit(0.25 * 6.0),
            PerfectTimeUnit(0.25),
            key + 11,
            vel,
        );
    }

    pub fn update(&mut self, message: ArpeggiatorMessage) {
        match message {
            ArpeggiatorMessage::Arpeggiator(s) => *self = Self::new_with(self.midi_channel_out, s),
            _ => self.derived_update(message),
        }
    }

    pub fn bpm(&self) -> f64 {
        self.bpm
    }

    pub fn set_bpm(&mut self, bpm: ParameterType) {
        self.bpm = bpm;
    }
}
