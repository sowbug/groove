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
    pub unison: bool,
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

    TriangleSine, // TODO
}

pub type GlideSettings = f32;

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
pub enum OscillatorTune {
    Note(u8),
    Float(f32),
    Osc { octave: i8, semi: i8, cent: i8 },
}

impl From<OscillatorTune> for f32 {
    fn from(val: OscillatorTune) -> Self {
        match val {
            OscillatorTune::Note(_) => 1.0,
            OscillatorTune::Float(value) => value,
            OscillatorTune::Osc { octave, semi, cent } => {
                OscillatorSettings::semis_and_cents(octave as i16 * 12 + semi as i16, cent as f64)
                    as f32
            }
        }
    }
}

impl From<OscillatorTune> for f64 {
    fn from(val: OscillatorTune) -> Self {
        match val {
            OscillatorTune::Note(_) => 1.0,
            OscillatorTune::Float(value) => value as f64,
            OscillatorTune::Osc { octave, semi, cent } => {
                OscillatorSettings::semis_and_cents(octave as i16 * 12 + semi as i16, cent as f64)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct OscillatorSettings {
    pub waveform: WaveformType,
    pub tune: OscillatorTune,

    #[serde(rename = "mix-pct")]
    pub mix: f32,
}

impl Default for OscillatorSettings {
    fn default() -> Self {
        Self {
            waveform: WaveformType::default(),
            tune: OscillatorTune::Osc {
                octave: 0,
                semi: 0,
                cent: 0,
            },
            mix: 1.0,
        }
    }
}

impl OscillatorSettings {
    #[allow(dead_code)]
    pub fn octaves(num: i16) -> f64 {
        Self::semis_and_cents(num * 12, 0.0)
    }

    pub fn semis_and_cents(semitones: i16, cents: f64) -> f64 {
        // https://en.wikipedia.org/wiki/Cent_(music)
        2.0f64.powf((semitones as f64 * 100.0 + cents) / 1200.0)
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
    FilterCutoff,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LfoDepth {
    None,
    Pct(f32),
    Cents(f32),
}
impl Default for LfoDepth {
    fn default() -> Self {
        LfoDepth::Pct(0.0)
    }
}
impl From<LfoDepth> for f32 {
    fn from(val: LfoDepth) -> Self {
        match val {
            LfoDepth::None => 0.0,
            LfoDepth::Pct(pct) => pct,
            LfoDepth::Cents(cents) => {
                1.0 - OscillatorSettings::semis_and_cents(0, cents as f64) as f32
            }
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct LfoPreset {
    pub routing: LfoRouting,
    pub waveform: WaveformType,
    pub frequency: f32,
    pub depth: LfoDepth,
}

// TODO: for Welsh presets, it's understood that they're all low-pass filters.
// Thus we can use defaults cutoff 0.0 and weight 0.0 as a hack for a passthrough.
// Eventually we'll want this preset to be richer, and then we'll need an explicit
// notion of a None filter type.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct FilterPreset {
    pub cutoff_hz: f32,
    pub cutoff_pct: f32,
}

#[cfg(test)]
mod tests {
    use crate::settings::patches::OscillatorSettings;
    use assert_approx_eq::assert_approx_eq;

    #[test]
    fn test_oscillator_tuning_helpers() {
        // tune
        assert_eq!(OscillatorSettings::octaves(0), 1.0);
        assert_eq!(OscillatorSettings::octaves(1), 2.0);
        assert_eq!(OscillatorSettings::octaves(-1), 0.5);
        assert_eq!(OscillatorSettings::octaves(2), 4.0);
        assert_eq!(OscillatorSettings::octaves(-2), 0.25);

        assert_eq!(OscillatorSettings::semis_and_cents(0, 0.0), 1.0);
        assert_eq!(OscillatorSettings::semis_and_cents(12, 0.0), 2.0);
        assert_approx_eq!(OscillatorSettings::semis_and_cents(5, 0.0), 1.334_839_6); // 349.2282÷261.6256, F4÷C4
        assert_eq!(
            OscillatorSettings::semis_and_cents(0, -100.0),
            2.0f64.powf(-100.0 / 1200.0)
        );

        assert_eq!(
            OscillatorSettings::octaves(1),
            OscillatorSettings::semis_and_cents(12, 0.0)
        );
        assert_eq!(
            OscillatorSettings::octaves(1),
            OscillatorSettings::semis_and_cents(0, 1200.0)
        );
        assert_eq!(
            OscillatorSettings::semis_and_cents(1, 0.0),
            OscillatorSettings::semis_and_cents(0, 100.0)
        );
    }
}
