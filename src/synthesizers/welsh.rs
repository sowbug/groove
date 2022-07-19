use num_traits::FromPrimitive;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use strum_macros::{EnumIter, IntoStaticStr};

use crate::{
    common::{MidiMessage, MidiMessageType, MidiNote, WaveformType},
    devices::traits::DeviceTrait,
    general_midi::GeneralMidiProgram,
    preset::{EnvelopePreset, FilterPreset, LfoPreset, LfoRouting, OscillatorPreset},
    primitives::{
        clock::Clock,
        envelopes::MiniEnvelope,
        filter::{MiniFilter2, MiniFilter2Type},
        oscillators::MiniOscillator,
        AudioSourceTrait, EffectTrait,
    },
};

#[derive(EnumIter, IntoStaticStr)]
pub enum PresetName {
    // -------------------- Strings
    Banjo,
    Cello,
    DoubleBass,
    Dulcimer,
    GuitarAcoustic,
    GuitarElectric,
    Harp,
    HurdyGurdy,
    Kora,
    Lute,
    Mandocello,
    Mandolin,
    Riti,
    Sitar,
    StandupBass,
    Viola,
    Violin,
    // -------------------- Woodwinds
    Bagpipes,
    BassClarinet,
    Bassoon,
    Clarinet,
    ConchShell,
    Contrabassoon,
    Digeridoo,
    EnglishHorn,
    Flute,
    Oboe,
    Piccolo,
    // -------------------- Brass
    FrenchHorn,
    Harmonica,
    PennyWhistle,
    Saxophone,
    Trombone,
    Trumpet,
    Tuba,
    // -------------------- Keyboards
    Accordion,
    Celeste,
    Clavichord,
    ElectricPiano,
    Harpsichord,
    Organ,
    Piano,
    // -------------------- Vocals
    Angels,
    Choir,
    VocalFemale,
    VocalMale,
    Whistling,
    // -------------------- Tuned Percussion
    Bell,
    Bongos,
    Conga,
    Glockenspiel,
    Marimba,
    Timpani,
    Xylophone,
    // -------------------- Untuned Percussion
    BassDrum,
    Castanets,
    Clap,
    Claves,
    Cowbell,
    CowbellAnalog,
    Cymbal,
    SideStick,
    SnareDrum,
    Tambourine,
    WheelsOfSteel,
    // -------------------- Leads
    BrassSection,
    Mellow70sLead,
    MonoSolo,
    NewAgeLead,
    RAndBSlide,
    ScreamingSync,
    StringsPwm,
    Trance5th,
    // -------------------- Bass
    AcidBass,
    BassOfTheTimeLords,
    DetroitBass,
    DeutscheBass,
    DigitalBass,
    FunkBass,
    GrowlingBass,
    RezBass,
    // -------------------- Pads
    AndroidDreams,
    CelestialWash,
    DarkCity,
    Aurora,
    GalacticCathedral,
    GalacticChapel,
    Portus,
    PostApocalypticSyncSweep,
    TerraEnceladus,
    // -------------------- Sound Effects
    Cat,
    DigitalAlarmClock,
    JourneyToTheCore,
    Kazoo,
    Laser,
    Motor,
    NerdOTron2000,
    OceanWavesWithFoghorn,
    PositronicRhythm,
    SpaceAttack,
    Toad,
    Wind,
}

#[derive(Debug, Clone, Copy)]
pub enum GlidePreset {
    Off,
    On(f32),
}

