use crate::{
    clock::Clock,
    common::{rrc, Rrc, Ww},
    midi::{MidiChannel, MidiMessage, MidiMessageType, MidiNote},
    traits::{IsMidiEffect, SinksMidi, SourcesMidi, Terminates, WatchesClock},
};
use std::{collections::HashMap, rc::Rc};

#[derive(Debug, Default)]
pub struct Arpeggiator {
    pub(crate) me: Ww<Self>,
    midi_channel_in: MidiChannel,
    midi_channel_out: MidiChannel,
    midi_sinks: HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>>,

    is_device_playing: bool,

    note: MidiNote,
    note_addition: u8,
    is_note_playing: bool,

    next_beat: f32,
}

impl SinksMidi for Arpeggiator {
    fn midi_channel(&self) -> MidiChannel {
        self.midi_channel_in
    }

    fn handle_midi_for_channel(&mut self, _clock: &Clock, message: &MidiMessage) {
        // TODO: we'll need clock to do cool things like schedule note change on next bar... maybe
        match message.status {
            MidiMessageType::NoteOn => {
                self.is_device_playing = true;
                self.note = MidiNote::C4
            }
            MidiMessageType::NoteOff => self.is_device_playing = false,
            MidiMessageType::ProgramChange => todo!(),
        }
    }

    fn set_midi_channel(&mut self, midi_channel: MidiChannel) {
        self.midi_channel_in = midi_channel;
    }
}

impl SourcesMidi for Arpeggiator {
    fn midi_sinks_mut(&mut self) -> &mut HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>> {
        &mut self.midi_sinks
    }

    fn midi_sinks(&self) -> &HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>> {
        &self.midi_sinks
    }

    fn midi_output_channel(&self) -> MidiChannel {
        self.midi_channel_out
    }

    fn set_midi_output_channel(&mut self, midi_channel: MidiChannel) {
        self.midi_channel_out = midi_channel;
    }
}

impl WatchesClock for Arpeggiator {
    fn tick(&mut self, clock: &Clock) {
        if clock.beats() >= self.next_beat {
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
                // TODO duh
                self.note_addition = if self.note_addition == 0 { 7 + 12 } else { 0 }
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
    }
}

impl Terminates for Arpeggiator {
    fn is_finished(&self) -> bool {
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

    pub fn new_wrapped_with(
        midi_channel_in: MidiChannel,
        midi_channel_out: MidiChannel,
    ) -> Rrc<Self> {
        let wrapped = rrc(Self::new_with(midi_channel_in, midi_channel_out));
        wrapped.borrow_mut().me = Rc::downgrade(&wrapped);
        wrapped
    }

    // TODO: placeholder for a bunch of knobs and dials
    pub(crate) fn nothing(&self) -> f32 {
        0.0
    }

    // this is a placeholder to get the trait requirements satisfied
    pub(crate) fn set_nothing(&mut self, _value: f32) {}
}
