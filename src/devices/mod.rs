pub mod effects;
pub mod midi;
mod mixer;
pub mod orchestrator;
pub mod sequencer;
pub mod traits; // TODO; make non-pub again so DeviceTrait doesn't leak out of this crate

#[cfg(test)]
mod tests {
    use crate::{
        common::{MidiMessage, MidiMessageType},
        primitives::clock::Clock,
    };

    use super::traits::DeviceTrait;

    #[derive(Default)]
    pub struct NullDevice {
        pub is_playing: bool,
        midi_channel: u8,
        pub midi_messages_received: usize,
        pub midi_messages_handled: usize,
    }

    impl NullDevice {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
        pub fn set_channel(&mut self, channel: u8) {
            self.midi_channel = channel;
        }
    }
    impl DeviceTrait for NullDevice {
        fn sinks_midi(&self) -> bool {
            true
        }
        fn handle_midi_message(&mut self, message: &MidiMessage, _clock: &Clock) {
            self.midi_messages_received += 1;

            // TODO: be more efficient about this -- don't dispatch in the first place!
            if message.channel != self.midi_channel {
                return;
            }

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
}