impl Default for GlidePreset {
    fn default() -> Self {
        GlidePreset::Off
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PolyphonyPreset {
    Multi,
    Mono,
    MultiLimit(u8),
}

impl Default for PolyphonyPreset {
    fn default() -> Self {
        PolyphonyPreset::Multi
    }
}

#[derive(Default, Debug, Clone)]
pub struct SynthPreset {
    pub oscillator_1_preset: OscillatorPreset,
    pub oscillator_2_preset: OscillatorPreset,
    pub oscillator_2_track: bool,
    pub oscillator_2_sync: bool,

    pub noise: f32,

    pub lfo_preset: LfoPreset,

    pub glide: GlidePreset,
    pub has_unison: bool,
    pub polyphony: PolyphonyPreset,

    // There is meant to be only one filter, but the Welsh book
    // provides alternate settings depending on the kind of filter
    // your synthesizer has.
    pub filter_type_24db: FilterPreset,
    pub filter_type_12db: FilterPreset,
    pub filter_resonance: f32, // This should be an appropriate interpretation of a linear 0..1
    pub filter_envelope_weight: f32,
    pub filter_envelope_preset: EnvelopePreset,

    pub amp_envelope_preset: EnvelopePreset,
}

impl SynthPreset {
    pub fn by_name(name: &PresetName) -> Self {
        match name {
            PresetName::Banjo => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: WaveformType::PulseWidth(0.2),
                    ..Default::default()
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: WaveformType::PulseWidth(0.1),
                    tune: OscillatorPreset::semis_and_cents(5.0, 0.0),
                    mix: 0.80,
                },
                oscillator_2_track: true,
                oscillator_2_sync: true,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::Amplitude,
                    waveform: WaveformType::Triangle,
                    frequency: 10.0,
                    depth: LfoPreset::percent(10.0),
                },
                glide: GlidePreset::Off,
                has_unison: false,
                polyphony: PolyphonyPreset::MultiLimit(5),
                filter_type_24db: FilterPreset {
                    cutoff: 2900.0,
                    weight: 0.72,
                },
                filter_type_12db: FilterPreset {
                    cutoff: 1500.0,
                    weight: 0.63,
                },
                filter_resonance: 0.0,
                filter_envelope_weight: 0.75,
                filter_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 0.19,
                    sustain_percentage: 0.0,
                    release_seconds: 0.19,
                },
                amp_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 0.67,
                    sustain_percentage: 0.0,
                    release_seconds: 0.67,
                },
            },
            PresetName::Cello => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: WaveformType::PulseWidth(0.1),
                    ..Default::default()
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: WaveformType::Square,
                    ..Default::default()
                },
                oscillator_2_track: true,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::Amplitude,
                    waveform: WaveformType::Sine,
                    frequency: 7.5,
                    depth: LfoPreset::percent(5.0),
                },
                glide: GlidePreset::Off,
                has_unison: false,
                polyphony: PolyphonyPreset::Multi,
                filter_type_24db: FilterPreset {
                    cutoff: 40.0,
                    weight: 0.1,
                },
                filter_type_12db: FilterPreset {
                    cutoff: 40.0,
                    weight: 0.1,
                },
                filter_resonance: 0.0,
                filter_envelope_weight: 0.9,
                filter_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 3.29,
                    sustain_percentage: 0.78,
                    release_seconds: EnvelopePreset::MAX,
                },
                amp_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.06,
                    decay_seconds: EnvelopePreset::MAX,
                    sustain_percentage: 1.0,
                    release_seconds: 0.3,
                },
            },
            PresetName::DoubleBass => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: WaveformType::PulseWidth(0.45),
                    tune: OscillatorPreset::octaves(-1.0),
                    ..Default::default()
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: WaveformType::Square,
                    mix: 0.6,
                    ..Default::default()
                },
                oscillator_2_track: true,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::Pitch,
                    waveform: WaveformType::Triangle,
                    frequency: 5.0,
                    depth: LfoPreset::semis_and_cents(0.0, 11.0),
                },
                glide: GlidePreset::Off,
                has_unison: false,
                polyphony: PolyphonyPreset::Multi,
                filter_type_24db: FilterPreset {
                    cutoff: 1600.0,
                    weight: 0.63,
                },
                filter_type_12db: FilterPreset {
                    cutoff: 750.0,
                    weight: 0.52,
                },
                filter_resonance: 0.0,
                filter_envelope_weight: 0.0,
                filter_envelope_preset: EnvelopePreset {
                    ..Default::default()
                },
                amp_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.35,
                    decay_seconds: EnvelopePreset::MAX,
                    sustain_percentage: 1.0,
                    release_seconds: 0.19,
                },
            },
            PresetName::Dulcimer => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: WaveformType::PulseWidth(0.25),
                    tune: OscillatorPreset::semis_and_cents(-7.0, 0.0),
                    mix: 0.80,
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: WaveformType::PulseWidth(0.05),
                    ..Default::default()
                },
                oscillator_2_track: true,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::Amplitude,
                    waveform: WaveformType::Triangle,
                    frequency: 1.5,
                    depth: 0.22,
                },
                glide: GlidePreset::Off,
                has_unison: false,
                polyphony: PolyphonyPreset::Multi,
                filter_type_24db: FilterPreset {
                    cutoff: 600.0,
                    weight: 0.49,
                },
                filter_type_12db: FilterPreset {
                    cutoff: 200.0,
                    weight: 0.33,
                },
                filter_resonance: 0.0,
                filter_envelope_weight: 0.50,
                filter_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 1.69,
                    sustain_percentage: 0.0,
                    release_seconds: 1.78,
                },
                amp_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 4.0,
                    sustain_percentage: 0.0,
                    release_seconds: 4.0,
                },
            },

            PresetName::GuitarAcoustic => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: WaveformType::PulseWidth(0.25),
                    ..Default::default()
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: WaveformType::PulseWidth(0.10),
                    tune: OscillatorPreset::semis_and_cents(10.0, 0.0),
                    mix: 0.9,
                },
                oscillator_2_track: true,
                oscillator_2_sync: true,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::None,
                    ..Default::default()
                },
                glide: GlidePreset::Off,
                has_unison: false,
                polyphony: PolyphonyPreset::Multi,
                filter_type_24db: FilterPreset {
                    cutoff: 3100.0,
                    weight: 0.73,
                },
                filter_type_12db: FilterPreset {
                    cutoff: 2000.0,
                    weight: 0.67,
                },
                filter_resonance: 0.0,
                filter_envelope_weight: 0.70,
                filter_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 0.35,
                    sustain_percentage: 0.0,
                    release_seconds: 0.29,
                },
                amp_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 1.7,
                    sustain_percentage: 0.0,
                    release_seconds: 1.7,
                },
            },

            PresetName::GuitarElectric => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: WaveformType::PulseWidth(0.2),
                    tune: OscillatorPreset::NATURAL_TUNING,
                    mix: 0.65,
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: WaveformType::PulseWidth(0.15),
                    tune: OscillatorPreset::semis_and_cents(10.0, 0.0),
                    mix: OscillatorPreset::FULL_MIX,
                },
                oscillator_2_track: true,
                oscillator_2_sync: true,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    ..Default::default()
                },
                glide: GlidePreset::Off,
                has_unison: false,
                polyphony: PolyphonyPreset::Multi,
                filter_type_24db: FilterPreset {
                    cutoff: 20000.0,
                    weight: 1.0,
                },
                filter_type_12db: FilterPreset {
                    cutoff: 20000.0,
                    weight: 1.0,
                },
                filter_resonance: 0.0,
                filter_envelope_weight: 0.0,
                filter_envelope_preset: EnvelopePreset {
                    ..Default::default()
                },
                amp_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 1.7,
                    sustain_percentage: 0.0,
                    release_seconds: 1.7,
                },
            },
            PresetName::Harp => {
                panic!()
            }
            PresetName::HurdyGurdy => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: WaveformType::PulseWidth(0.15),
                    tune: OscillatorPreset::NATURAL_TUNING,
                    mix: 0.90,
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: WaveformType::Square,
                    tune: (MidiNote::D3 as u8) as f32,
                    mix: OscillatorPreset::FULL_MIX,
                },
                oscillator_2_track: false,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::None,
                    ..Default::default()
                },
                glide: GlidePreset::On(0.04),
                has_unison: false,
                polyphony: PolyphonyPreset::Mono,
                filter_type_24db: FilterPreset {
                    cutoff: 40.0,
                    weight: 0.10,
                },
                filter_type_12db: FilterPreset {
                    cutoff: 40.0,
                    weight: 0.10,
                },
                filter_resonance: 0.0,
                filter_envelope_weight: 1.0,
                filter_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.04,
                    decay_seconds: EnvelopePreset::MAX,
                    sustain_percentage: 1.0,
                    release_seconds: 0.23,
                },
                amp_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: EnvelopePreset::MAX,
                    sustain_percentage: 1.0,
                    release_seconds: 0.85,
                },
            },
            PresetName::Kora => {
                panic!()
            }
            PresetName::Lute => {
                panic!()
            }
            PresetName::Mandocello => {
                panic!()
            }
            PresetName::Mandolin => {
                panic!()
            }
            PresetName::Riti => {
                panic!()
            }
            PresetName::Sitar => {
                panic!()
            }
            PresetName::StandupBass => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: WaveformType::PulseWidth(0.25),
                    tune: OscillatorPreset::octaves(-1.0),
                    mix: OscillatorPreset::FULL_MIX,
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: WaveformType::Triangle,
                    tune: OscillatorPreset::octaves(-1.0),
                    mix: OscillatorPreset::FULL_MIX,
                },
                oscillator_2_track: true,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::Amplitude,
                    waveform: WaveformType::Triangle,
                    frequency: 15.0,
                    depth: 0.1,
                },
                glide: GlidePreset::Off,
                has_unison: false,
                polyphony: PolyphonyPreset::Multi,
                filter_type_24db: FilterPreset {
                    cutoff: 40.0,
                    weight: 0.1,
                },
                filter_type_12db: FilterPreset {
                    cutoff: 40.0,
                    weight: 0.1,
                },
                filter_resonance: 0.0,
                filter_envelope_weight: 0.75,
                filter_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 2.33,
                    sustain_percentage: 0.6,
                    release_seconds: 2.33,
                },
                amp_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 1.28,
                    sustain_percentage: 0.0,
                    release_seconds: 1.38,
                },
            },
            PresetName::Viola => {
                panic!()
            }
            PresetName::Violin => {
                panic!()
            }
            // -------------------- Woodwinds
            PresetName::Bagpipes => {
                panic!()
            }
            PresetName::BassClarinet => {
                panic!()
            }
            PresetName::Bassoon => {
                panic!()
            }
            PresetName::Clarinet => {
                panic!()
            }
            PresetName::ConchShell => {
                panic!()
            }
            PresetName::Contrabassoon => {
                panic!()
            }
            PresetName::Digeridoo => {
                panic!()
            }
            PresetName::EnglishHorn => {
                panic!()
            }
            PresetName::Flute => {
                panic!()
            }
            PresetName::Oboe => {
                panic!()
            }
            PresetName::Piccolo => {
                panic!()
            }
            // -------------------- Brass
            PresetName::FrenchHorn => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: WaveformType::PulseWidth(0.1),
                    ..Default::default()
                },
                oscillator_2_preset: OscillatorPreset {
                    ..Default::default()
                },
                oscillator_2_track: true,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::None,
                    waveform: WaveformType::None,
                    frequency: 0.0,
                    depth: 0.0,
                },
                glide: GlidePreset::Off,
                has_unison: false,
                polyphony: PolyphonyPreset::Multi,
                filter_type_24db: FilterPreset {
                    cutoff: 40.0,
                    weight: 0.1,
                },
                filter_type_12db: FilterPreset {
                    cutoff: 40.0,
                    weight: 0.1,
                },
                filter_resonance: 0.20,
                filter_envelope_weight: 0.45,
                filter_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.05,
                    decay_seconds: 5.76,
                    sustain_percentage: 0.94,
                    release_seconds: 0.39,
                },
                amp_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 3.9,
                    sustain_percentage: 0.96,
                    release_seconds: 0.93,
                },
            },
            PresetName::Harmonica => {
                panic!()
            }
            PresetName::PennyWhistle => {
                panic!()
            }
            PresetName::Saxophone => {
                SynthPreset {
                    oscillator_1_preset: OscillatorPreset {
                        waveform: WaveformType::PulseWidth(0.3),
                        ..Default::default()
                    },
                    oscillator_2_preset: OscillatorPreset {
                        waveform: WaveformType::PulseWidth(0.45),
                        tune: OscillatorPreset::semis_and_cents(8.0, 0.0),
                        mix: 0.75,
                    },
                    oscillator_2_track: true,
                    oscillator_2_sync: true,
                    noise: 0.0,
                    lfo_preset: LfoPreset {
                        routing: LfoRouting::Pitch, // TODO osc1/osc2 is an option
                        waveform: WaveformType::Sine,
                        frequency: 7.5,
                        depth: LfoPreset::semis_and_cents(0.0, 10.0),
                    },
                    glide: GlidePreset::Off,
                    has_unison: false,
                    polyphony: PolyphonyPreset::Multi,
                    filter_type_24db: FilterPreset {
                        cutoff: 40.0,
                        weight: 0.10,
                    },
                    filter_type_12db: FilterPreset {
                        cutoff: 40.0,
                        weight: 0.10,
                    },
                    filter_resonance: 0.0,
                    filter_envelope_weight: 0.90,
                    filter_envelope_preset: EnvelopePreset {
                        attack_seconds: 0.14,
                        decay_seconds: 0.37,
                        sustain_percentage: 0.78,
                        release_seconds: EnvelopePreset::MAX,
                    },
                    amp_envelope_preset: EnvelopePreset {
                        attack_seconds: 0.0,
                        decay_seconds: EnvelopePreset::MAX,
                        sustain_percentage: 1.0,
                        release_seconds: 0.3,
                    },
                }
            }
            PresetName::Trombone => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: WaveformType::Sawtooth,
                    ..Default::default()
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: WaveformType::Triangle,
                    tune: OscillatorPreset::octaves(1.0),
                    mix: OscillatorPreset::FULL_MIX,
                },
                oscillator_2_track: true,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::Amplitude,
                    waveform: WaveformType::Triangle,
                    frequency: 5.0,
                    depth: LfoPreset::percent(5.0),
                },
                glide: GlidePreset::Off,
                has_unison: false,
                polyphony: PolyphonyPreset::Multi,
                filter_type_24db: FilterPreset {
                    cutoff: 900.0,
                    weight: 0.55,
                },
                filter_type_12db: FilterPreset {
                    cutoff: 200.0,
                    weight: 0.33,
                },
                filter_resonance: 0.0,
                filter_envelope_weight: 0.3,
                filter_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.11,
                    decay_seconds: EnvelopePreset::MAX,
                    sustain_percentage: 1.0,
                    release_seconds: 0.18,
                },
                amp_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.06,
                    decay_seconds: EnvelopePreset::MAX,
                    sustain_percentage: 1.0,
                    release_seconds: 0.50,
                },
            },
            PresetName::Trumpet => {
                panic!()
            }
            PresetName::Tuba => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: WaveformType::Sawtooth,
                    tune: OscillatorPreset::NATURAL_TUNING,
                    mix: 0.85,
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: WaveformType::Sawtooth,
                    tune: OscillatorPreset::octaves(-1.0),
                    mix: OscillatorPreset::FULL_MIX,
                },
                oscillator_2_track: true,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::Amplitude,
                    waveform: WaveformType::Triangle,
                    frequency: 2.4,
                    depth: 0.05,
                },
                glide: GlidePreset::Off,
                has_unison: false,
                polyphony: PolyphonyPreset::Multi,
                filter_type_24db: FilterPreset {
                    cutoff: 40.0,
                    weight: 0.1,
                },
                filter_type_12db: FilterPreset {
                    cutoff: 40.0,
                    weight: 0.1,
                },

                filter_resonance: 0.0,
                filter_envelope_weight: 0.6,
                filter_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.7,
                    decay_seconds: EnvelopePreset::MAX,
                    sustain_percentage: 1.0,
                    release_seconds: 0.11,
                },
                amp_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.03,
                    decay_seconds: EnvelopePreset::MAX,
                    sustain_percentage: 1.0,
                    release_seconds: 0.11,
                },
            },
            // -------------------- Keyboards
            PresetName::Accordion => {
                panic!()
            }
            PresetName::Celeste => {
                panic!()
            }
            PresetName::Clavichord => {
                panic!()
            }
            PresetName::ElectricPiano => {
                panic!()
            }
            PresetName::Harpsichord => {
                panic!()
            }
            PresetName::Organ => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: WaveformType::Triangle,
                    ..Default::default()
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: WaveformType::Triangle,
                    tune: OscillatorPreset::octaves(-2.0),
                    ..Default::default()
                },
                oscillator_2_track: true,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::None,
                    ..Default::default()
                },
                glide: GlidePreset::Off,
                has_unison: false,
                polyphony: PolyphonyPreset::Multi,
                filter_type_24db: FilterPreset {
                    cutoff: 20000.0,
                    weight: 1.0,
                },
                filter_type_12db: FilterPreset {
                    cutoff: 20000.0,
                    weight: 1.0,
                },
                filter_resonance: 0.0,
                filter_envelope_weight: 0.0,
                filter_envelope_preset: EnvelopePreset {
                    ..Default::default()
                },
                amp_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.6,
                    decay_seconds: EnvelopePreset::MAX,
                    sustain_percentage: 1.0,
                    release_seconds: 0.4,
                },
            },
            PresetName::Piano => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: WaveformType::Sawtooth,
                    mix: 0.75,
                    ..Default::default()
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: WaveformType::PulseWidth(0.15),
                    tune: OscillatorPreset::semis_and_cents(12.0 + 2.0, 0.0),
                    ..Default::default()
                },
                oscillator_2_track: true,
                oscillator_2_sync: true,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::None,
                    ..Default::default()
                },
                glide: GlidePreset::Off,
                has_unison: false,
                polyphony: PolyphonyPreset::Multi,
                filter_type_24db: FilterPreset {
                    cutoff: 40.0,
                    weight: 0.10,
                },
                filter_type_12db: FilterPreset {
                    cutoff: 40.0,
                    weight: 0.10,
                },
                filter_resonance: 0.0,
                filter_envelope_weight: 0.75,
                filter_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 5.22,
                    sustain_percentage: 0.0,
                    release_seconds: EnvelopePreset::MAX,
                },
                amp_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 0.67,
                    sustain_percentage: 0.25,
                    release_seconds: 0.50,
                },
            },
            // -------------------- Vocals
            PresetName::Angels => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: WaveformType::Sawtooth,
                    ..Default::default()
                },
                oscillator_2_preset: OscillatorPreset {
                    ..Default::default()
                },
                oscillator_2_track: true,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::Pitch,
                    waveform: WaveformType::Triangle,
                    frequency: 2.4,
                    depth: LfoPreset::semis_and_cents(0.0, 20.0),
                },
                glide: GlidePreset::Off,
                has_unison: false,
                polyphony: PolyphonyPreset::Multi,
                filter_type_24db: FilterPreset {
                    cutoff: 900.0,
                    weight: 0.55,
                },
                filter_type_12db: FilterPreset {
                    cutoff: 900.0,
                    weight: 0.55,
                },
                filter_resonance: 0.7,
                filter_envelope_weight: 0.0,
                filter_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 0.0,
                    sustain_percentage: 0.0,
                    release_seconds: 0.0,
                },
                amp_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.32,
                    decay_seconds: EnvelopePreset::MAX,
                    sustain_percentage: 1.0,
                    release_seconds: 0.93,
                },
            },
            PresetName::Choir => {
                panic!()
            }
            PresetName::VocalFemale => {
                panic!()
            }
            PresetName::VocalMale => {
                panic!()
            }
            PresetName::Whistling => {
                panic!()
            }
            // -------------------- Tuned Percussion
            PresetName::Bell => {
                panic!()
            }
            PresetName::Bongos => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: WaveformType::Triangle,
                    ..Default::default()
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: WaveformType::Square,
                    tune: OscillatorPreset::NATURAL_TUNING,
                    mix: 0.65,
                },
                oscillator_2_track: true,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    ..Default::default()
                },
                glide: GlidePreset::Off,
                has_unison: false,
                polyphony: PolyphonyPreset::MultiLimit(2),
                filter_type_24db: FilterPreset {
                    cutoff: 600.0,
                    weight: 0.49,
                },
                filter_type_12db: FilterPreset {
                    cutoff: 400.0,
                    weight: 0.43,
                },
                filter_resonance: 0.0,
                filter_envelope_weight: 0.6,
                filter_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 0.11,
                    sustain_percentage: 0.0,
                    release_seconds: 0.11,
                },
                amp_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 0.22,
                    sustain_percentage: 0.0,
                    release_seconds: 0.22,
                },
            },
            PresetName::Conga => {
                panic!()
            }
            PresetName::Glockenspiel => {
                panic!()
            }
            PresetName::Marimba => {
                panic!()
            }
            PresetName::Timpani => {
                panic!()
            }
            PresetName::Xylophone => {
                panic!()
            }
            // -------------------- Untuned Percussion
            PresetName::BassDrum => {
                panic!()
            }
            PresetName::Castanets => {
                panic!()
            }
            PresetName::Clap => {
                panic!()
            }
            PresetName::Claves => {
                panic!()
            }
            PresetName::Cowbell => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: WaveformType::Square,
                    ..Default::default()
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: WaveformType::PulseWidth(0.35),
                    tune: OscillatorPreset::semis_and_cents(12.0 + 2.0, 0.0),
                    ..Default::default()
                },
                oscillator_2_track: true,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::None,
                    ..Default::default()
                },
                glide: GlidePreset::Off,
                has_unison: false,
                polyphony: PolyphonyPreset::Mono,
                filter_type_24db: FilterPreset {
                    cutoff: 8800.0,
                    weight: 0.88,
                },
                filter_type_12db: FilterPreset {
                    cutoff: 8800.0,
                    weight: 0.88,
                },
                filter_resonance: 0.55,
                filter_envelope_weight: 0.65,
                filter_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 0.02,
                    sustain_percentage: 0.65,
                    release_seconds: EnvelopePreset::MAX,
                },
                amp_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 0.15,
                    sustain_percentage: 0.0,
                    release_seconds: 0.15,
                },
            },
            PresetName::CowbellAnalog => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: WaveformType::PulseWidth(0.1),
                    ..Default::default()
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: WaveformType::PulseWidth(0.35),
                    tune: OscillatorPreset::semis_and_cents(5.0, 0.0),
                    ..Default::default()
                },
                oscillator_2_track: true,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::None,
                    ..Default::default()
                },
                glide: GlidePreset::Off,
                has_unison: false,
                polyphony: PolyphonyPreset::Mono,
                filter_type_24db: FilterPreset {
                    cutoff: 8100.0,
                    weight: 0.87,
                },
                filter_type_12db: FilterPreset {
                    cutoff: 3400.0,
                    weight: 0.74,
                },
                filter_resonance: 0.0,
                filter_envelope_weight: 0.65,
                filter_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 0.02,
                    sustain_percentage: 0.65,
                    release_seconds: EnvelopePreset::MAX,
                },
                amp_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 0.15,
                    sustain_percentage: 0.0,
                    release_seconds: 0.15,
                },
            },
            PresetName::Cymbal => Self {
                noise: 1.0,
                polyphony: PolyphonyPreset::Mono,
                filter_type_24db: FilterPreset {
                    cutoff: 9400.0,
                    weight: 0.89,
                },
                filter_type_12db: FilterPreset {
                    cutoff: 9400.0,
                    weight: 0.89,
                },
                filter_resonance: 0.50,
                filter_envelope_weight: 0.70,
                filter_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 0.14,
                    sustain_percentage: 0.0,
                    release_seconds: 1.8,
                },
                amp_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 1.1,
                    sustain_percentage: 0.0,
                    release_seconds: 1.0,
                },
                ..Default::default()
            },
            PresetName::SideStick => Self {
                noise: 1.0,
                polyphony: PolyphonyPreset::Mono,
                filter_type_24db: FilterPreset {
                    cutoff: 2700.0,
                    weight: 0.71,
                },
                filter_type_12db: FilterPreset {
                    cutoff: 2700.0,
                    weight: 0.71,
                },
                filter_resonance: 1.0,
                filter_envelope_weight: 0.85,
                filter_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 1.19,
                    sustain_percentage: 0.0,
                    release_seconds: 0.0,
                },
                amp_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 0.09,
                    sustain_percentage: 0.0,
                    release_seconds: 0.0,
                },
                ..Default::default()
            },
            PresetName::SnareDrum => {
                panic!()
            }
            PresetName::Tambourine => {
                panic!()
            }
            PresetName::WheelsOfSteel => {
                panic!()
            }
            // -------------------- Leads
            PresetName::BrassSection => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: WaveformType::Square,
                    tune: OscillatorPreset::semis_and_cents(0.0, -10.0),
                    mix: OscillatorPreset::FULL_MIX,
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: WaveformType::PulseWidth(0.2),
                    tune: OscillatorPreset::semis_and_cents(12.0, 10.0),
                    mix: OscillatorPreset::FULL_MIX,
                },
                oscillator_2_track: true,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::PulseWidth,
                    waveform: WaveformType::Triangle,
                    frequency: 5.5,
                    depth: 0.45,
                },
                glide: GlidePreset::Off,
                has_unison: false,
                polyphony: PolyphonyPreset::Multi,
                filter_type_24db: FilterPreset {
                    cutoff: 40.0,
                    weight: 0.1,
                },
                filter_type_12db: FilterPreset {
                    cutoff: 40.0,
                    weight: 0.1,
                },
                filter_resonance: 0.0,
                filter_envelope_weight: 1.0,
                filter_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.03,
                    decay_seconds: EnvelopePreset::MAX,
                    sustain_percentage: 1.0,
                    release_seconds: 0.6,
                },
                amp_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: EnvelopePreset::MAX,
                    sustain_percentage: 1.0,
                    release_seconds: 0.35,
                },
            },
            PresetName::Mellow70sLead => {
                panic!()
            }
            PresetName::MonoSolo => {
                panic!()
            }
            PresetName::NewAgeLead => {
                panic!()
            }
            PresetName::RAndBSlide => {
                panic!()
            }
            PresetName::ScreamingSync => {
                panic!()
            }
            PresetName::StringsPwm => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: WaveformType::Square,
                    tune: OscillatorPreset::semis_and_cents(0.0, -10.0),
                    mix: OscillatorPreset::FULL_MIX,
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: WaveformType::Square,
                    tune: OscillatorPreset::semis_and_cents(0.0, 10.0),
                    mix: OscillatorPreset::FULL_MIX,
                },
                oscillator_2_track: true,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::PulseWidth,
                    waveform: WaveformType::Sine,
                    frequency: 2.0,
                    depth: 0.47,
                },
                glide: GlidePreset::Off,
                has_unison: false,
                polyphony: PolyphonyPreset::Multi,
                filter_type_24db: FilterPreset {
                    cutoff: 2000.0,
                    weight: 0.67,
                },
                filter_type_12db: FilterPreset {
                    cutoff: 2000.0,
                    weight: 0.67,
                },
                filter_resonance: 0.0,
                filter_envelope_weight: 1.0,
                filter_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.09,
                    decay_seconds: EnvelopePreset::MAX,
                    sustain_percentage: 1.0,
                    release_seconds: EnvelopePreset::MAX,
                },
                amp_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.11,
                    decay_seconds: EnvelopePreset::MAX,
                    sustain_percentage: 1.0,
                    release_seconds: 0.35,
                },
            },
            PresetName::Trance5th => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: WaveformType::Square,
                    ..Default::default()
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: WaveformType::Square,
                    tune: OscillatorPreset::semis_and_cents(7.0, 0.0),
                    ..Default::default()
                },
                oscillator_2_track: true,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::PulseWidth,
                    waveform: WaveformType::Triangle, // TODO: this should be two different waveforms, one for each osc
                    frequency: 6.0,
                    depth: 0.8,
                },
                glide: GlidePreset::Off,
                has_unison: true,
                polyphony: PolyphonyPreset::Mono,
                filter_type_24db: FilterPreset {
                    cutoff: 20000.0,
                    weight: 1.0,
                },
                filter_type_12db: FilterPreset {
                    cutoff: 20000.0,
                    weight: 1.0,
                },
                filter_resonance: 0.0,
                filter_envelope_weight: 0.0,
                filter_envelope_preset: EnvelopePreset {
                    ..Default::default()
                },
                amp_envelope_preset: EnvelopePreset {
                    decay_seconds: EnvelopePreset::MAX,
                    ..Default::default()
                },
            }, // -------------------- Bass
            PresetName::AcidBass => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: WaveformType::PulseWidth(0.25),
                    tune: OscillatorPreset::semis_and_cents(0.0, 10.),
                    mix: 0.7,
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: WaveformType::Square,
                    tune: OscillatorPreset::semis_and_cents(-24.0, -10.),
                    mix: OscillatorPreset::FULL_MIX,
                },
                oscillator_2_track: true,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    ..Default::default()
                },
                glide: GlidePreset::Off,
                has_unison: false,
                polyphony: PolyphonyPreset::Mono,
                filter_type_24db: FilterPreset {
                    cutoff: 450.0,
                    weight: 0.45,
                },
                filter_type_12db: FilterPreset {
                    cutoff: 330.0,
                    weight: 0.40,
                },
                filter_resonance: 0.6,
                filter_envelope_weight: 0.0,
                filter_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 0.0,
                    sustain_percentage: 0.0,
                    release_seconds: 0.0,
                },
                amp_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 0.45,
                    sustain_percentage: 0.15,
                    release_seconds: 0.26,
                },
            },
            PresetName::BassOfTheTimeLords => {
                panic!()
            }
            PresetName::DetroitBass => {
                panic!()
            }
            PresetName::DeutscheBass => {
                panic!()
            }
            PresetName::DigitalBass => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: WaveformType::Square,
                    tune: OscillatorPreset::octaves(-1.0),
                    mix: 0.85,
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: WaveformType::Sawtooth,
                    tune: OscillatorPreset::octaves(-2.0),
                    mix: OscillatorPreset::FULL_MIX,
                },
                oscillator_2_track: true,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::None,
                    ..Default::default()
                },
                glide: GlidePreset::Off,
                has_unison: false,
                polyphony: PolyphonyPreset::Mono,
                filter_type_24db: FilterPreset {
                    cutoff: 122.0,
                    weight: 0.26,
                },
                filter_type_12db: FilterPreset {
                    cutoff: 75.0,
                    weight: 0.19,
                },
                filter_resonance: 0.0,
                filter_envelope_weight: 1.0,
                filter_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 0.15,
                    sustain_percentage: 0.0,
                    release_seconds: 0.0,
                },
                amp_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: EnvelopePreset::MAX,
                    sustain_percentage: 1.0,
                    release_seconds: 1.0,
                },
            },
            PresetName::FunkBass => {
                panic!()
            }
            PresetName::GrowlingBass => {
                panic!()
            }
            PresetName::RezBass => {
                panic!()
            }
            // -------------------- Pads
            PresetName::AndroidDreams => {
                panic!()
            }
            PresetName::CelestialWash => {
                panic!()
            }
            PresetName::DarkCity => {
                panic!()
            }
            PresetName::Aurora => {
                panic!()
            }
            PresetName::GalacticCathedral => {
                panic!()
            }
            PresetName::GalacticChapel => {
                panic!()
            }
            PresetName::Portus => {
                panic!()
            }
            PresetName::PostApocalypticSyncSweep => {
                panic!()
            }
            PresetName::TerraEnceladus => {
                panic!()
            }
            // -------------------- Sound Effects
            PresetName::Cat => {
                panic!()
            }
            PresetName::DigitalAlarmClock => {
                panic!()
            }
            PresetName::JourneyToTheCore => {
                panic!()
            }
            PresetName::Kazoo => {
                panic!()
            }
            PresetName::Laser => {
                panic!()
            }
            PresetName::Motor => {
                panic!()
            }
            PresetName::NerdOTron2000 => {
                panic!()
            }
            PresetName::OceanWavesWithFoghorn => {
                panic!()
            }
            PresetName::PositronicRhythm => {
                panic!()
            }
            PresetName::SpaceAttack => {
                panic!()
            }
            PresetName::Toad => {
                panic!()
            }
            PresetName::Wind => Self {
                noise: 1.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::Amplitude,
                    waveform: WaveformType::Noise,
                    frequency: 0.7, // what does it mean for noise to have a frequency?
                    depth: 0.4,
                },
                filter_type_24db: FilterPreset {
                    cutoff: 780.0,
                    weight: 0.53,
                },
                filter_type_12db: FilterPreset {
                    cutoff: 780.0,
                    weight: 0.53,
                },
                filter_resonance: 0.75,
                amp_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.4,
                    decay_seconds: EnvelopePreset::MAX,
                    release_seconds: 2.7,
                    ..Default::default()
                },
                ..Default::default()
            },

            _ => {
                panic!();
            }
        }
    }
}

