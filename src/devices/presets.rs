use crate::primitives::{
    envelopes::MiniEnvelopePreset,
    filter::MiniFilterType,
    oscillators::{LfoPreset, LfoRouting, OscillatorPreset, Waveform},
};

use super::{instruments::{MiniSynthPreset}, synthesizers::SuperSynth};

pub enum GeneralMidiProgram {
    AcousticGrand = 0,
    BrightAcoustic = 1,
    ElectricGrand = 2,
    HonkyTonk = 3,
    ElectricPiano1 = 4,
    ElectricPiano2 = 5,
    Harpsichord = 6,
    Clav = 7,
    Celesta = 8,
    Glockenspiel = 9,
    MusicBox = 10,
    Vibraphone = 11,
    Marimba = 12,
    Xylophone = 13,
    TubularBells = 14,
    Dulcimer = 15,
    DrawbarOrgan = 16,
    PercussiveOrgan = 17,
    RockOrgan = 18,
    ChurchOrgan = 19,
    ReedOrgan = 20,
    Accordion = 21,
    Harmonica = 22,
    TangoAccordion = 23,
    AcousticGuitar_Nylon = 24,
    AcousticGuitar_Steel = 25,
    ElectricGuitar_Jazz = 26,
    ElectricGuitar_Clean = 27,
    ElectricGuitar_Muted = 28,
    OverdrivenGuitar = 29,
    DistortionGuitar = 30,
    GuitarHarmonics = 31,
    AcousticBass = 32,
    ElectricBass_Finger = 33,
    ElectricBass_Pick = 34,
    FretlessBass = 35,
    SlapBass1 = 36,
    SlapBass2 = 37,
    SynthBass1 = 38,
    SynthBass2 = 39,
    Violin = 40,
    Viola = 41,
    Cello = 42,
    Contrabass = 43,
    TremoloStrings = 44,
    PizzicatoStrings = 45,
    OrchestralHarp = 46,
    Timpani = 47,
    StringEnsemble1 = 48,
    StringEnsemble2 = 49,
    Synthstrings1 = 50,
    Synthstrings2 = 51,
    ChoirAahs = 52,
    VoiceOohs = 53,
    SynthVoice = 54,
    OrchestraHit = 55,
    Trumpet = 56,
    Trombone = 57,
    Tuba = 58,
    MutedTrumpet = 59,
    FrenchHorn = 60,
    BrassSection = 61,
    Synthbrass1 = 62,
    Synthbrass2 = 63,
    SopranoSax = 64,
    AltoSax = 65,
    TenorSax = 66,
    BaritoneSax = 67,
    Oboe = 68,
    EnglishHorn = 69,
    Bassoon = 70,
    Clarinet = 71,
    Piccolo = 72,
    Flute = 73,
    Recorder = 74,
    PanFlute = 75,
    BlownBottle = 76,
    Shakuhachi = 77,
    Whistle = 78,
    Ocarina = 79,
    Lead1_Square = 80,
    Lead2_Sawtooth = 81,
    Lead3_Calliope = 82,
    Lead4_Chiff = 83,
    Lead5_Charang = 84,
    Lead6_Voice = 85,
    Lead7_Fifths = 86,
    Lead8_BassLead = 87,
    Pad1_NewAge = 88,
    Pad2_Warm = 89,
    Pad3_Polysynth = 90,
    Pad4_Choir = 91,
    Pad5_Bowed = 92,
    Pad6_Metallic = 93,
    Pad7_Halo = 94,
    Pad8_Sweep = 95,
    Fx1_Rain = 96,
    Fx2_Soundtrack = 97,
    Fx3_Crystal = 98,
    Fx4_Atmosphere = 99,
    Fx5_Brightness = 100,
    Fx6_Goblins = 101,
    Fx7_Echoes = 102,
    Fx8_SciFi = 103,
    Sitar = 104,
    Banjo = 105,
    Shamisen = 106,
    Koto = 107,
    Kalimba = 108,
    Bagpipe = 109,
    Fiddle = 110,
    Shanai = 111,
    TinkleBell = 112,
    Agogo = 113,
    SteelDrums = 114,
    Woodblock = 115,
    TaikoDrum = 116,
    MelodicTom = 117,
    SynthDrum = 118,
    ReverseCymbal = 119,
    GuitarFretNoise = 120,
    BreathNoise = 121,
    Seashore = 122,
    BirdTweet = 123,
    TelephoneRing = 124,
    Helicopter = 125,
    Applause = 126,
    Gunshot = 127,
}

