// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::LoadError;
use convert_case::{Boundary, Case, Casing};
use ensnare::prelude::*;
use groove_core::{
    generators::{EnvelopeParams, Oscillator, OscillatorParams, Waveform},
    midi::{note_to_frequency, GeneralMidiProgram},
    DcaParams,
};
use groove_entities::{
    effects::{BiQuadFilter, BiQuadFilterLowPass24dbParams},
    instruments::{LfoRouting, WelshSynthParams, WelshVoiceParams},
};
use groove_utils::Paths;
use serde::{Deserialize, Serialize};
use std::{io::Read, path::Path};
use strum_macros::IntoStaticStr;

#[derive(Debug, Default, Serialize, Deserialize)]
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
    pub filter_envelope: EnvelopeParams,

    pub amp_envelope: EnvelopeParams,
}

// TODO: cache these as they're loaded
impl WelshPatchSettings {
    pub fn patch_name_to_settings_name(name: &str) -> String {
        name.from_case(Case::Camel)
            .without_boundaries(&[Boundary::DigitLower])
            .to_case(Case::Kebab)
    }

    pub fn new_from_json(json: &str) -> Result<Self, LoadError> {
        serde_json::from_str(json).map_err(|e| {
            println!("{e}");
            LoadError::FormatError
        })
    }

    pub fn by_name(paths: &Paths, name: &str) -> Self {
        let path = paths.build_patch(
            "welsh",
            Path::new(&format!("{}.json", Self::patch_name_to_settings_name(name))),
        );
        if let Ok(mut file) = paths.search_and_open(&path) {
            let mut contents = String::new();
            if let Ok(_bytes_read) = file.read_to_string(&mut contents) {
                match Self::new_from_json(&contents) {
                    Ok(patch) => {
                        return patch;
                    }
                    Err(err) => {
                        // TODO: this should return a failsafe patch, maybe a boring
                        // square wave
                        panic!("couldn't parse patch file named {:?}: {err:?}", &path);
                    }
                }
            }
        }
        panic!("couldn't read patch file named {:?}", &path);
    }