impl Synth {
    pub fn get_general_midi_preset(program: GeneralMidiProgram) -> SynthPreset {
        match program {
            GeneralMidiProgram::AcousticGrand => SynthPreset::by_name(&PresetName::Piano),
            GeneralMidiProgram::BrightAcoustic => {
                SynthPreset::by_name(&PresetName::Piano) // TODO dup
            }
            GeneralMidiProgram::ElectricGrand => SynthPreset::by_name(&PresetName::ElectricPiano),
            GeneralMidiProgram::HonkyTonk => {
                panic!();
            }
            GeneralMidiProgram::ElectricPiano1 => {
                SynthPreset::by_name(&PresetName::ElectricPiano) // TODO dup
            }
            GeneralMidiProgram::ElectricPiano2 => {
                SynthPreset::by_name(&PresetName::ElectricPiano) // TODO dup
            }
            GeneralMidiProgram::Harpsichord => SynthPreset::by_name(&PresetName::Harpsichord),
            GeneralMidiProgram::Clav => SynthPreset::by_name(&PresetName::Clavichord),
            GeneralMidiProgram::Celesta => SynthPreset::by_name(&PresetName::Celeste),
            GeneralMidiProgram::Glockenspiel => SynthPreset::by_name(&PresetName::Glockenspiel),
            GeneralMidiProgram::MusicBox => {
                panic!();
            }
            GeneralMidiProgram::Vibraphone => {
                panic!();
            }
            GeneralMidiProgram::Marimba => SynthPreset::by_name(&PresetName::Marimba),
            GeneralMidiProgram::Xylophone => SynthPreset::by_name(&PresetName::Xylophone),
            GeneralMidiProgram::TubularBells => SynthPreset::by_name(&PresetName::Bell),
            GeneralMidiProgram::Dulcimer => SynthPreset::by_name(&PresetName::Dulcimer),
            GeneralMidiProgram::DrawbarOrgan => {
                SynthPreset::by_name(&PresetName::Organ) // TODO dup
            }
            GeneralMidiProgram::PercussiveOrgan => {
                SynthPreset::by_name(&PresetName::Organ) // TODO dup
            }
            GeneralMidiProgram::RockOrgan => {
                SynthPreset::by_name(&PresetName::Organ) // TODO dup
            }
            GeneralMidiProgram::ChurchOrgan => {
                SynthPreset::by_name(&PresetName::Organ) // TODO dup
            }
            GeneralMidiProgram::ReedOrgan => {
                SynthPreset::by_name(&PresetName::Organ) // TODO dup
            }
            GeneralMidiProgram::Accordion => SynthPreset::by_name(&PresetName::Accordion),
            GeneralMidiProgram::Harmonica => SynthPreset::by_name(&PresetName::Harmonica),
            GeneralMidiProgram::TangoAccordion => {
                panic!();
            }
            GeneralMidiProgram::AcousticGuitarNylon => {
                SynthPreset::by_name(&PresetName::GuitarAcoustic)
            }
            GeneralMidiProgram::AcousticGuitarSteel => {
                SynthPreset::by_name(&PresetName::GuitarAcoustic) // TODO dup
            }
            GeneralMidiProgram::ElectricGuitarJazz => {
                SynthPreset::by_name(&PresetName::GuitarElectric) // TODO dup
            }
            GeneralMidiProgram::ElectricGuitarClean => {
                SynthPreset::by_name(&PresetName::GuitarElectric) // TODO dup
            }
            GeneralMidiProgram::ElectricGuitarMuted => {
                SynthPreset::by_name(&PresetName::GuitarElectric) // TODO dup
            }
            GeneralMidiProgram::OverdrivenGuitar => {
                SynthPreset::by_name(&PresetName::GuitarElectric) // TODO dup
            }
            GeneralMidiProgram::DistortionGuitar => {
                SynthPreset::by_name(&PresetName::GuitarElectric) // TODO dup
            }
            GeneralMidiProgram::GuitarHarmonics => {
                SynthPreset::by_name(&PresetName::GuitarElectric) // TODO dup
            }
            GeneralMidiProgram::AcousticBass => SynthPreset::by_name(&PresetName::DoubleBass),
            GeneralMidiProgram::ElectricBassFinger => {
                SynthPreset::by_name(&PresetName::StandupBass)
            }
            GeneralMidiProgram::ElectricBassPick => SynthPreset::by_name(&PresetName::AcidBass),
            GeneralMidiProgram::FretlessBass => {
                SynthPreset::by_name(&PresetName::DetroitBass) // TODO same?
            }
            GeneralMidiProgram::SlapBass1 => SynthPreset::by_name(&PresetName::FunkBass),
            GeneralMidiProgram::SlapBass2 => SynthPreset::by_name(&PresetName::FunkBass),
            GeneralMidiProgram::SynthBass1 => SynthPreset::by_name(&PresetName::DigitalBass),
            GeneralMidiProgram::SynthBass2 => SynthPreset::by_name(&PresetName::DigitalBass),
            GeneralMidiProgram::Violin => SynthPreset::by_name(&PresetName::Violin),
            GeneralMidiProgram::Viola => SynthPreset::by_name(&PresetName::Viola),
            GeneralMidiProgram::Cello => SynthPreset::by_name(&PresetName::Cello),
            GeneralMidiProgram::Contrabass => SynthPreset::by_name(&PresetName::Contrabassoon),
            GeneralMidiProgram::TremoloStrings => {
                panic!();
            }
            GeneralMidiProgram::PizzicatoStrings => {
                panic!();
            }
            GeneralMidiProgram::OrchestralHarp => SynthPreset::by_name(&PresetName::Harp),
            GeneralMidiProgram::Timpani => SynthPreset::by_name(&PresetName::Timpani),
            GeneralMidiProgram::StringEnsemble1 => {
                panic!();
            }
            GeneralMidiProgram::StringEnsemble2 => {
                SynthPreset::by_name(&PresetName::StringsPwm) // TODO same?
            }
            GeneralMidiProgram::Synthstrings1 => SynthPreset::by_name(&PresetName::StringsPwm), // TODO same?

            GeneralMidiProgram::Synthstrings2 => {
                panic!();
            }
            GeneralMidiProgram::ChoirAahs => SynthPreset::by_name(&PresetName::Angels),

            GeneralMidiProgram::VoiceOohs => SynthPreset::by_name(&PresetName::Choir),
            GeneralMidiProgram::SynthVoice => SynthPreset::by_name(&PresetName::VocalFemale),

            GeneralMidiProgram::OrchestraHit => {
                panic!();
            }
            GeneralMidiProgram::Trumpet => SynthPreset::by_name(&PresetName::Trumpet),
            GeneralMidiProgram::Trombone => SynthPreset::by_name(&PresetName::Trombone),
            GeneralMidiProgram::Tuba => SynthPreset::by_name(&PresetName::Tuba),
            GeneralMidiProgram::MutedTrumpet => {
                panic!();
            }
            GeneralMidiProgram::FrenchHorn => SynthPreset::by_name(&PresetName::FrenchHorn),

            GeneralMidiProgram::BrassSection => SynthPreset::by_name(&PresetName::BrassSection),

            GeneralMidiProgram::Synthbrass1 => {
                SynthPreset::by_name(&PresetName::BrassSection) // TODO dup
            }
            GeneralMidiProgram::Synthbrass2 => {
                SynthPreset::by_name(&PresetName::BrassSection) // TODO dup
            }
            GeneralMidiProgram::SopranoSax => {
                SynthPreset::by_name(&PresetName::Saxophone) // TODO dup
            }
            GeneralMidiProgram::AltoSax => SynthPreset::by_name(&PresetName::Saxophone),
            GeneralMidiProgram::TenorSax => {
                SynthPreset::by_name(&PresetName::Saxophone) // TODO dup
            }
            GeneralMidiProgram::BaritoneSax => {
                SynthPreset::by_name(&PresetName::Saxophone) // TODO dup
            }
            GeneralMidiProgram::Oboe => SynthPreset::by_name(&PresetName::Oboe),
            GeneralMidiProgram::EnglishHorn => SynthPreset::by_name(&PresetName::EnglishHorn),
            GeneralMidiProgram::Bassoon => SynthPreset::by_name(&PresetName::Bassoon),
            GeneralMidiProgram::Clarinet => SynthPreset::by_name(&PresetName::Clarinet),
            GeneralMidiProgram::Piccolo => SynthPreset::by_name(&PresetName::Piccolo),
            GeneralMidiProgram::Flute => SynthPreset::by_name(&PresetName::Flute),
            GeneralMidiProgram::Recorder => {
                panic!();
            }
            GeneralMidiProgram::PanFlute => {
                panic!();
            }
            GeneralMidiProgram::BlownBottle => {
                panic!();
            }
            GeneralMidiProgram::Shakuhachi => {
                panic!();
            }
            GeneralMidiProgram::Whistle => {
                panic!();
            }
            GeneralMidiProgram::Ocarina => {
                panic!();
            }
            GeneralMidiProgram::Lead1Square => {
                SynthPreset::by_name(&PresetName::MonoSolo) // TODO: same?
            }
            GeneralMidiProgram::Lead2Sawtooth => {
                SynthPreset::by_name(&PresetName::Trance5th) // TODO: same?
            }
            GeneralMidiProgram::Lead3Calliope => {
                panic!();
            }
            GeneralMidiProgram::Lead4Chiff => {
                panic!();
            }
            GeneralMidiProgram::Lead5Charang => {
                panic!();
            }
            GeneralMidiProgram::Lead6Voice => {
                panic!();
            }
            GeneralMidiProgram::Lead7Fifths => {
                panic!();
            }
            GeneralMidiProgram::Lead8BassLead => {
                panic!();
            }
            GeneralMidiProgram::Pad1NewAge => {
                SynthPreset::by_name(&PresetName::NewAgeLead) // TODO pad or lead?
            }
            GeneralMidiProgram::Pad2Warm => {
                panic!();
            }
            GeneralMidiProgram::Pad3Polysynth => {
                panic!();
            }
            GeneralMidiProgram::Pad4Choir => {
                panic!();
            }
            GeneralMidiProgram::Pad5Bowed => {
                panic!();
            }
            GeneralMidiProgram::Pad6Metallic => {
                panic!();
            }
            GeneralMidiProgram::Pad7Halo => {
                panic!();
            }
            GeneralMidiProgram::Pad8Sweep => {
                panic!();
            }
            GeneralMidiProgram::Fx1Rain => {
                panic!();
            }
            GeneralMidiProgram::Fx2Soundtrack => {
                panic!();
            }
            GeneralMidiProgram::Fx3Crystal => {
                panic!();
            }
            GeneralMidiProgram::Fx4Atmosphere => {
                panic!();
            }
            GeneralMidiProgram::Fx5Brightness => {
                panic!();
            }
            GeneralMidiProgram::Fx6Goblins => {
                panic!();
            }
            GeneralMidiProgram::Fx7Echoes => {
                panic!();
            }
            GeneralMidiProgram::Fx8SciFi => {
                panic!();
            }
            GeneralMidiProgram::Sitar => SynthPreset::by_name(&PresetName::Sitar),
            GeneralMidiProgram::Banjo => SynthPreset::by_name(&PresetName::Banjo),
            GeneralMidiProgram::Shamisen => {
                panic!();
            }
            GeneralMidiProgram::Koto => {
                panic!();
            }
            GeneralMidiProgram::Kalimba => {
                panic!();
            }
            GeneralMidiProgram::Bagpipe => SynthPreset::by_name(&PresetName::Bagpipes),
            GeneralMidiProgram::Fiddle => {
                panic!();
            }
            GeneralMidiProgram::Shanai => {
                panic!();
            }
            GeneralMidiProgram::TinkleBell => {
                panic!();
            }
            GeneralMidiProgram::Agogo => {
                panic!();
            }
            GeneralMidiProgram::SteelDrums => {
                SynthPreset::by_name(&PresetName::WheelsOfSteel) // TODO same?
            }
            GeneralMidiProgram::Woodblock => SynthPreset::by_name(&PresetName::SideStick),
            GeneralMidiProgram::TaikoDrum => {
                // XXXXXXXXXXXXX TMP
                SynthPreset::by_name(&PresetName::Cello) // TODO substitute.....
            }
            GeneralMidiProgram::MelodicTom => SynthPreset::by_name(&PresetName::Bongos),
            GeneralMidiProgram::SynthDrum => SynthPreset::by_name(&PresetName::SnareDrum),
            GeneralMidiProgram::ReverseCymbal => SynthPreset::by_name(&PresetName::Cymbal),
            GeneralMidiProgram::GuitarFretNoise => {
                panic!();
            }
            GeneralMidiProgram::BreathNoise => {
                panic!();
            }
            GeneralMidiProgram::Seashore => {
                SynthPreset::by_name(&PresetName::OceanWavesWithFoghorn)
            }
            GeneralMidiProgram::BirdTweet => {
                panic!();
            }
            GeneralMidiProgram::TelephoneRing => {
                panic!();
            }
            GeneralMidiProgram::Helicopter => {
                panic!();
            }
            GeneralMidiProgram::Applause => {
                panic!();
            }
            GeneralMidiProgram::Gunshot => {
                panic!();
            }
        }
    }
}

