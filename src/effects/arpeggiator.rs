use std::{cell::RefCell, collections::HashMap, rc::Weak};

use crate::{
    common::{MidiChannel, MidiMessage, MidiNote},
    primitives::clock::Clock,
    traits::{IsMidiEffect, SinksControl, SinksControlParam, SinksMidi, SourcesMidi, WatchesClock},
};

#[derive(Debug, Default)]
pub struct Arpeggiator {
    midi_channel_in: MidiChannel,
    midi_channel_out: MidiChannel,
    midi_sinks: HashMap<MidiChannel, Vec<Weak<RefCell<dyn SinksMidi>>>>,

    is_device_playing: bool,

    note: MidiNote,
    note_addition: u8,
    is_note_playing: bool,

    next_beat: f32,
}

impl SinksControl for Arpeggiator {
    fn handle_control(&mut self, _clock: &Clock, _param: &SinksControlParam) {
        todo!()
    }
}

impl SinksMidi for Arpeggiator {
    fn midi_channel(&self) -> MidiChannel {
        self.midi_channel_in
    }

    fn handle_midi_for_channel(&mut self, _clock: &Clock, message: &MidiMessage) {
        // TODO: we'll need clock to do cool things like schedule note change on next bar... maybe
        match message.status {
            crate::common::MidiMessageType::NoteOn => {
                self.is_device_playing = true;
                self.note = MidiNote::C4
            }
            crate::common::MidiMessageType::NoteOff => self.is_device_playing = false,
            crate::common::MidiMessageType::ProgramChange => todo!(),
        }
    }

    fn set_midi_channel(&mut self, midi_channel: MidiChannel) {
        self.midi_channel_in = midi_channel;
    }
}

impl SourcesMidi for Arpeggiator {
    fn midi_sinks_mut(&mut self) -> &mut HashMap<MidiChannel, Vec<Weak<RefCell<dyn SinksMidi>>>> {
        &mut self.midi_sinks
    }

    fn midi_sinks(&self) -> &HashMap<MidiChannel, Vec<Weak<RefCell<dyn SinksMidi>>>> {
        &self.midi_sinks
    }
}

impl WatchesClock for Arpeggiator {
    fn tick(&mut self, clock: &Clock) -> bool {
        if clock.beats >= self.next_beat {
            self.next_beat += 1.0;
            if self.is_note_playing {
                self.issue_midi(
                    clock,
                    &MidiMessage::new_note_off(
                        self.midi_channel_out,
                        self.note as u8 + self.note_addition,
                        100,
                    ),
                );
                self.is_note_playing = false;
                self.note_addition = if self.note_addition == 0 { 7 } else { 0 }
            }
            if self.is_device_playing {
                self.issue_midi(
                    clock,
                    &MidiMessage::new_note_on(
                        self.midi_channel_out,
                        self.note as u8 + self.note_addition,
                        100,
                    ),
                );
                self.is_note_playing = true;
            }
        }
        true
    }
}

impl IsMidiEffect for Arpeggiator {}

impl Arpeggiator {
    pub fn new_with(midi_channel_in: MidiChannel, midi_channel_out: MidiChannel) -> Self {
        Self {
            midi_channel_in,
            midi_channel_out,
            ..Default::default()
        }
    }
}