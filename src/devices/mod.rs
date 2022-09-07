mod automation;
pub mod effects;
pub mod midi;
mod mixer;
pub mod orchestrator;
pub mod sequencer;
pub mod traits; // TODO; make non-pub again so DeviceTrait doesn't leak out of this crate

#[cfg(test)]
mod tests {
    use crate::{
        common::{MidiMessage, MidiMessageType, MonoSample},
        primitives::clock::Clock,
    };

    use super::traits::DeviceTrait;

    #[derive(Default)]
    pub struct NullDevice {
        pub is_playing: bool,
        midi_channel: u8,
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
        pub fn set_channel(&mut self, channel: u8) {
            self.midi_channel = channel;
        }
        pub fn set_value(&mut self, value: f32) {
            self.value = value;
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
        fn handle_automation(&mut self, _param_name: &String, param_value: f32) {
            self.set_value(param_value);
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

    impl DeviceTrait for SingleLevelDevice {
        fn sources_audio(&self) -> bool {
            true
        }

        fn get_audio_sample(&mut self) -> MonoSample {
            self.level
        }
    }
}