#[derive(Default)]
pub struct Voice {
    oscillators: Vec<MiniOscillator>,
    osc_mix: Vec<f32>,
    amp_envelope: MiniEnvelope,

    lfo: MiniOscillator,
    lfo_routing: LfoRouting,
    lfo_depth: f32,

    filter: MiniFilter2,
    filter_cutoff_start: f32,
    filter_cutoff_end: f32,
    filter_envelope: MiniEnvelope,
}

impl Voice {
    pub fn new(sample_rate: u32, preset: &SynthPreset) -> Self {
        let mut r = Self {
            oscillators: Vec::new(),
            osc_mix: Vec::new(),
            amp_envelope: MiniEnvelope::new(sample_rate, &preset.amp_envelope_preset),

            lfo: MiniOscillator::new_lfo(&preset.lfo_preset),
            lfo_routing: preset.lfo_preset.routing,
            lfo_depth: preset.lfo_preset.depth,

            filter: MiniFilter2::new(MiniFilter2Type::LowPass(
                sample_rate,
                preset.filter_type_12db.cutoff,
                1.0 / 2.0f32.sqrt(), // TODO: resonance
            )),
            filter_cutoff_start: MiniFilter2::frequency_to_percent(preset.filter_type_12db.cutoff),
            filter_cutoff_end: preset.filter_envelope_weight,
            filter_envelope: MiniEnvelope::new(sample_rate, &preset.filter_envelope_preset),
        };
        if !matches!(preset.oscillator_1_preset.waveform, WaveformType::None) {
            r.oscillators
                .push(MiniOscillator::new_from_preset(&preset.oscillator_1_preset));
            r.osc_mix.push(preset.oscillator_1_preset.mix);
        }
        if !matches!(preset.oscillator_2_preset.waveform, WaveformType::None) {
            let mut o = MiniOscillator::new_from_preset(&preset.oscillator_2_preset);
            if !preset.oscillator_2_track {
                o.set_fixed_frequency(MidiMessage::note_to_frequency(
                    preset.oscillator_2_preset.tune as u8,
                ));
            }
            r.oscillators.push(o);
            r.osc_mix.push(preset.oscillator_2_preset.mix);
        }
        if preset.noise > 0.0 {
            r.oscillators.push(MiniOscillator::new(WaveformType::Noise));
            r.osc_mix.push(preset.noise);
        }
        r
    }

