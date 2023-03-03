use crate::{
    effects::{BiQuadFilter, FilterParams},
    instruments::{
        envelopes::EnvelopeGenerator,
        oscillators::{Oscillator, Waveform},
        welsh::{LfoRouting, WelshVoice},
        StealingVoiceStore, Synthesizer, WelshSynth,
    },
    utils::Paths,
};
use convert_case::{Boundary, Case, Casing};
use groove_core::{midi::note_to_frequency, BipolarNormal, Normal, ParameterType};
use serde::{Deserialize, Serialize};

use super::LoadError;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct WelshPatchSettings {
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

// TODO: cache these as they're loaded
impl WelshPatchSettings {
    pub fn patch_name_to_settings_name(name: &str) -> String {
        name.from_case(Case::Camel)
            .without_boundaries(&[Boundary::DigitLower])
            .to_case(Case::Kebab)
    }

    pub fn new_from_yaml(yaml: &str) -> Result<Self, LoadError> {
        serde_yaml::from_str(yaml).map_err(|e| {
            println!("{e}");
            LoadError::FormatError
        })
    }

    pub fn by_name(name: &str) -> Self {
        let mut filename = Paths::asset_path();
        filename.push("patches");
        filename.push("welsh");
        filename.push(format!(
            "{}.yaml",
            Self::patch_name_to_settings_name(name.to_string().as_str())
        ));
        if let Ok(contents) = std::fs::read_to_string(&filename) {
            match Self::new_from_yaml(&contents) {
                Ok(patch) => patch,
                Err(err) => {
                    // TODO: this should return a failsafe patch, maybe a boring
                    // square wave
                    panic!("couldn't parse patch file: {err:?}");
                }
            }
        } else {
            panic!("couldn't read patch file named {:?}", &filename);
        }
    }

    pub fn into_welsh_voice(&self, sample_rate: usize) -> WelshVoice {
        let mut oscillators = Vec::default();
        let mut oscillator_2_sync = false;
        if !matches!(self.oscillator_1.waveform, WaveformType::None) {
            oscillators.push(self.oscillator_1.into_with(sample_rate));
        }
        if !matches!(self.oscillator_2.waveform, WaveformType::None) {
            let mut o = self.oscillator_2.into_with(sample_rate);
            if !self.oscillator_2_track {
                if let crate::settings::patches::OscillatorTune::Note(note) = self.oscillator_2.tune
                {
                    o.set_fixed_frequency(note_to_frequency(note));
                } else {
                    panic!("Patch configured without oscillator 2 tracking, but tune is not a note specification");
                }
            }
            oscillator_2_sync = self.oscillator_2_sync;
            oscillators.push(o);
        }
        if self.noise > 0.0 {
            oscillators.push(Oscillator::new_with_waveform(sample_rate, Waveform::Noise));
        }

        let oscillator_mix = if oscillators.len() == 0 {
            0.0
        } else if oscillators.len() == 1 {
            1.0
        } else if self.oscillator_1.mix == 0.0 && self.oscillator_2.mix == 0.0 {
            1.0
        } else {
            let total = self.oscillator_1.mix + self.oscillator_2.mix;
            (self.oscillator_1.mix / total) as f64
        };

        //        WelshVoice::new_with(oscillators, oscillator_mix, oscillator_2_sync, )

        let amp_envelope = self.amp_envelope.into_with(sample_rate);
        let lfo = self.lfo.into_with(sample_rate);
        let lfo_routing = self.lfo.routing.into();
        let lfo_depth = self.lfo.depth.into();
        let filter = BiQuadFilter::new_with(
            &FilterParams::LowPass12db {
                cutoff: self.filter_type_12db.cutoff_hz,
                q: BiQuadFilter::denormalize_q(self.filter_resonance),
            },
            sample_rate,
        );
        let filter_cutoff_start =
            BiQuadFilter::frequency_to_percent(self.filter_type_12db.cutoff_hz);
        let filter_cutoff_end = self.filter_envelope_weight;
        let filter_envelope = self.filter_envelope.into_with(sample_rate);

        WelshVoice::new_with(
            oscillators,
            oscillator_2_sync,
            oscillator_mix,
            amp_envelope,
            filter,
            filter_cutoff_start,
            filter_cutoff_end,
            filter_envelope,
            lfo,
            lfo_routing,
            lfo_depth,
        )
    }

    pub fn into_welsh_synth(&self, sample_rate: usize) -> WelshSynth {
        let mut voice_store = Box::new(StealingVoiceStore::<WelshVoice>::new_with(sample_rate));
        for _ in 0..8 {
            voice_store.add_voice(Box::new(self.into_welsh_voice(sample_rate)));
        }
        WelshSynth::new_with(Synthesizer::<WelshVoice>::new_with(
            sample_rate,
            voice_store,
        ))
    }
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
    DebugZero,
    DebugMax,
    DebugMin,

    TriangleSine, // TODO
}
impl Into<Waveform> for WaveformType {
    fn into(self) -> Waveform {
        match self {
            WaveformType::None => Waveform::Sine,
            WaveformType::Sine => Waveform::Sine,
            WaveformType::Square => Waveform::Square,
            WaveformType::PulseWidth(pct) => Waveform::PulseWidth(pct),
            WaveformType::Triangle => Waveform::Triangle,
            WaveformType::Sawtooth => Waveform::Sawtooth,
            WaveformType::Noise => Waveform::Noise,
            WaveformType::DebugZero => Waveform::DebugZero,
            WaveformType::DebugMax => Waveform::DebugMax,
            WaveformType::DebugMin => Waveform::DebugMin,
            WaveformType::TriangleSine => Waveform::TriangleSine,
        }
    }
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
impl From<OscillatorTune> for f32 {
    fn from(val: OscillatorTune) -> Self {
        let r: f64 = val.into();
        r as f32
    }
}
impl From<OscillatorTune> for BipolarNormal {
    fn from(val: OscillatorTune) -> Self {
        let r: f64 = val.into();
        BipolarNormal::new(r)
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

    pub fn into_with(&self, sample_rate: usize) -> Oscillator {
        Oscillator::new_with_waveform(sample_rate, self.waveform.into())
    }
}

// attack/decay/release are in time units.
// sustain is a 0..=1 percentage.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct EnvelopeSettings {
    pub attack: ParameterType,
    pub decay: ParameterType,
    pub sustain: ParameterType, // TODO: this should be a Normal
    pub release: ParameterType,
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
    pub const MAX: f64 = 10000.0; // TODO: what exactly does Welsh mean by "max"?

    pub fn into_with(&self, sample_rate: usize) -> EnvelopeGenerator {
        EnvelopeGenerator::new_with(
            sample_rate,
            self.attack,
            self.decay,
            Normal::new(self.sustain),
            self.release,
        )
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LfoRoutingType {
    #[default]
    None,
    Amplitude,
    Pitch,
    PulseWidth,
    FilterCutoff,
}
impl Into<LfoRouting> for LfoRoutingType {
    fn into(self) -> LfoRouting {
        match self {
            LfoRoutingType::None => LfoRouting::None,
            LfoRoutingType::Amplitude => LfoRouting::Amplitude,
            LfoRoutingType::Pitch => LfoRouting::Pitch,
            LfoRoutingType::PulseWidth => LfoRouting::PulseWidth,
            LfoRoutingType::FilterCutoff => LfoRouting::FilterCutoff,
        }
    }
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
impl From<LfoDepth> for Normal {
    fn from(val: LfoDepth) -> Self {
        match val {
            LfoDepth::None => Normal::minimum(),
            LfoDepth::Pct(pct) => Normal::new(pct as f64),
            LfoDepth::Cents(cents) => {
                Normal::new(1.0 - OscillatorSettings::semis_and_cents(0, cents as f64))
            }
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct LfoPreset {
    pub routing: LfoRoutingType,
    pub waveform: WaveformType,
    pub frequency: f32,
    pub depth: LfoDepth,
}
impl LfoPreset {
    pub fn into_with(&self, sample_rate: usize) -> Oscillator {
        Oscillator::new_with_waveform_and_frequency(
            sample_rate,
            self.waveform.into(),
            self.frequency as ParameterType,
        )
    }
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
    use crate::{
        common::DEFAULT_SAMPLE_RATE, instruments::welsh::WelshVoice,
        settings::patches::OscillatorSettings, utils::tests::canonicalize_filename, Clock,
    };
    use assert_approx_eq::assert_approx_eq;
    use groove_core::{
        traits::{Generates, PlaysNotes, Ticks},
        SampleType, StereoSample,
    };

    use super::{
        EnvelopeSettings, FilterPreset, LfoPreset, LfoRoutingType, PolyphonySettings, WaveformType,
        WelshPatchSettings,
    };

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

    // TODO: get rid of this
    fn write_sound(
        source: &mut WelshVoice,
        clock: &mut Clock,
        duration: f64,
        when: f64,
        basename: &str,
    ) {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: clock.sample_rate() as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        const AMPLITUDE: SampleType = i16::MAX as SampleType;
        let mut writer = hound::WavWriter::create(canonicalize_filename(basename), spec).unwrap();

        let mut is_message_sent = false;
        while clock.seconds() < duration {
            if when <= clock.seconds() && !is_message_sent {
                is_message_sent = true;
                source.note_off(0);
            }
            source.tick(1);
            let sample = source.value();
            let _ = writer.write_sample((sample.0 .0 * AMPLITUDE) as i16);
            let _ = writer.write_sample((sample.1 .0 * AMPLITUDE) as i16);
            clock.tick(1);
        }
    }

    fn cello_patch() -> WelshPatchSettings {
        WelshPatchSettings {
            name: WelshPatchSettings::patch_name_to_settings_name("Cello"),
            oscillator_1: OscillatorSettings {
                waveform: WaveformType::PulseWidth(0.1),
                ..Default::default()
            },
            oscillator_2: OscillatorSettings {
                waveform: WaveformType::Square,
                ..Default::default()
            },
            oscillator_2_track: true,
            oscillator_2_sync: false,
            noise: 0.0,
            lfo: LfoPreset {
                routing: LfoRoutingType::Amplitude,
                waveform: WaveformType::Sine,
                frequency: 7.5,
                depth: crate::settings::patches::LfoDepth::Pct(0.05),
            },
            glide: 0.0,
            unison: false,
            polyphony: PolyphonySettings::Multi,
            filter_type_24db: FilterPreset {
                cutoff_hz: 40.0,
                cutoff_pct: 0.1,
            },
            filter_type_12db: FilterPreset {
                cutoff_hz: 40.0,
                cutoff_pct: 0.1,
            },
            filter_resonance: 0.0,
            filter_envelope_weight: 0.9,
            filter_envelope: EnvelopeSettings {
                attack: 0.0,
                decay: 3.29,
                sustain: 0.78,
                release: EnvelopeSettings::MAX,
            },
            amp_envelope: EnvelopeSettings {
                attack: 0.06,
                decay: EnvelopeSettings::MAX,
                sustain: 1.0,
                release: 0.3,
            },
        }
    }

    fn test_patch() -> WelshPatchSettings {
        WelshPatchSettings {
            name: WelshPatchSettings::patch_name_to_settings_name("Test"),
            oscillator_1: OscillatorSettings {
                waveform: WaveformType::Sawtooth,
                ..Default::default()
            },
            oscillator_2: OscillatorSettings {
                waveform: WaveformType::None,
                ..Default::default()
            },
            oscillator_2_track: true,
            oscillator_2_sync: false,
            noise: 0.0,
            lfo: LfoPreset {
                routing: LfoRoutingType::None,
                ..Default::default()
            },
            glide: 0.0,
            unison: false,
            polyphony: PolyphonySettings::Multi,
            filter_type_24db: FilterPreset {
                cutoff_hz: 40.0,
                cutoff_pct: 0.1,
            },
            filter_type_12db: FilterPreset {
                cutoff_hz: 20.0,
                cutoff_pct: 0.05,
            },
            filter_resonance: 0.0,
            filter_envelope_weight: 1.0,
            filter_envelope: EnvelopeSettings {
                attack: 5.0,
                decay: EnvelopeSettings::MAX,
                sustain: 1.0,
                release: EnvelopeSettings::MAX,
            },
            amp_envelope: EnvelopeSettings {
                attack: 0.5,
                decay: EnvelopeSettings::MAX,
                sustain: 1.0,
                release: EnvelopeSettings::MAX,
            },
        }
    }

    #[test]
    fn welsh_makes_any_sound_at_all() {
        let mut voice = test_patch().into_welsh_voice(DEFAULT_SAMPLE_RATE);
        voice.note_on(60, 127);

        // Skip a few frames in case attack is slow
        voice.tick(5);
        assert!(
            voice.value() != StereoSample::SILENCE,
            "once triggered, voice should make a sound"
        );
    }

    #[test]
    fn basic_synth_patch() {
        let mut clock = Clock::new_test();
        let mut voice = test_patch().into_welsh_voice(clock.sample_rate());
        voice.note_on(60, 127);
        voice.tick(1);
        write_sound(&mut voice, &mut clock, 5.0, 5.0, "voice_basic_test_c4");
    }

    #[test]
    fn basic_cello_patch() {
        let mut clock = Clock::new_test();
        let mut voice = cello_patch().into_welsh_voice(clock.sample_rate());
        voice.note_on(60, 127);
        voice.tick(1);
        write_sound(&mut voice, &mut clock, 5.0, 3.0, "voice_cello_c4");
    }
}