    pub fn derive_welsh_synth_params(&self) -> WelshSynthParams {
        let mut oscillators = Vec::default();
        if !matches!(self.oscillator_1.waveform, Waveform::None) {
            oscillators.push(self.oscillator_1.derive_oscillator());
        }
        if !matches!(self.oscillator_2.waveform, Waveform::None) {
            let mut o = self.oscillator_2.derive_oscillator();
            if !self.oscillator_2_track {
                if let OscillatorTune::Note(note) = self.oscillator_2.tune {
                    o.set_fixed_frequency(note_to_frequency(note));
                } else {
                    panic!("Patch configured without oscillator 2 tracking, but tune is not a note specification");
                }
            }
            oscillators.push(o);
        }
        if self.noise > 0.0 {
            oscillators.push(Oscillator::new_with(&OscillatorParams {
                waveform: Waveform::Noise,
                ..Default::default()
            }));
        }

        WelshSynthParams {
            voice: WelshVoiceParams {
                oscillator_1: OscillatorParams {
                    waveform: self.oscillator_1.waveform.into(),
                    frequency_tune: self.oscillator_1.tune.into(),
                    ..Default::default()
                },
                oscillator_2: OscillatorParams {
                    waveform: self.oscillator_2.waveform.into(),
                    frequency_tune: self.oscillator_2.tune.into(),
                    ..Default::default()
                },
                oscillator_2_sync: self.oscillator_2_sync,
                oscillator_mix: if oscillators.is_empty() {
                    Normal::zero()
                } else if oscillators.len() == 1
                    || (self.oscillator_1.mix == 0.0 && self.oscillator_2.mix == 0.0)
                {
                    Normal::maximum()
                } else {
                    let total = self.oscillator_1.mix + self.oscillator_2.mix;
                    Normal::from(self.oscillator_1.mix / total)
                },
                amp_envelope: EnvelopeParams {
                    attack: self.amp_envelope.attack(),
                    decay: self.amp_envelope.decay(),
                    sustain: self.amp_envelope.sustain(),
                    release: self.amp_envelope.decay(),
                },
                lfo: OscillatorParams {
                    waveform: self.lfo.waveform,
                    frequency: self.lfo.frequency.into(),
                    ..Default::default()
                },
                lfo_routing: self.lfo.routing.into(),
                lfo_depth: self.lfo.depth.into(),
                filter: BiQuadFilterLowPass24dbParams {
                    cutoff: self.filter_type_24db.cutoff_hz.into(),
                    passband_ripple: BiQuadFilter::denormalize_q(self.filter_resonance.into()),
                },
                filter_cutoff_start: FrequencyHz::frequency_to_percent(
                    self.filter_type_12db.cutoff_hz.into(),
                ),
                filter_cutoff_end: self.filter_envelope_weight.into(),
                filter_envelope: EnvelopeParams {
                    attack: self.filter_envelope.attack(),
                    decay: self.filter_envelope.decay(),
                    sustain: self.filter_envelope.sustain(),
                    release: self.filter_envelope.decay(),
                },
                dca: DcaParams {
                    gain: 1.0.into(),
                    pan: Default::default(),
                },
            },
            dca: DcaParams {
                gain: 1.0.into(),
                pan: Default::default(),
            },
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Deserialize, IntoStaticStr, PartialEq, Serialize)]
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

pub type GlideSettings = f32;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PolyphonySettings {
    #[default]
    Multi,
    Mono,
    MultiLimit(u8),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OscillatorTune {
    Note(u8),
    Float(ParameterType),
    Osc { octave: i8, semi: i8, cent: i8 },
}
impl From<OscillatorTune> for Ratio {
    fn from(val: OscillatorTune) -> Self {
        match val {
            OscillatorTune::Note(_) => Ratio::from(1.0),
            OscillatorTune::Float(value) => Ratio::from(value),
            OscillatorTune::Osc { octave, semi, cent } => {
                OscillatorSettings::semis_and_cents(octave as i16 * 12 + semi as i16, cent as f64)
            }
        }
    }
}
// impl From<OscillatorTune> for f32 {
//     fn from(val: OscillatorTune) -> Self {
//         let r: Ratio = val.into();
//         r.value() as f32
//     }
// }

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct OscillatorSettings {
    pub waveform: Waveform,
    pub tune: OscillatorTune,

    #[serde(rename = "mix-pct")]
    pub mix: f32,
}
impl Default for OscillatorSettings {
    fn default() -> Self {
        Self {
            waveform: Waveform::default(),
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
    pub fn octaves(num: i16) -> Ratio {
        Self::semis_and_cents(num * 12, 0.0)
    }

    pub fn semis_and_cents(semitones: i16, cents: f64) -> Ratio {
        // https://en.wikipedia.org/wiki/Cent_(music)
        Ratio::from(2.0f64.powf((semitones as f64 * 100.0 + cents) / 1200.0))
    }

    pub fn derive_oscillator(&self) -> Oscillator {
        let mut r = Oscillator::new_with(&OscillatorParams::default_with_waveform(
            self.waveform.into(),
        ));
        r.set_frequency_tune(self.tune.into());
        r
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LfoRoutingType {
    #[default]
    None,
    Amplitude,
    Pitch,
    PulseWidth,
    FilterCutoff,
}
#[allow(clippy::from_over_into)]
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
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
                Normal::new(1.0 - OscillatorSettings::semis_and_cents(0, cents as f64).value())
            }
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LfoPreset {
    pub routing: LfoRoutingType,
    pub waveform: Waveform,
    pub frequency: f32,
    pub depth: LfoDepth,
}

// TODO: for Welsh presets, it's understood that they're all low-pass filters.
// Thus we can use defaults cutoff 0.0 and weight 0.0 as a hack for a passthrough.
// Eventually we'll want this preset to be richer, and then we'll need an explicit
// notion of a None filter type.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FilterPreset {
    pub cutoff_hz: f32,
    pub cutoff_pct: f32,
}

impl WelshPatchSettings {
    #[allow(dead_code)]
    pub fn general_midi_preset(
        paths: &Paths,
        program: &GeneralMidiProgram,
    ) -> anyhow::Result<WelshPatchSettings> {
        let mut delegated = false;
        let preset = match program {
            GeneralMidiProgram::AcousticGrand => "Piano",
            GeneralMidiProgram::BrightAcoustic => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::ElectricGrand => "ElectricPiano",
            GeneralMidiProgram::HonkyTonk => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::ElectricPiano1 => "ElectricPiano",
            GeneralMidiProgram::ElectricPiano2 => "ElectricPiano",
            GeneralMidiProgram::Harpsichord => "Harpsichord",
            GeneralMidiProgram::Clav => "Clavichord",
            GeneralMidiProgram::Celesta => "Celeste",
            GeneralMidiProgram::Glockenspiel => "Glockenspiel",
            GeneralMidiProgram::MusicBox => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Vibraphone => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Marimba => "Marimba",
            GeneralMidiProgram::Xylophone => "Xylophone",
            GeneralMidiProgram::TubularBells => "Bell",
            GeneralMidiProgram::Dulcimer => "Dulcimer",
            GeneralMidiProgram::DrawbarOrgan => {
                "Organ" // TODO dup
            }
            GeneralMidiProgram::PercussiveOrgan => {
                "Organ" // TODO dup
            }
            GeneralMidiProgram::RockOrgan => {
                "Organ" // TODO dup
            }
            GeneralMidiProgram::ChurchOrgan => {
                "Organ" // TODO dup
            }
            GeneralMidiProgram::ReedOrgan => {
                "Organ" // TODO dup
            }
            GeneralMidiProgram::Accordion => "Accordion",
            GeneralMidiProgram::Harmonica => "Harmonica",
            GeneralMidiProgram::TangoAccordion => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::AcousticGuitarNylon => "GuitarAcoustic",
            GeneralMidiProgram::AcousticGuitarSteel => {
                "GuitarAcoustic" // TODO dup
            }
            GeneralMidiProgram::ElectricGuitarJazz => {
                "GuitarElectric" // TODO dup
            }
            GeneralMidiProgram::ElectricGuitarClean => {
                "GuitarElectric" // TODO dup
            }
            GeneralMidiProgram::ElectricGuitarMuted => {
                "GuitarElectric" // TODO dup
            }
            GeneralMidiProgram::OverdrivenGuitar => {
                "GuitarElectric" // TODO dup
            }
            GeneralMidiProgram::DistortionGuitar => {
                "GuitarElectric" // TODO dup
            }
            GeneralMidiProgram::GuitarHarmonics => {
                "GuitarElectric" // TODO dup
            }
            GeneralMidiProgram::AcousticBass => "DoubleBass",
            GeneralMidiProgram::ElectricBassFinger => "StandupBass",
            GeneralMidiProgram::ElectricBassPick => "AcidBass",
            GeneralMidiProgram::FretlessBass => {
                "DetroitBass" // TODO same?
            }
            GeneralMidiProgram::SlapBass1 => "FunkBass",
            GeneralMidiProgram::SlapBass2 => "FunkBass",
            GeneralMidiProgram::SynthBass1 => "DigitalBass",
            GeneralMidiProgram::SynthBass2 => "DigitalBass",
            GeneralMidiProgram::Violin => "Violin",
            GeneralMidiProgram::Viola => "Viola",
            GeneralMidiProgram::Cello => "Cello",
            GeneralMidiProgram::Contrabass => "Contrabassoon",
            GeneralMidiProgram::TremoloStrings => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::PizzicatoStrings => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::OrchestralHarp => "Harp",
            GeneralMidiProgram::Timpani => "Timpani",
            GeneralMidiProgram::StringEnsemble1 => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::StringEnsemble2 => {
                "StringsPwm" // TODO same?
            }
            GeneralMidiProgram::Synthstrings1 => "StringsPwm", // TODO same?

            GeneralMidiProgram::Synthstrings2 => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::ChoirAahs => "Angels",

            GeneralMidiProgram::VoiceOohs => "Choir",
            GeneralMidiProgram::SynthVoice => "VocalFemale",

            GeneralMidiProgram::OrchestraHit => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Trumpet => "Trumpet",
            GeneralMidiProgram::Trombone => "Trombone",
            GeneralMidiProgram::Tuba => "Tuba",
            GeneralMidiProgram::MutedTrumpet => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::FrenchHorn => "FrenchHorn",

            GeneralMidiProgram::BrassSection => "BrassSection",

            GeneralMidiProgram::Synthbrass1 => {
                "BrassSection" // TODO dup
            }
            GeneralMidiProgram::Synthbrass2 => {
                "BrassSection" // TODO dup
            }
            GeneralMidiProgram::SopranoSax => {
                "Saxophone" // TODO dup
            }
            GeneralMidiProgram::AltoSax => "Saxophone",
            GeneralMidiProgram::TenorSax => {
                "Saxophone" // TODO dup
            }
            GeneralMidiProgram::BaritoneSax => {
                "Saxophone" // TODO dup
            }
            GeneralMidiProgram::Oboe => "Oboe",
            GeneralMidiProgram::EnglishHorn => "EnglishHorn",
            GeneralMidiProgram::Bassoon => "Bassoon",
            GeneralMidiProgram::Clarinet => "Clarinet",
            GeneralMidiProgram::Piccolo => "Piccolo",
            GeneralMidiProgram::Flute => "Flute",
            GeneralMidiProgram::Recorder => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::PanFlute => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::BlownBottle => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Shakuhachi => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Whistle => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Ocarina => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Lead1Square => {
                "MonoSolo" // TODO: same?
            }
            GeneralMidiProgram::Lead2Sawtooth => {
                "Trance5th" // TODO: same?
            }
            GeneralMidiProgram::Lead3Calliope => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Lead4Chiff => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Lead5Charang => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Lead6Voice => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Lead7Fifths => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Lead8BassLead => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Pad1NewAge => {
                "NewAgeLead" // TODO pad or lead?
            }
            GeneralMidiProgram::Pad2Warm => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Pad3Polysynth => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Pad4Choir => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Pad5Bowed => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Pad6Metallic => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Pad7Halo => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Pad8Sweep => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Fx1Rain => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Fx2Soundtrack => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Fx3Crystal => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Fx4Atmosphere => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Fx5Brightness => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Fx6Goblins => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Fx7Echoes => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Fx8SciFi => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Sitar => "Sitar",
            GeneralMidiProgram::Banjo => "Banjo",
            GeneralMidiProgram::Shamisen => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Koto => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Kalimba => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Bagpipe => "Bagpipes",
            GeneralMidiProgram::Fiddle => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Shanai => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::TinkleBell => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Agogo => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::SteelDrums => {
                "WheelsOfSteel" // TODO same?
            }
            GeneralMidiProgram::Woodblock => "SideStick",
            GeneralMidiProgram::TaikoDrum => {
                // XXXXXXXXXXXXX TMP
                "Cello" // TODO substitute.....
            }
            GeneralMidiProgram::MelodicTom => "Bongos",
            GeneralMidiProgram::SynthDrum => "SnareDrum",
            GeneralMidiProgram::ReverseCymbal => "Cymbal",
            GeneralMidiProgram::GuitarFretNoise => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::BreathNoise => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Seashore => "OceanWavesWithFoghorn",
            GeneralMidiProgram::BirdTweet => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::TelephoneRing => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Helicopter => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Applause => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Gunshot => {
                delegated = true;
                "Piano"
            }
        };
        if delegated {
            eprintln!("Delegated {program} to {preset}");
        }
        //        Ok(WelshPatchSettings::by_name(preset))
        Ok(WelshPatchSettings::by_name(paths, "todo"))
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FmSynthesizerSettings {
    pub ratio: ParameterType, // TODO: needs a ratio type, which I suppose would range from 0..infinity.
    pub depth: ParameterType,
    pub beta: ParameterType,

    pub carrier_envelope: EnvelopeParams,
    pub modulator_envelope: EnvelopeParams,
}

impl FmSynthesizerSettings {
    #[allow(dead_code)]
    pub fn from_name(_name: &str) -> FmSynthesizerSettings {
        let carrier_envelope = EnvelopeParams::safe_default();
        let modulator_envelope = EnvelopeParams::safe_default();
        FmSynthesizerSettings {
            ratio: 2.0, // Modulator frequency is 2x carrier
            depth: 1.0, // full strength
            beta: 1.0,  // per Wikipedia, this one is visible and audible
            carrier_envelope,
            modulator_envelope,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FilterPreset, LfoDepth, LfoPreset, LfoRoutingType, PolyphonySettings, WelshPatchSettings,
    };
    use crate::patches::OscillatorSettings;
    use convert_case::{Case, Casing};
    use ensnare::prelude::*;
    use float_cmp::approx_eq;
    use groove_core::{
        generators::{Envelope, EnvelopeParams, Waveform},
        time::Seconds,
        traits::{Configurable, Generates, PlaysNotes, Ticks},
        util::tests::TestOnlyPaths,
    };
    use groove_entities::instruments::WelshVoice;

    pub const DEFAULT_BPM: ParameterType = 128.0;
    pub const DEFAULT_MIDI_TICKS_PER_SECOND: usize = 960;

    impl WelshPatchSettings {
        pub fn derive_welsh_voice(&self) -> WelshVoice {
            WelshVoice::new_with(self.derive_welsh_synth_params().voice())
        }
    }

    // TODO dedup
    pub fn canonicalize_output_filename_and_path(filename: &str) -> String {
        let mut path = TestOnlyPaths::writable_out_path();
        path.push(format!("{}.wav", filename.to_case(Case::Snake)).to_string());
        if let Some(path) = path.to_str() {
            path.to_string()
        } else {
            panic!("trouble creating output path")
        }
    }

    #[test]
    fn oscillator_tuning_helpers() {
        // tune
        assert_eq!(OscillatorSettings::octaves(0), Ratio::from(1.0));
        assert_eq!(OscillatorSettings::octaves(1), Ratio::from(2.0));
        assert_eq!(OscillatorSettings::octaves(-1), Ratio::from(0.5));
        assert_eq!(OscillatorSettings::octaves(2), Ratio::from(4.0));
        assert_eq!(OscillatorSettings::octaves(-2), Ratio::from(0.25));

        assert_eq!(
            OscillatorSettings::semis_and_cents(0, 0.0),
            Ratio::from(1.0)
        );
        assert_eq!(
            OscillatorSettings::semis_and_cents(12, 0.0),
            Ratio::from(2.0)
        );
        assert!(
            approx_eq!(
                f64,
                OscillatorSettings::semis_and_cents(5, 0.0).value(),
                1.334_839_854_170_034_4
            ),
            "semis_and_cents() should give sane results"
        ); // 349.2282÷261.6256, F4÷C4
        assert_eq!(
            OscillatorSettings::semis_and_cents(0, -100.0).value(),
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

    // // TODO: get rid of this
    // fn write_sound(
    //     source: &mut WelshVoice,
    //     clock: &mut Clock,
    //     duration: f64,
    //     when: f64,
    //     basename: &str,
    // ) {
    //     let spec = hound::WavSpec {
    //         channels: 2,
    //         sample_rate: clock.sample_rate().value() as u32,
    //         bits_per_sample: 16,
    //         sample_format: hound::SampleFormat::Int,
    //     };
    //     const AMPLITUDE: SampleType = i16::MAX as SampleType;
    //     let mut writer =
    //         hound::WavWriter::create(canonicalize_output_filename_and_path(basename), spec)
    //             .unwrap();

    //     let mut is_message_sent = false;
    //     while clock.seconds() < duration {
    //         if when <= clock.seconds() && !is_message_sent {
    //             is_message_sent = true;
    //             source.note_off(0);
    //         }
    //         source.tick(1);
    //         let sample = source.value();
    //         let _ = writer.write_sample((sample.0 .0 * AMPLITUDE) as i16);
    //         let _ = writer.write_sample((sample.1 .0 * AMPLITUDE) as i16);
    //         clock.tick(1);
    //     }
    // }

    fn cello_patch() -> WelshPatchSettings {
        WelshPatchSettings {
            name: WelshPatchSettings::patch_name_to_settings_name("Cello"),
            oscillator_1: OscillatorSettings {
                waveform: Waveform::PulseWidth(0.1.into()),
                ..Default::default()
            },
            oscillator_2: OscillatorSettings {
                waveform: Waveform::Square,
                ..Default::default()
            },
            oscillator_2_track: true,
            oscillator_2_sync: false,
            noise: 0.0,
            lfo: LfoPreset {
                routing: LfoRoutingType::Amplitude,
                waveform: Waveform::Sine,
                frequency: 7.5,
                depth: LfoDepth::Pct(0.05),
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
            filter_envelope: EnvelopeParams {
                attack: Normal::minimum(),
                decay: Envelope::from_seconds_to_normal(Seconds(3.29)),
                sustain: Envelope::from_seconds_to_normal(Seconds(0.78)),
                release: Normal::maximum(),
            },
            amp_envelope: EnvelopeParams {
                attack: Envelope::from_seconds_to_normal(Seconds(0.06)),
                decay: Normal::maximum(),
                sustain: Normal::maximum(),
                release: Envelope::from_seconds_to_normal(Seconds(0.3)),
            },
        }
    }

    fn boring_test_patch() -> WelshPatchSettings {
        WelshPatchSettings {
            name: WelshPatchSettings::patch_name_to_settings_name("Test"),
            oscillator_1: OscillatorSettings {
                waveform: Waveform::Sawtooth,
                ..Default::default()
            },
            oscillator_2: OscillatorSettings {
                waveform: Waveform::None,
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
            filter_envelope: EnvelopeParams {
                attack: Envelope::from_seconds_to_normal(Seconds(5.0)),
                decay: Normal::maximum(),
                sustain: Normal::maximum(),
                release: Normal::maximum(),
            },
            amp_envelope: EnvelopeParams {
                attack: Envelope::from_seconds_to_normal(Seconds(0.5)),
                decay: Normal::maximum(),
                sustain: Normal::maximum(),
                release: Normal::maximum(),
            },
        }
    }

    #[test]
    fn welsh_makes_any_sound_at_all() {
        let mut voice = boring_test_patch().derive_welsh_voice();
        voice.note_on(60, 127);

        // Skip a few frames in case attack is slow
        voice.tick(5);
        assert!(
            voice.value() != StereoSample::SILENCE,
            "once triggered, voice should make a sound"
        );
    }

    #[cfg(obsolete)]
    #[test]
    fn basic_synth_patch() {
        let mut clock = Clock::new_with(
            DEFAULT_BPM,
            DEFAULT_MIDI_TICKS_PER_SECOND,
            TimeSignature::default(),
        );
        let mut voice = boring_test_patch().derive_welsh_voice();
        clock.update_sample_rate(SampleRate::DEFAULT);
        voice.update_sample_rate(SampleRate::DEFAULT);
        voice.note_on(60, 127);
        voice.tick(1);
        write_sound(&mut voice, &mut clock, 5.0, 5.0, "voice_basic_test_c4");
    }

    #[cfg(obsolete)]
    #[test]
    fn basic_cello_patch() {
        let mut clock = Clock::new_with(
            DEFAULT_BPM,
            DEFAULT_MIDI_TICKS_PER_SECOND,
            TimeSignature::default(),
        );
        let mut voice = cello_patch().derive_welsh_voice();
        clock.update_sample_rate(SampleRate::DEFAULT);
        voice.update_sample_rate(SampleRate::DEFAULT);
        voice.note_on(60, 127);
        voice.tick(1);
        write_sound(&mut voice, &mut clock, 5.0, 3.0, "voice_cello_c4");
    }
}