    pub(crate) fn process(&mut self, time_seconds: f32) -> f32 {
        // LFO
        let lfo = self.lfo.process(time_seconds) * self.lfo_depth;
        if matches!(self.lfo_routing, LfoRouting::Pitch) {
            let lfo_for_pitch = lfo / 10000.0;
            // TODO: divide by 10,000 until we figure out how pitch depth is supposed to go
            // TODO: this could leave a side effect if we reuse voices and forget to clean up.
            for o in self.oscillators.iter_mut() {
                o.set_frequency_modulation(lfo_for_pitch);
            }
        }

        // Oscillators
        let osc_sum = if self.oscillators.is_empty() {
            0.0
        } else {
            let t: f32 = self
                .oscillators
                .iter_mut()
                .map(|o| o.process(time_seconds))
                .sum();
            t / self.oscillators.len() as f32
        };

        // Filters
        self.filter_envelope.tick(time_seconds);
        let new_cutoff_percentage = (self.filter_cutoff_start
            + (self.filter_cutoff_end - self.filter_cutoff_start) * self.filter_envelope.value());
        let new_cutoff = MiniFilter2::percent_to_frequency(new_cutoff_percentage);
        self.filter.set_cutoff(new_cutoff);
        let filtered_mix = self.filter.process(osc_sum, time_seconds);

        // LFO amplitude modulation
        let lfo_amplitude_modulation = if matches!(self.lfo_routing, LfoRouting::Amplitude) {
            // LFO ranges from [-1, 1], so convert to something that can silence or double the volume.
            lfo + 1.0
        } else {
            1.0
        };

        // Envelope
        self.amp_envelope.tick(time_seconds);

        // Final
        filtered_mix * self.amp_envelope.value() * lfo_amplitude_modulation
    }

