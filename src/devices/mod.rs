use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    common::{MidiChannel, MidiMessage, MidiNote},
    primitives::{clock::Clock, WatchesClock},
};

use self::traits::{AutomationMessage, AutomationSink, MidiSink, MidiSource};

mod automation;
pub mod effects;
pub mod midi;
mod mixer;
pub mod orchestrator;
pub mod patterns;
pub mod sequencer;
pub mod traits; // TODO; make non-pub again so DeviceTrait doesn't leak out of this crate

#[derive(Default)]
pub struct Arpeggiator {
    midi_channel_in: MidiChannel,
    midi_channel_out: MidiChannel,
    midi_sinks: HashMap<MidiChannel, Vec<Rc<RefCell<dyn MidiSink>>>>,

    is_device_playing: bool,

    note: MidiNote,
    note_addition: u8,
    is_note_playing: bool,

    next_beat: f32,
}

impl AutomationSink for Arpeggiator {
    fn handle_automation_message(&mut self, _message: &AutomationMessage) {
        todo!()
    }
}

impl MidiSink for Arpeggiator {
    fn midi_channel(&self) -> MidiChannel {
        self.midi_channel_in
    }

    fn handle_message_for_channel(&mut self, _clock: &Clock, message: &MidiMessage) {
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

impl MidiSource for Arpeggiator {
    fn midi_sinks(&mut self) -> &mut HashMap<MidiChannel, Vec<Rc<RefCell<dyn MidiSink>>>> {
        &mut self.midi_sinks
    }
}

impl WatchesClock for Arpeggiator {
    fn tick(&mut self, clock: &Clock) -> bool {
        if clock.beats >= self.next_beat {
            self.next_beat += 1.0;
            if self.is_note_playing {
                self.broadcast_midi_message(
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
                self.broadcast_midi_message(
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

impl Arpeggiator {
    fn new(midi_channel_in: MidiChannel, midi_channel_out: MidiChannel) -> Self {
        Self {
            midi_channel_in,
            midi_channel_out,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        common::{MidiChannel, MidiMessage, MidiMessageType, MonoSample},
        primitives::{clock::Clock, SourcesAudio},
    };

    use super::traits::{AutomationMessage, AutomationSink, MidiSink};

    #[derive(Default)]
    pub struct NullDevice {
        pub is_playing: bool,
        midi_channel: MidiChannel,
        pub midi_messages_received: usize,
        pub midi_messages_handled: usize,
        pub value: f32,
    }

    impl NullDevice {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
        pub fn set_value(&mut self, value: &f32) {
            self.value = *value;
        }
    }
    impl MidiSink for NullDevice {
        fn midi_channel(&self) -> MidiChannel {
            self.midi_channel
        }

        fn set_midi_channel(&mut self, midi_channel: MidiChannel) {
            self.midi_channel = midi_channel;
        }
        fn handle_message_for_channel(&mut self, _clock: &Clock, message: &MidiMessage) {
            self.midi_messages_received += 1;

            match message.status {
                MidiMessageType::NoteOn => {
                    self.is_playing = true;
                    self.midi_messages_handled += 1;
                }
                MidiMessageType::NoteOff => {
                    self.is_playing = false;
                    self.midi_messages_handled += 1;
                }
                MidiMessageType::ProgramChange => {
                    self.midi_messages_handled += 1;
                }
            }
        }
    }
    impl AutomationSink for NullDevice {
        fn handle_automation_message(&mut self, message: &AutomationMessage) {
            match message {
                AutomationMessage::UpdatePrimaryValue { value } => {
                    self.set_value(value);
                }
                _ => todo!(),
            }
        }
    }

    pub struct SingleLevelDevice {
        level: MonoSample,
    }

    impl SingleLevelDevice {
        pub fn new(level: MonoSample) -> Self {
            Self { level }
        }
    }

    impl SourcesAudio for SingleLevelDevice {
        fn source_audio(&mut self, _clock: &Clock) -> MonoSample {
            self.level
        }
    }
}
