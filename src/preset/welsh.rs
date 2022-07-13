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
    WheelsofSteel,
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
    pub filter_resonance: f32,
    pub filter_envelope_weight: f32,
    pub filter_envelope_preset: EnvelopePreset,

    pub amp_envelope_preset: EnvelopePreset,
}

impl WelshSynthPreset {
    pub fn by_name(name: WelshPresetName) -> Self {
        match name {
            WelshPresetName::Banjo => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: Waveform::Square(0.2),
                    tune: 1.0,
                    mix: 1.0,
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: Waveform::Square(0.1),
                    tune: 1.0,
                    mix: 1.0,
                },
                oscillator_2_track: true,
                oscillator_2_sync: true,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::Amplitude,
                    waveform: Waveform::Triangle,
                    frequency: 10.0,
                    depth: LfoPreset::percent(5.0),
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
                    waveform: Waveform::Square(0.1),
                    tune: 1.0,
                    mix: 1.0,
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: Waveform::Square(0.5),
                    tune: 1.0,
                    mix: 1.0,
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
                    release_seconds: 0.0,
                },
                amp_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.06,
                    decay_seconds: 0.0,
                    sustain_percentage: 1.0,
                    release_seconds: 0.3,
                },
            },
            WelshPresetName::DoubleBass => Self {
                oscillator_1_preset: OscillatorPreset { 
                    waveform: Waveform::Square(0.45), 
                    tune: -0.1, 
                    mix: 1.0,
                },
                oscillator_2_preset: OscillatorPreset { 
                    waveform:Waveform::Square(0.0), 
                    tune: 0.0, 
                    mix: 0.6, 
                },
                oscillator_2_sync: false,
                oscillator_2_track: true,
                noise: 0.0,
                lfo_preset: LfoPreset { 
                    routing: LfoRouting::Pitch,
                    waveform: Waveform::Triangle,
                    frequency: 5.0,
                    depth: 0.11,
                },
                glide: GlidePreset::Off,
                has_unison: false,
                polyphony: PolyphonyPreset::Multi,
                filter_type_24db: FilterPreset {
                    cutoff: 1.6,
                    weight: 0.63,
                },
                filter_type_12db: FilterPreset { 
                    cutoff: 750.0, 
                    weight: 0.52, 
                },
                filter_resonance: 0.0,
                filter_envelope_weight: 0.0,
                filter_envelope_preset: EnvelopePreset { attack_seconds: 0.0, 
                    decay_seconds: 0.0, 
                    sustain_percentage: 0.0, 
                    release_seconds: 0.0,
                },
                amp_envelope_preset: EnvelopePreset { attack_seconds: 0.35, decay_seconds: 0.0,
                    sustain_percentage: 1.0, release_seconds: 0.19 }


            },
            WelshPresetName::Dulcimer => {
                panic!()
            }
            WelshPresetName::GuitarAcoustic => {
                panic!()
            }
            WelshPresetName::GuitarElectric => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: Waveform::Square(0.2),
                    tune: 1.0,
                    mix: 0.65,
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: Waveform::Square(0.15),
                    tune: OscillatorPreset::semis_and_cents(10.0, 0.0),
                    mix: 1.0,
                },
                oscillator_2_track: true,
                oscillator_2_sync: true,
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
                    cutoff: 20.0,
                    weight: 1.0,
                },
                filter_type_12db: FilterPreset {
                    cutoff: 20.0,
                    weight: 1.0,
                },
                filter_resonance: 0.0,
                filter_envelope_weight: 0.0,
                filter_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 0.0,
                    sustain_percentage: 0.0,
                    release_seconds: 0.0,
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
            WelshPresetName::StandupBass => {
                panic!()
            }
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
            WelshPresetName::FrenchHorn => WelshSynthPreset {
                oscillator_1_preset: OscillatorPreset {
                    waveform: Waveform::Square(0.1),
                    tune: 1.0,
                    mix: 1.0,
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: Waveform::None,
                    tune: 1.0,
                    mix: 1.0,
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
                filter_resonance: 0.2,
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
                        waveform: Waveform::Square(0.3),
                        tune: 1.0,
                        mix: 1.0,
                    },
                    oscillator_2_preset: OscillatorPreset {
                        waveform: Waveform::Square(0.45),
                        tune: OscillatorPreset::semis_and_cents(8.0, 0.0),
                        mix: 0.75,
                    },
                    oscillator_2_track: true,
                    oscillator_2_sync: true,
                    noise: 0.0,
                    glide: GlidePreset::Off,
                    has_unison: false,
                    polyphony: PolyphonyPreset::Multi,
                    lfo_preset: LfoPreset {
                        routing: LfoRouting::Pitch, // TODO osc1/osc2 is an option
                        waveform: Waveform::Sine,
                        frequency: 7.5,
                        depth: LfoPreset::cents(10.0),
                    },
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
                        release_seconds: 0.0,
                    },
                    amp_envelope_preset: EnvelopePreset {
                        attack_seconds: 0.0,
                        decay_seconds: 0.0,
                        sustain_percentage: 1.0,
                        release_seconds: 0.3,
                    },
                }
            }
            WelshPresetName::Trombone => Self {
                oscillator_1_preset: OscillatorPreset {
                    waveform: Waveform::Sawtooth,
                    tune: 1.0,
                    mix: 1.0,
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: Waveform::Triangle,
                    tune: OscillatorPreset::octaves(1.0),
                    mix: 1.0,
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
                    decay_seconds: 0.0,
                    sustain_percentage: 1.0,
                    release_seconds: 0.18,
                },
                amp_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.06,
                    decay_seconds: 0.0,
                    sustain_percentage: 1.0,
                    release_seconds: 0.50,
                },
            },
            WelshPresetName::Trumpet => {
                panic!()
            }
            WelshPresetName::Tuba => {
                panic!()
            }
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
            WelshPresetName::Organ => {
                panic!()
            }
            WelshPresetName::Piano => {
                panic!()
            }
            // -------------------- Vocals
            WelshPresetName::Angels => WelshSynthPreset {
                oscillator_1_preset: OscillatorPreset {
                    waveform: Waveform::Sawtooth,
                    ..Default::default()
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: Waveform::None,
                    ..Default::default()
                },
                oscillator_2_track: false,
                oscillator_2_sync: false,
                noise: 0.0,
                lfo_preset: LfoPreset {
                    routing: LfoRouting::Pitch,
                    waveform: Waveform::Triangle,
                    frequency: 2.4,
                    depth: LfoPreset::cents(20.0),
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
                    attack_seconds: 0.,
                    decay_seconds: 0.,
                    sustain_percentage: 0.,
                    release_seconds: 0.,
                },
                amp_envelope_preset: EnvelopePreset {
                    attack_seconds: 0.32,
                    decay_seconds: 0.0,
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
            WelshPresetName::Bongos => {
                panic!()
            }
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
            WelshPresetName::Cymbal => {
                panic!()
            }
            WelshPresetName::SideStick => {
                panic!()
            }
            WelshPresetName::SnareDrum => {
                panic!()
            }
            WelshPresetName::Tambourine => {
                panic!()
            }
            WelshPresetName::WheelsofSteel => {
                panic!()
            }
            // -------------------- Leads
            WelshPresetName::BrassSection => {
                panic!()
            }
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
            WelshPresetName::StringsPwm => {
                panic!()
            }
            WelshPresetName::Trance5th => {
                panic!()
            }
            // -------------------- Bass
            WelshPresetName::AcidBass => {
                panic!()
            }
            WelshPresetName::BassOfTheTimeLords => {
                panic!()
            }
            WelshPresetName::DetroitBass => {
                panic!()
            }
            WelshPresetName::DeutscheBass => {
                panic!()
            }
            WelshPresetName::DigitalBass => {
                panic!()
            }
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
            WelshPresetName::Wind => {
                panic!()
            }

            _ => {
                panic!();
            }
        }
    }
}