    pub(crate) fn is_playing(&self) -> bool {
        !self.amp_envelope.is_idle()
    }
}

impl DeviceTrait for Voice {
    fn handle_midi_message(&mut self, message: &MidiMessage, clock: &Clock) {
        self.amp_envelope
            .handle_midi_message(message, clock.seconds);
        self.filter_envelope
            .handle_midi_message(message, clock.seconds);
        match message.status {
            MidiMessageType::NoteOn => {
                let frequency = message.to_frequency();
                for o in self.oscillators.iter_mut() {
                    if matches!(o.waveform, WaveformType::Noise) {
                        continue;
                    }
                    o.set_frequency(frequency);
                }
            }
            MidiMessageType::NoteOff => {}
            MidiMessageType::ProgramChange => {}
        }
    }
}

#[derive(Default)]
pub struct Synth {
    sample_rate: u32,
    preset: SynthPreset,
    note_to_voice: HashMap<u8, Rc<RefCell<Voice>>>,
    current_value: f32,
}

impl Synth {
    pub fn new(sample_rate: u32, preset: SynthPreset) -> Self {
        Self {
            sample_rate,
            preset,
            //voices: Vec::new(),
            note_to_voice: HashMap::new(),
            ..Default::default()
        }
    }

    fn voice_for_note(&mut self, note: u8) -> Rc<RefCell<Voice>> {
        let opt = self.note_to_voice.get(&note);
        if opt.is_some() {
            let voice = opt.unwrap().clone();
            voice
        } else {
            let voice = Rc::new(RefCell::new(Voice::new(self.sample_rate, &self.preset)));
            self.note_to_voice.insert(note, voice.clone());
            voice
        }
    }
}

