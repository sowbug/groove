use crate::primitives::oscillators::Waveform;

pub mod welsh;

#[derive(Debug, Clone, Copy)]
pub struct OscillatorPreset {
    pub waveform: Waveform,
    pub tune: f32,
    pub mix: f32,
}

impl OscillatorPreset {
    pub fn octaves(num: f32) -> f32 {
        return 1.0 + num;
    }
    pub fn semis_and_cents(semitones: f32, cents: f32) -> f32 {
        // https://en.wikipedia.org/wiki/Cent_(music)
        2.0f32.powf((semitones * 100.0 + cents) / 1200.0)
    }
}

impl Default for OscillatorPreset {
    fn default() -> Self {
        Self {
            waveform: Waveform::None,
            tune: 1.0,
            mix: 1.0,
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct EnvelopePreset {
    pub attack_seconds: f32,
    pub decay_seconds: f32,
    pub sustain_percentage: f32,
    pub release_seconds: f32,
}

#[derive(Debug, Clone, Copy)]
pub enum LfoRouting {
    None,
    Amplitude,
    Pitch,
}

impl Default for LfoRouting {
    fn default() -> Self {
        LfoRouting::None
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LfoPreset {
    pub routing: LfoRouting,
    pub waveform: Waveform,
    pub frequency: f32,
    pub depth: f32,
}

impl LfoPreset {
    pub fn percent(num: f32) -> f32 {
        num / 100.0
    }

    pub(crate) fn cents(arg: f64) -> f32 {
        todo!()
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct FilterPreset {
    pub cutoff: f32,
    pub weight: f32,
}

#[cfg(test)]
mod tests {
    use crate::{
        common::{MidiMessage, MidiMessageType},
        devices::traits::DeviceTrait,
        primitives::clock::Clock,
    };

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
