use crate::common::WaveformType;

#[derive(Debug, Clone, Copy)]
pub struct OscillatorPreset {
    pub waveform: WaveformType,
    pub tune: f32,
    pub mix: f32,
}

impl Default for OscillatorPreset {
    fn default() -> Self {
        Self {
            waveform: WaveformType::Sine,
            tune: OscillatorPreset::NATURAL_TUNING,
            mix: OscillatorPreset::FULL_MIX,
        }
    }
}

impl OscillatorPreset {
    pub const NATURAL_TUNING: f32 = 1.0; // tune field
    pub const FULL_MIX: f32 = 1.0; // mix field

    pub fn octaves(num: f32) -> f32 {
        Self::semis_and_cents(num * 12.0, 0.0)
    }

    pub fn semis_and_cents(semitones: f32, cents: f32) -> f32 {
        // https://en.wikipedia.org/wiki/Cent_(music)
        2.0f32.powf((semitones * 100.0 + cents) / 1200.0)
    }
}

// attack/decay/release are in time units.
// sustain is a 0..=1 percentage.
#[derive(Debug, Clone, Copy)]
pub struct EnvelopePreset {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

impl Default for EnvelopePreset {
    fn default() -> Self {
        Self {
            attack: 0.0,
            decay: 0.0,
            sustain: 1.0,
            release: 0.0,
        }
    }
}

impl EnvelopePreset {
    pub const MAX: f32 = 10000.0; // TODO: what exactly does Welsh mean by "max"?
}

#[derive(Debug, Clone, Copy)]
pub enum LfoRouting {
    None,
    Amplitude,
    Pitch,
    PulseWidth,
}

impl Default for LfoRouting {
    fn default() -> Self {
        LfoRouting::None
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LfoPreset {
    pub routing: LfoRouting,
    pub waveform: WaveformType,
    pub frequency: f32,
    pub depth: f32,
}

impl LfoPreset {
    pub fn percent(num: f32) -> f32 {
        num / 100.0
    }

    pub fn semis_and_cents(semitones: f32, cents: f32) -> f32 {
        // https://en.wikipedia.org/wiki/Cent_(music)
        2.0f32.powf((semitones * 100.0 + cents) / 1200.0)
    }
}

// TODO: for Welsh presets, it's understood that they're all low-pass filters.
// Thus we can use defaults cutoff 0.0 and weight 0.0 as a hack for a passthrough.
// Eventually we'll want this preset to be richer, and then we'll need an explicit
// notion of a None filter type.
#[derive(Debug, Default, Clone, Copy)]
pub struct FilterPreset {
    pub cutoff: f32,
    pub weight: f32, // TODO: this is unused because it's just another way to say cutoff
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;

    use crate::{
        clock::Clock,
        midi::{MidiChannel, MidiMessage, MidiMessageType},
        traits::SinksMidi,
    };

    use super::OscillatorPreset;

    #[derive(Debug, Default)]
    pub struct NullDevice {
        pub is_playing: bool,
        midi_channel: MidiChannel,
        pub midi_messages_received: usize,
        pub midi_messages_handled: usize,
    }

    impl NullDevice {
        #[allow(dead_code)]
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }

    impl SinksMidi for NullDevice {
        fn midi_channel(&self) -> MidiChannel {
            self.midi_channel
        }

        fn set_midi_channel(&mut self, midi_channel: MidiChannel) {
            self.midi_channel = midi_channel;
        }

        fn handle_midi_for_channel(&mut self, _clock: &Clock, message: &MidiMessage) {
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

    #[test]
    fn test_oscillator_tuning_helpers() {
        assert_eq!(OscillatorPreset::NATURAL_TUNING, 1.0);

        // tune
        assert_eq!(OscillatorPreset::octaves(0.0), 1.0);
        assert_eq!(OscillatorPreset::octaves(1.0), 2.0);
        assert_eq!(OscillatorPreset::octaves(-1.0), 0.5);
        assert_eq!(OscillatorPreset::octaves(2.0), 4.0);
        assert_eq!(OscillatorPreset::octaves(-2.0), 0.25);

        assert_eq!(OscillatorPreset::semis_and_cents(0.0, 0.0), 1.0);
        assert_eq!(OscillatorPreset::semis_and_cents(12.0, 0.0), 2.0);
        assert_approx_eq!(OscillatorPreset::semis_and_cents(5.0, 0.0), 1.334839557); // 349.2282÷261.6256, F4÷C4
        assert_eq!(
            OscillatorPreset::semis_and_cents(0.0, -100.0),
            2.0f32.powf(-100.0 / 1200.0)
        );

        assert_eq!(
            OscillatorPreset::octaves(0.5),
            OscillatorPreset::semis_and_cents(6.0, 0.0)
        );
        assert_eq!(
            OscillatorPreset::octaves(1.0),
            OscillatorPreset::semis_and_cents(0.0, 1200.0)
        );
        assert_eq!(
            OscillatorPreset::semis_and_cents(1.0, 0.0),
            OscillatorPreset::semis_and_cents(0.0, 100.0)
        );
    }
}