impl DeviceTrait for Synth {
    fn sources_audio(&self) -> bool {
        true
    }

    fn sinks_midi(&self) -> bool {
        true
    }

    fn handle_midi_message(&mut self, message: &MidiMessage, clock: &Clock) {
        match message.status {
            MidiMessageType::NoteOn => {
                let note = message.data1;
                let voice = self.voice_for_note(note);
                voice.borrow_mut().handle_midi_message(message, clock);
            }
            MidiMessageType::NoteOff => {
                let note = message.data1;
                let voice = self.voice_for_note(note);
                voice.borrow_mut().handle_midi_message(message, clock);

                // TODO: this is incorrect because it kills voices before release is complete
                self.note_to_voice.remove(&note);
            }
            MidiMessageType::ProgramChange => {
                self.preset =
                    Synth::get_general_midi_preset(FromPrimitive::from_u8(message.data1).unwrap());
            }
        }
    }

    fn tick(&mut self, clock: &Clock) -> bool {
        let mut done = true;
        self.current_value = 0.0;
        for (_note, voice) in self.note_to_voice.iter_mut() {
            self.current_value += voice.borrow_mut().process(clock.seconds);
            done = done && !voice.borrow().is_playing();
        }
        if !self.note_to_voice.is_empty() {
            self.current_value /= self.note_to_voice.len() as f32;
        }
        done
    }

