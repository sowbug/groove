use crate::primitives::oscillators::Waveform;

use super::{EnvelopePreset, FilterPreset, LfoPreset, LfoRouting, OscillatorPreset};

pub enum WelshPresetName {
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
pub struct WelshSynthPreset {
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

impl WelshSynthPreset {
    pub fn by_name(name: WelshPresetName) -> Self {
        match name {
            WelshPresetName::Banjo => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: Waveform::PulseWidth(0.2),
                    ..Default::default()
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: Waveform::PulseWidth(0.1),
                    tune: OscillatorPreset::semis_and_cents(5.0, 0.0),
                    mix: 0.80,
                },
                oscillator_2_track: true,
                oscillator_2_sync: true,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::Amplitude,
                    waveform: Waveform::Triangle,
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
            WelshPresetName::Cello => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: Waveform::PulseWidth(0.1),
                    ..Default::default()
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: Waveform::Square,
                    ..Default::default()
                },
                oscillator_2_track: true,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::Amplitude,
                    waveform: Waveform::Sine,
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
            WelshPresetName::DoubleBass => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: Waveform::PulseWidth(0.45),
                    tune: OscillatorPreset::octaves(-1.0),
                    ..Default::default()
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: Waveform::Square,
                    mix: 0.6,
                    ..Default::default()
                },
                oscillator_2_track: true,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::Pitch,
                    waveform: Waveform::Triangle,
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
            WelshPresetName::Dulcimer => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: Waveform::PulseWidth(0.25),
                    tune: OscillatorPreset::semis_and_cents(-7.0, 0.0),
                    mix: 0.80,
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: Waveform::PulseWidth(0.05),
                    ..Default::default()
                },
                oscillator_2_track: true,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::Amplitude,
                    waveform: Waveform::Triangle,
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

            WelshPresetName::GuitarAcoustic => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: Waveform::PulseWidth(0.25),
                    ..Default::default()
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: Waveform::PulseWidth(0.10),
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