impl SuperSynth {
    pub fn new_for_general_midi(sample_rate: u32, program: GeneralMidiProgram) -> Self {
        let preset = match program {
            GeneralMidiProgram::AcousticGrand => {
                // 1
                panic!();
            }
            GeneralMidiProgram::BrightAcoustic => {
                // 2
                panic!();
            }
            GeneralMidiProgram::ElectricGrand => {
                // 3
                panic!();
            }
            GeneralMidiProgram::HonkyTonk => {
                // 4
                panic!();
            }
            GeneralMidiProgram::ElectricPiano1 => {
                // 5
                panic!();
            }
            GeneralMidiProgram::ElectricPiano2 => {
                // 6
                panic!();
            }
            GeneralMidiProgram::Harpsichord => {
                // 7
                panic!();
            }
            GeneralMidiProgram::Clav => {
                // 8
                panic!();
            }
            GeneralMidiProgram::Celesta => {
                // 9
                panic!();
            }
            GeneralMidiProgram::Glockenspiel => {
                // 10
                panic!();
            }
            GeneralMidiProgram::MusicBox => {
                // 11
                panic!();
            }
            GeneralMidiProgram::Vibraphone => {
                // 12
                panic!();
            }
            GeneralMidiProgram::Marimba => {
                // 13
                panic!();
            }
            GeneralMidiProgram::Xylophone => {
                // 14
                panic!();
            }
            GeneralMidiProgram::TubularBells => {
                // 15
                panic!();
            }
            GeneralMidiProgram::Dulcimer => {
                // 16
                panic!();
            }
            GeneralMidiProgram::DrawbarOrgan => {
                // 17
                panic!();
            }
            GeneralMidiProgram::PercussiveOrgan => {
                // 18
                panic!();
            }
            GeneralMidiProgram::RockOrgan => {
                // 19
                panic!();
            }
            GeneralMidiProgram::ChurchOrgan => {
                // 20
                panic!();
            }
            GeneralMidiProgram::ReedOrgan => {
                // 21
                panic!();
            }
            GeneralMidiProgram::Accordion => {
                // 22
                panic!();
            }
            GeneralMidiProgram::Harmonica => {
                // 23
                panic!();
            }
            GeneralMidiProgram::TangoAccordion => {
                // 24
                panic!();
            }
            GeneralMidiProgram::AcousticGuitar_Nylon => {
                // 25
                panic!();
            }
            GeneralMidiProgram::AcousticGuitar_Steel => {
                // 26
                panic!();
            }
            GeneralMidiProgram::ElectricGuitar_Jazz => {
                // 27
                panic!();
            }
            GeneralMidiProgram::ElectricGuitar_Clean => {
                // 28
                panic!();
            }
            GeneralMidiProgram::ElectricGuitar_Muted => {
                // 29
                panic!();
            }
            GeneralMidiProgram::OverdrivenGuitar => {
                // 30
                panic!();
            }
            GeneralMidiProgram::DistortionGuitar => {
                // 31
                panic!();
            }
            GeneralMidiProgram::GuitarHarmonics => {
                // 32
                panic!();
            }
            GeneralMidiProgram::AcousticBass => {
                // 33
                panic!();
            }
            GeneralMidiProgram::ElectricBass_Finger => {
                // 34
                panic!();
            }
            GeneralMidiProgram::ElectricBass_Pick => {
                // 35
                panic!();
            }
            GeneralMidiProgram::FretlessBass => {
                // 36
                panic!();
            }
            GeneralMidiProgram::SlapBass1 => {
                // 37
                panic!();
            }
            GeneralMidiProgram::SlapBass2 => {
                // 38
                panic!();
            }
            GeneralMidiProgram::SynthBass1 => {
                // 39
                panic!();
            }
            GeneralMidiProgram::SynthBass2 => {
                // 40
                panic!();
            }
            GeneralMidiProgram::Violin => {
                // 41
                panic!();
            }
            GeneralMidiProgram::Viola => {
                // 42
                panic!();
            }
            GeneralMidiProgram::Cello => {
                // 43
                MiniSynthPreset {
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
                    amp_envelope_preset: MiniEnvelopePreset {
                        attack_seconds: 0.06,
                        decay_seconds: 0.0,
                        sustain_percentage: 1.0,
                        release_seconds: 0.3,
                    },
                    lfo_preset: LfoPreset {
                        routing: LfoRouting::Amplitude,
                        waveform: Waveform::Sine,
                        frequency: 7.5,
                        depth: 0.05,
                    },
                    filter_24db_type: MiniFilterType::FourthOrderLowPass(300.),
                    filter_12db_type: MiniFilterType::SecondOrderLowPass(40., 0.),
                    filter_24db_weight: 0.9,
                    filter_12db_weight: 0.1,
                    filter_envelope_preset: MiniEnvelopePreset {
                        attack_seconds: 0.0,
                        decay_seconds: 3.29,
                        sustain_percentage: 0.78,
                        release_seconds: 0.0,
                    },
                    filter_envelope_weight: 0.9,
                }
            }
            GeneralMidiProgram::Contrabass => {
                // 44
                panic!();
            }
            GeneralMidiProgram::TremoloStrings => {
                // 45
                panic!();
            }
            GeneralMidiProgram::PizzicatoStrings => {
                // 46
                panic!();
            }
            GeneralMidiProgram::OrchestralHarp => {
                // 47
                panic!();
            }
            GeneralMidiProgram::Timpani => {
                // 48
                panic!();
            }
            GeneralMidiProgram::StringEnsemble1 => {
                // 49
                panic!();
            }
            GeneralMidiProgram::StringEnsemble2 => {
                // 50
                panic!();
            }
            GeneralMidiProgram::Synthstrings1 => {
                // 51
                panic!();
            }
            GeneralMidiProgram::Synthstrings2 => {
                // 52
                panic!();
            }
            GeneralMidiProgram::ChoirAahs => {
                // 53
                panic!();
            }
            GeneralMidiProgram::ChoirAahs => {
                // 53
                MiniSynthPreset {
                    oscillator_1_preset: OscillatorPreset {
                        waveform: Waveform::Sawtooth,
                        ..Default::default()
                    },
                    oscillator_2_preset: OscillatorPreset {
                        waveform: Waveform::None,
                        ..Default::default()
                    },
                    amp_envelope_preset: MiniEnvelopePreset {
                        attack_seconds: 0.32,
                        decay_seconds: 0.0,
                        sustain_percentage: 1.0,
                        release_seconds: 0.93,
                    },
                    lfo_preset: LfoPreset {
                        routing: LfoRouting::None,
                        waveform: Waveform::Triangle,
                        frequency: 2.4,
                        depth: 0.0000119, // TODO 20 cents
                    },
                    filter_24db_type: MiniFilterType::FourthOrderLowPass(900.), // TODO: map Q to %
                    filter_12db_type: MiniFilterType::SecondOrderLowPass(900., 1.0),
                    filter_24db_weight: 0.85,
                    filter_12db_weight: 0.25,
                    filter_envelope_preset: MiniEnvelopePreset {
                        attack_seconds: 0.,
                        decay_seconds: 0.,
                        sustain_percentage: 0.,
                        release_seconds: 0.,
                    },
                    filter_envelope_weight: 0.0,
                }
            }

            GeneralMidiProgram::VoiceOohs => {
                // 54
                panic!();
            }
            GeneralMidiProgram::SynthVoice => {
                // 55
                panic!();
            }
            GeneralMidiProgram::OrchestraHit => {
                // 56
                panic!();
            }
            GeneralMidiProgram::Trumpet => {
                // 57
                panic!();
            }
            GeneralMidiProgram::Trombone => {
                // 58
                panic!();
            }
            GeneralMidiProgram::Tuba => {
                // 59
                panic!();
            }
            GeneralMidiProgram::MutedTrumpet => {
                // 60
                panic!();
            }
            GeneralMidiProgram::FrenchHorn => {
                // 61
                panic!();
            }
            GeneralMidiProgram::BrassSection => {
                // 62
                panic!();
            }
            GeneralMidiProgram::Synthbrass1 => {
                // 63
                panic!();
            }
            GeneralMidiProgram::Synthbrass2 => {
                // 64
                panic!();
            }
            GeneralMidiProgram::SopranoSax => {
                // 65
                panic!();
            }
            GeneralMidiProgram::AltoSax => {
                // 66
                panic!();
            }
            GeneralMidiProgram::TenorSax => {
                // 67
                panic!();
            }
            GeneralMidiProgram::BaritoneSax => {
                // 68
                panic!();
            }
            GeneralMidiProgram::Oboe => {
                // 69
                panic!();
            }
            GeneralMidiProgram::EnglishHorn => {
                // 70
                panic!();
            }
            GeneralMidiProgram::Bassoon => {
                // 71
                panic!();
            }
            GeneralMidiProgram::Clarinet => {
                // 72
                panic!();
            }
            GeneralMidiProgram::Piccolo => {
                // 73
                panic!();
            }
            GeneralMidiProgram::Flute => {
                // 74
                panic!();
            }
            GeneralMidiProgram::Recorder => {
                // 75
                panic!();
            }
            GeneralMidiProgram::PanFlute => {
                // 76
                panic!();
            }
            GeneralMidiProgram::BlownBottle => {
                // 77
                panic!();
            }
            GeneralMidiProgram::Shakuhachi => {
                // 78
                panic!();
            }
            GeneralMidiProgram::Whistle => {
                // 79
                panic!();
            }
            GeneralMidiProgram::Ocarina => {
                // 80
                panic!();
            }
            GeneralMidiProgram::Lead1_Square => {
                // 81
                panic!();
            }
            GeneralMidiProgram::Lead2_Sawtooth => {
                // 82
                panic!();
            }
            GeneralMidiProgram::Lead3_Calliope => {
                // 83
                panic!();
            }
            GeneralMidiProgram::Lead4_Chiff => {
                // 84
                panic!();
            }
            GeneralMidiProgram::Lead5_Charang => {
                // 85
                panic!();
            }
            GeneralMidiProgram::Lead6_Voice => {
                // 86
                panic!();
            }
            GeneralMidiProgram::Lead7_Fifths => {
                // 87
                panic!();
            }
            GeneralMidiProgram::Lead8_BassLead => {
                // 88
                panic!();
            }
            GeneralMidiProgram::Pad1_NewAge => {
                // 89
                panic!();
            }
            GeneralMidiProgram::Pad2_Warm => {
                // 90
                panic!();
            }
            GeneralMidiProgram::Pad3_Polysynth => {
                // 91
                panic!();
            }
            GeneralMidiProgram::Pad4_Choir => {
                // 92
                panic!();
            }
            GeneralMidiProgram::Pad5_Bowed => {
                // 93
                panic!();
            }
            GeneralMidiProgram::Pad6_Metallic => {
                // 94
                panic!();
            }
            GeneralMidiProgram::Pad7_Halo => {
                // 95
                panic!();
            }
            GeneralMidiProgram::Pad8_Sweep => {
                // 96
                panic!();
            }
            GeneralMidiProgram::Fx1_Rain => {
                // 97
                panic!();
            }
            GeneralMidiProgram::Fx2_Soundtrack => {
                // 98
                panic!();
            }
            GeneralMidiProgram::Fx3_Crystal => {
                // 99
                panic!();
            }
            GeneralMidiProgram::Fx4_Atmosphere => {
                // 100
                panic!();
            }
            GeneralMidiProgram::Fx5_Brightness => {
                // 101
                panic!();
            }
            GeneralMidiProgram::Fx6_Goblins => {
                // 102
                panic!();
            }
            GeneralMidiProgram::Fx7_Echoes => {
                // 103
                panic!();
            }
            GeneralMidiProgram::Fx8_SciFi => {
                // 104
                panic!();
            }
            GeneralMidiProgram::Sitar => {
                // 105
                panic!();
            }
            GeneralMidiProgram::Banjo => {
                // 106
                panic!();
            }
            GeneralMidiProgram::Shamisen => {
                // 107
                panic!();
            }
            GeneralMidiProgram::Koto => {
                // 108
                panic!();
            }
            GeneralMidiProgram::Kalimba => {
                // 109
                panic!();
            }
            GeneralMidiProgram::Bagpipe => {
                // 110
                panic!();
            }
            GeneralMidiProgram::Fiddle => {
                // 111
                panic!();
            }
            GeneralMidiProgram::Shanai => {
                // 112
                panic!();
            }
            GeneralMidiProgram::TinkleBell => {
                // 113
                panic!();
            }
            GeneralMidiProgram::Agogo => {
                // 114
                panic!();
            }
            GeneralMidiProgram::SteelDrums => {
                // 115
                panic!();
            }
            GeneralMidiProgram::Woodblock => {
                // 116
                panic!();
            }
            GeneralMidiProgram::TaikoDrum => {
                // 117
                panic!();
            }
            GeneralMidiProgram::MelodicTom => {
                // 118
                panic!();
            }
            GeneralMidiProgram::SynthDrum => {
                // 119
                panic!();
            }
            GeneralMidiProgram::ReverseCymbal => {
                // 120
                panic!();
            }
            GeneralMidiProgram::GuitarFretNoise => {
                // 121
                panic!();
            }
            GeneralMidiProgram::BreathNoise => {
                // 122
                panic!();
            }
            GeneralMidiProgram::Seashore => {
                // 123
                panic!();
            }
            GeneralMidiProgram::BirdTweet => {
                // 124
                panic!();
            }
            GeneralMidiProgram::TelephoneRing => {
                // 125
                panic!();
            }
            GeneralMidiProgram::Helicopter => {
                // 126
                panic!();
            }
            GeneralMidiProgram::Applause => {
                // 127
                panic!();
            }
            GeneralMidiProgram::Gunshot => {
                // 128
                panic!();
            }
        };
        Self::new(sample_rate, preset)
    }
}
