use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SynthPatch {
    pub name: String,
    pub oscillator_1: OscillatorSettings,
    pub oscillator_2: OscillatorSettings,
    pub oscillator_2_track: bool,
    pub oscillator_2_sync: bool,

    pub noise: f32,

    pub lfo: LfoPreset,

    pub glide: GlideSettings,
    pub has_unison: bool,
    pub polyphony: PolyphonySettings,

    // There is meant to be only one filter, but the Welsh book
    // provides alternate settings depending on the kind of filter
    // your synthesizer has.
    pub filter_type_24db: FilterPreset,
    pub filter_type_12db: FilterPreset,
    pub filter_resonance: f32, // This should be an appropriate interpretation of a linear 0..1
    pub filter_envelope_weight: f32,
    pub filter_envelope: EnvelopeSettings,

    pub amp_envelope: EnvelopeSettings,
}

#[derive(PartialEq, Copy, Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WaveformType {
    None,
    #[default]
    Sine,
    Square,
    PulseWidth(f32),
    Triangle,
    Sawtooth,
    Noise,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum GlideSettings {
    #[default]
    Off,
    On(f32),
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PolyphonySettings {
    #[default]
    Multi,
    Mono,
    MultiLimit(u8),
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct OscillatorSettings {
    pub waveform: WaveformType,
    pub tune: f32,
    pub mix: f32,
}

impl Default for OscillatorSettings {
    fn default() -> Self {
        Self {
            waveform: WaveformType::default(),
            tune: OscillatorSettings::NATURAL_TUNING,
            mix: OscillatorSettings::FULL_MIX,
        }
    }
}

impl OscillatorSettings {
    pub const NATURAL_TUNING: f32 = 1.0; // tune field
    pub const FULL_MIX: f32 = 1.0; // mix field

    #[allow(dead_code)]
    pub fn octaves(num: f32) -> f32 {
        Self::semis_and_cents(num * 12.0, 0.0)
    }

    #[allow(dead_code)]
    pub fn semis_and_cents(semitones: f32, cents: f32) -> f32 {
        // https://en.wikipedia.org/wiki/Cent_(music)
        2.0f32.powf((semitones * 100.0 + cents) / 1200.0)
    }
}

// attack/decay/release are in time units.
// sustain is a 0..=1 percentage.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct EnvelopeSettings {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

impl Default for EnvelopeSettings {
    fn default() -> Self {
        Self {
            attack: 0.0,
            decay: 0.0,
            sustain: 1.0,
            release: 0.0,
        }
    }
}

impl EnvelopeSettings {
    #[allow(dead_code)]
    pub const MAX: f32 = 10000.0; // TODO: what exactly does Welsh mean by "max"?
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LfoRouting {
    #[default]
    None,
    Amplitude,
    Pitch,
    PulseWidth,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct LfoPreset {
    pub routing: LfoRouting,
    pub waveform: WaveformType,
    pub frequency: f32,
    pub depth: f32,
}

impl LfoPreset {
    #[allow(dead_code)]
    pub fn percent(num: f32) -> f32 {
        num / 100.0
    }

    #[allow(dead_code)]
    pub fn semis_and_cents(semitones: f32, cents: f32) -> f32 {
        // https://en.wikipedia.org/wiki/Cent_(music)
        2.0f32.powf((semitones * 100.0 + cents) / 1200.0)
    }
}

// TODO: for Welsh presets, it's understood that they're all low-pass filters.
// Thus we can use defaults cutoff 0.0 and weight 0.0 as a hack for a passthrough.
// Eventually we'll want this preset to be richer, and then we'll need an explicit
// notion of a None filter type.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
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
        settings::patches::OscillatorSettings,
        traits::SinksMidi,
    };

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
                MidiMessageType::Controller => todo!(),
            }
        }
    }

    #[test]
    fn test_oscillator_tuning_helpers() {
        assert_eq!(OscillatorSettings::NATURAL_TUNING, 1.0);

        // tune
        assert_eq!(OscillatorSettings::octaves(0.0), 1.0);
        assert_eq!(OscillatorSettings::octaves(1.0), 2.0);
        assert_eq!(OscillatorSettings::octaves(-1.0), 0.5);
        assert_eq!(OscillatorSettings::octaves(2.0), 4.0);
        assert_eq!(OscillatorSettings::octaves(-2.0), 0.25);

        assert_eq!(OscillatorSettings::semis_and_cents(0.0, 0.0), 1.0);
        assert_eq!(OscillatorSettings::semis_and_cents(12.0, 0.0), 2.0);
        assert_approx_eq!(OscillatorSettings::semis_and_cents(5.0, 0.0), 1.334_839_6); // 349.2282÷261.6256, F4÷C4
        assert_eq!(
            OscillatorSettings::semis_and_cents(0.0, -100.0),
            2.0f32.powf(-100.0 / 1200.0)
        );

        assert_eq!(
            OscillatorSettings::octaves(0.5),
            OscillatorSettings::semis_and_cents(6.0, 0.0)
        );
        assert_eq!(
            OscillatorSettings::octaves(1.0),
            OscillatorSettings::semis_and_cents(0.0, 1200.0)
        );
        assert_eq!(
            OscillatorSettings::semis_and_cents(1.0, 0.0),
            OscillatorSettings::semis_and_cents(0.0, 100.0)
        );
    }
}