            WelshPresetName::GuitarElectric => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: Waveform::PulseWidth(0.2),
                    tune: OscillatorPreset::NATURAL_TUNING,
                    mix: 0.65,
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: Waveform::PulseWidth(0.15),
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
            WelshPresetName::Harp => {
                panic!()
            }
            WelshPresetName::HurdyGurdy => {
                panic!()
            }
            WelshPresetName::Kora => {
                panic!()
            }
            WelshPresetName::Lute => {
                panic!()
            }
            WelshPresetName::Mandocello => {
                panic!()
            }
            WelshPresetName::Mandolin => {
                panic!()
            }
            WelshPresetName::Riti => {
                panic!()
            }
            WelshPresetName::Sitar => {
                panic!()
            }
            WelshPresetName::StandupBass => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: Waveform::PulseWidth(0.25),
                    tune: OscillatorPreset::octaves(-1.0),
                    mix: OscillatorPreset::FULL_MIX,
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: Waveform::Triangle,
                    tune: OscillatorPreset::octaves(-1.0),
                    mix: OscillatorPreset::FULL_MIX,
                },
                oscillator_2_track: true,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::Amplitude,
                    waveform: Waveform::Triangle,
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
            WelshPresetName::Viola => {
                panic!()
            }
            WelshPresetName::Violin => {
                panic!()
            }
            // -------------------- Woodwinds
            WelshPresetName::Bagpipes => {
                panic!()
            }
            WelshPresetName::BassClarinet => {
                panic!()
            }
            WelshPresetName::Bassoon => {
                panic!()
            }
            WelshPresetName::Clarinet => {
                panic!()
            }
            WelshPresetName::ConchShell => {
                panic!()
            }
            WelshPresetName::Contrabassoon => {
                panic!()
            }
            WelshPresetName::Digeridoo => {
                panic!()
            }
            WelshPresetName::EnglishHorn => {
                panic!()
            }
            WelshPresetName::Flute => {
                panic!()
            }
            WelshPresetName::Oboe => {
                panic!()
            }
            WelshPresetName::Piccolo => {
                panic!()
            }
            // -------------------- Brass
            WelshPresetName::FrenchHorn => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: Waveform::PulseWidth(0.1),
                    ..Default::default()
                },
                oscillator_2_preset: OscillatorPreset {
                    ..Default::default()
                },
                oscillator_2_track: false,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::None,
                    waveform: Waveform::None,
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
            WelshPresetName::Harmonica => {
                panic!()
            }
            WelshPresetName::PennyWhistle => {
                panic!()
            }
            WelshPresetName::Saxophone => {
                WelshSynthPreset {
                    oscillator_1_preset: OscillatorPreset {
                        waveform: Waveform::PulseWidth(0.3),
                        ..Default::default()
                    },
                    oscillator_2_preset: OscillatorPreset {
                        waveform: Waveform::PulseWidth(0.45),
                        tune: OscillatorPreset::semis_and_cents(8.0, 0.0),
                        mix: 0.75,
                    },
                    oscillator_2_track: true,
                    oscillator_2_sync: true,
                    noise: 0.0,
                    lfo_preset: LfoPreset {
                        routing: LfoRouting::Pitch, // TODO osc1/osc2 is an option
                        waveform: Waveform::Sine,
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
            WelshPresetName::Trombone => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: Waveform::Sawtooth,
                    ..Default::default()
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: Waveform::Triangle,
                    tune: OscillatorPreset::octaves(1.0),
                    mix: OscillatorPreset::FULL_MIX,
                },
                oscillator_2_track: true,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::Amplitude,
                    waveform: Waveform::Triangle,
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
            WelshPresetName::Trumpet => {
                panic!()
            }
            WelshPresetName::Tuba => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: Waveform::Sawtooth,
                    tune: OscillatorPreset::NATURAL_TUNING,
                    mix: 0.85,
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: Waveform::Sawtooth,
                    tune: OscillatorPreset::octaves(-1.0),
                    mix: OscillatorPreset::FULL_MIX,
                },
                oscillator_2_track: true,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::Amplitude,
                    waveform: Waveform::Triangle,
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
            WelshPresetName::Accordion => {
                panic!()
            }
            WelshPresetName::Celeste => {
                panic!()
            }
            WelshPresetName::Clavichord => {
                panic!()
            }
            WelshPresetName::ElectricPiano => {
                panic!()
            }
            WelshPresetName::Harpsichord => {
                panic!()
            }
            WelshPresetName::Organ => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: Waveform::Triangle,
                    ..Default::default()
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: Waveform::Triangle,
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
            WelshPresetName::Piano => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: Waveform::Sawtooth,
                    mix: 0.75,
                    ..Default::default()
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: Waveform::PulseWidth(0.15),
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
            WelshPresetName::Angels => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: Waveform::Sawtooth,
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
                    waveform: Waveform::Triangle,
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
            WelshPresetName::Choir => {
                panic!()
            }
            WelshPresetName::VocalFemale => {
                panic!()
            }
            WelshPresetName::VocalMale => {
                panic!()
            }
            WelshPresetName::Whistling => {
                panic!()
            }
            // -------------------- Tuned Percussion
            WelshPresetName::Bell => {
                panic!()
            }
            WelshPresetName::Bongos => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: Waveform::Triangle,
                    ..Default::default()
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: Waveform::Square,
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
            WelshPresetName::Conga => {
                panic!()
            }
            WelshPresetName::Glockenspiel => {
                panic!()
            }
            WelshPresetName::Marimba => {
                panic!()
            }
            WelshPresetName::Timpani => {
                panic!()
            }
            WelshPresetName::Xylophone => {
                panic!()
            }
            // -------------------- Untuned Percussion
            WelshPresetName::BassDrum => {
                panic!()
            }
            WelshPresetName::Castanets => {
                panic!()
            }
            WelshPresetName::Clap => {
                panic!()
            }
            WelshPresetName::Claves => {
                panic!()
            }
            WelshPresetName::Cowbell => {
                panic!()
            }
            WelshPresetName::CowbellAnalog => {
                panic!()
            }
            WelshPresetName::Cymbal => Self {
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
            WelshPresetName::SideStick => Self {
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
            WelshPresetName::SnareDrum => {
                panic!()
            }
            WelshPresetName::Tambourine => {
                panic!()
            }
            WelshPresetName::WheelsOfSteel => {
                panic!()
            }
            // -------------------- Leads
            WelshPresetName::BrassSection => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: Waveform::Square,
                    tune: OscillatorPreset::semis_and_cents(0.0, -10.0),
                    mix: OscillatorPreset::FULL_MIX,
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: Waveform::PulseWidth(0.2),
                    tune: OscillatorPreset::semis_and_cents(12.0, 10.0),
                    mix: OscillatorPreset::FULL_MIX,
                },
                oscillator_2_track: true,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::PulseWidth,
                    waveform: Waveform::Triangle,
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
            WelshPresetName::Mellow70sLead => {
                panic!()
            }
            WelshPresetName::MonoSolo => {
                panic!()
            }
            WelshPresetName::NewAgeLead => {
                panic!()
            }
            WelshPresetName::RAndBSlide => {
                panic!()
            }
            WelshPresetName::ScreamingSync => {
                panic!()
            }
            WelshPresetName::StringsPwm => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: Waveform::Square,
                    tune: OscillatorPreset::semis_and_cents(0.0, -10.0),
                    mix: OscillatorPreset::FULL_MIX,
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: Waveform::Square,
                    tune: OscillatorPreset::semis_and_cents(0.0, 10.0),
                    mix: OscillatorPreset::FULL_MIX,
                },
                oscillator_2_track: true,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::PulseWidth,
                    waveform: Waveform::Sine,
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
            WelshPresetName::Trance5th => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: Waveform::Square,
                    ..Default::default()
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: Waveform::Square,
                    tune: OscillatorPreset::semis_and_cents(7.0, 0.0),
                    ..Default::default()
                },
                oscillator_2_track: true,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::PulseWidth,
                    waveform: Waveform::Triangle, // TODO: this should be two different waveforms, one for each osc
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
            WelshPresetName::AcidBass => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: Waveform::PulseWidth(0.25),
                    tune: OscillatorPreset::semis_and_cents(0.0, 10.),
                    mix: 0.7,
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: Waveform::Square,
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
            WelshPresetName::BassOfTheTimeLords => {
                panic!()
            }
            WelshPresetName::DetroitBass => {
                panic!()
            }
            WelshPresetName::DeutscheBass => {
                panic!()
            }
            WelshPresetName::DigitalBass => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: Waveform::Square,
                    tune: OscillatorPreset::octaves(-1.0),
                    mix: 0.85,
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: Waveform::Sawtooth,
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
            WelshPresetName::FunkBass => {
                panic!()
            }
            WelshPresetName::GrowlingBass => {
                panic!()
            }
            WelshPresetName::RezBass => {
                panic!()
            }
            // -------------------- Pads
            WelshPresetName::AndroidDreams => {
                panic!()
            }
            WelshPresetName::CelestialWash => {
                panic!()
            }
            WelshPresetName::DarkCity => {
                panic!()
            }
            WelshPresetName::Aurora => {
                panic!()
            }
            WelshPresetName::GalacticCathedral => {
                panic!()
            }
            WelshPresetName::GalacticChapel => {
                panic!()
            }
            WelshPresetName::Portus => {
                panic!()
            }
            WelshPresetName::PostApocalypticSyncSweep => {
                panic!()
            }
            WelshPresetName::TerraEnceladus => {
                panic!()
            }
            // -------------------- Sound Effects
            WelshPresetName::Cat => {
                panic!()
            }
            WelshPresetName::DigitalAlarmClock => {
                panic!()
            }
            WelshPresetName::JourneyToTheCore => {
                panic!()
            }
            WelshPresetName::Kazoo => {
                panic!()
            }
            WelshPresetName::Laser => {
                panic!()
            }
            WelshPresetName::Motor => {
                panic!()
            }
            WelshPresetName::NerdOTron2000 => {
                panic!()
            }
            WelshPresetName::OceanWavesWithFoghorn => {
                panic!()
            }
            WelshPresetName::PositronicRhythm => {
                panic!()
            }
            WelshPresetName::SpaceAttack => {
                panic!()
            }
            WelshPresetName::Toad => {
                panic!()
            }
            WelshPresetName::Wind => Self {
                noise: 1.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::Amplitude,
                    waveform: Waveform::Noise,
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
