mod automation;
pub mod effects;
pub mod midi;
mod mixer;
pub mod orchestrator;
pub mod patterns;
pub mod sequencer;
pub mod traits; // TODO; make non-pub again so DeviceTrait doesn't leak out of this crate

#[cfg(test)]
mod tests {
    use crate::{
        common::{MidiChannel, MidiMessage, MidiMessageType, MonoSample},
        primitives::clock::Clock,
    };

    use super::traits::{AudioSource, AutomationMessage, AutomationSink, MidiSink};

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
        fn handle_message(&mut self, message: &AutomationMessage) {
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

    impl AudioSource for SingleLevelDevice {
        fn sample(&mut self) -> MonoSample {
            self.level
        }
    }
}