    fn get_audio_sample(&self) -> f32 {
        self.current_value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic;

    use crate::{
        common::MidiMessage,
        devices::traits::DeviceTrait,
        primitives::{clock::Clock, tests::canonicalize_filename},
    };

    use super::PresetName;
    use strum::IntoEnumIterator;

    use crate::{
        common::WaveformType,
        preset::{EnvelopePreset, FilterPreset, LfoPreset, LfoRouting, OscillatorPreset},
    };

    use super::Voice;

    const SAMPLE_RATE: u32 = 44100;

    // TODO: refactor out to common test utilities
    fn write_voice(voice: &mut Voice, duration: f32, basename: &str) {
        let mut clock = Clock::new(44100, 4, 4, 128.);

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: clock.sample_rate(),
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        const AMPLITUDE: f32 = i16::MAX as f32;
        let mut writer = hound::WavWriter::create(canonicalize_filename(basename), spec).unwrap();

        let midi_on = MidiMessage::note_on_c4();
        let midi_off = MidiMessage::note_off_c4();

        let mut last_recognized_time_point = -1.;
        let time_note_off = duration / 2.0;
        while clock.seconds < duration {
            if clock.seconds >= 0.0 && last_recognized_time_point < 0.0 {
                last_recognized_time_point = clock.seconds;
                voice.handle_midi_message(&midi_on, &clock);
            } else {
                if clock.seconds >= time_note_off && last_recognized_time_point < time_note_off {
                    last_recognized_time_point = clock.seconds;
                    voice.handle_midi_message(&midi_off, &clock);
                }
            }

            let sample = voice.process(clock.seconds);
            let _ = writer.write_sample((sample * AMPLITUDE) as i16);
            clock.tick();
        }
    }

    #[test]
    fn test_presets() {
        let clock = Clock::new(44100, 4, 4, 128.0);
        for preset in PresetName::iter() {
            let result = panic::catch_unwind(|| {
                Voice::new(clock.sample_rate(), &super::SynthPreset::by_name(&preset))
            });
            if result.is_ok() {
                let mut voice = result.unwrap();
                let preset_name: &str = preset.into();
                write_voice(&mut voice, 2.0, &format!("voice_{}", preset_name));
            }
        }
    }

    // TODO: get rid of this
    fn write_sound(
        source: &mut Voice,
        clock: &mut Clock,
        duration: f32,
        message: &MidiMessage,
        when: f32,
        basename: &str,
    ) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: clock.sample_rate(),
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        const AMPLITUDE: f32 = i16::MAX as f32;
        let mut writer = hound::WavWriter::create(canonicalize_filename(basename), spec).unwrap();

        let mut is_message_sent = false;
        while clock.seconds < duration {
            if (when <= clock.seconds && !is_message_sent) {
                is_message_sent = true;
                source.handle_midi_message(message, clock);
            }
            let sample = source.process(clock.seconds);
            let _ = writer.write_sample((sample * AMPLITUDE) as i16);
            clock.tick();
        }
    }

    fn angels_patch() -> SynthPreset {
        SynthPreset {
            oscillator_1_preset: OscillatorPreset {
                waveform: WaveformType::Sawtooth,
                ..Default::default()
            },
            oscillator_2_preset: OscillatorPreset {
                ..Default::default()
            },
            oscillator_2_track: false,
            oscillator_2_sync: false,
            noise: 0.0,
            lfo_preset: LfoPreset {
                routing: LfoRouting::Pitch,
                waveform: WaveformType::Triangle,
                frequency: 2.4,
                depth: LfoPreset::semis_and_cents(0.0, 20.0),
            },
            glide: GlidePreset::Off,
            has_unison: false,
            polyphony: PolyphonyPreset::Multi,
            filter_type_24db: FilterPreset {
                cutoff: 900.0,
                weight: 0.55,
            },
            filter_type_12db: FilterPreset {
                cutoff: 900.0,
                weight: 0.55,
            },
            filter_resonance: 0.7,
            filter_envelope_weight: 0.0,
            filter_envelope_preset: EnvelopePreset {
                attack_seconds: 0.0,
                decay_seconds: 0.0,
                sustain_percentage: 0.0,
                release_seconds: 0.0,
            },
            amp_envelope_preset: EnvelopePreset {
                attack_seconds: 0.32,
                decay_seconds: EnvelopePreset::MAX,
                sustain_percentage: 1.0,
                release_seconds: 0.93,
            },
        }
    }

    fn cello_patch() -> SynthPreset {
        SynthPreset {
            oscillator_1_preset: OscillatorPreset {
                waveform: WaveformType::PulseWidth(0.1),
                ..Default::default()
            },
            oscillator_2_preset: OscillatorPreset {
                waveform: WaveformType::Square,
                ..Default::default()
            },
            oscillator_2_track: true,
            oscillator_2_sync: false,
            noise: 0.0,
            lfo_preset: LfoPreset {
                routing: LfoRouting::Amplitude,
                waveform: WaveformType::Sine,
                frequency: 7.5,
                depth: LfoPreset::percent(5.0),
            },
            glide: GlidePreset::Off,
            has_unison: false,
            polyphony: PolyphonyPreset::Multi,
            filter_type_24db: FilterPreset {
                cutoff: 40.0,
                weight: 0.1,
            },
            filter_type_12db: FilterPreset {
                cutoff: 40.0,
                weight: 0.1,
            },
            filter_resonance: 0.0,
            filter_envelope_weight: 0.9,
            filter_envelope_preset: EnvelopePreset {
                attack_seconds: 0.0,
                decay_seconds: 3.29,
                sustain_percentage: 0.78,
                release_seconds: EnvelopePreset::MAX,
            },
            amp_envelope_preset: EnvelopePreset {
                attack_seconds: 0.06,
                decay_seconds: EnvelopePreset::MAX,
                sustain_percentage: 1.0,
                release_seconds: 0.3,
            },
        }
    }

    fn test_patch() -> SynthPreset {
        SynthPreset {
            oscillator_1_preset: OscillatorPreset {
                waveform: WaveformType::Sawtooth,
                ..Default::default()
            },
            oscillator_2_preset: OscillatorPreset {
                waveform: WaveformType::None,
                ..Default::default()
            },
            oscillator_2_track: true,
            oscillator_2_sync: false,
            noise: 0.0,
            lfo_preset: LfoPreset {
                routing: LfoRouting::None,
                ..Default::default()
            },
            glide: GlidePreset::Off,
            has_unison: false,
            polyphony: PolyphonyPreset::Multi,
            filter_type_24db: FilterPreset {
                cutoff: 40.0,
                weight: 0.1,
            },
            filter_type_12db: FilterPreset {
                cutoff: 20.0,
                weight: 0.1,
            },
            filter_resonance: 0.0,
            filter_envelope_weight: 1.0,
            filter_envelope_preset: EnvelopePreset {
                attack_seconds: 5.0,
                decay_seconds: EnvelopePreset::MAX,
                sustain_percentage: 1.0,
                release_seconds: EnvelopePreset::MAX,
            },
            amp_envelope_preset: EnvelopePreset {
                attack_seconds: 0.5,
                decay_seconds: EnvelopePreset::MAX,
                sustain_percentage: 1.0,
                release_seconds: EnvelopePreset::MAX,
            },
        }
    }

    #[test]
    fn test_basic_synth_patch() {
        let message_on = MidiMessage::note_on_c4();
        let message_off = MidiMessage::note_off_c4();

        let mut clock = Clock::new(SAMPLE_RATE, 4, 4, 128.);
        let mut voice = Voice::new(SAMPLE_RATE, &test_patch());
        voice.handle_midi_message(&message_on, &clock);
        write_sound(
            &mut voice,
            &mut clock,
            5.0,
            &message_off,
            5.0,
            "voice_basic_test_c4",
        );
    }

    #[test]
    fn test_basic_cello_patch() {
        let message_on = MidiMessage::note_on_c4();
        let message_off = MidiMessage::note_off_c4();

        let mut clock = Clock::new(SAMPLE_RATE, 4, 4, 128.);
        let mut voice = Voice::new(SAMPLE_RATE, &cello_patch());
        voice.handle_midi_message(&message_on, &clock);
        write_sound(
            &mut voice,
            &mut clock,
            5.0,
            &message_off,
            1.0,
            "voice_cello_c4",
        );
    }
}
