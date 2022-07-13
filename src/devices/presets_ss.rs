use crate::{
    preset::welsh::{WelshPresetName, WelshSynthPreset},
    primitives::{
        filter::{MiniFilter2, MiniFilter2Type},
        oscillators::Waveform,
    },
};

use super::{instruments::SuperSynthPreset, synthesizers::SuperSynth};

#[allow(dead_code)]
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
    AcousticGuitarNylon = 24,
    AcousticGuitarSteel = 25,
    ElectricGuitarJazz = 26,
    ElectricGuitarClean = 27,
    ElectricGuitarMuted = 28,
    OverdrivenGuitar = 29,
    DistortionGuitar = 30,
    GuitarHarmonics = 31,
    AcousticBass = 32,
    ElectricBassFinger = 33,
    ElectricBassPick = 34,
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
    Lead1Square = 80,
    Lead2Sawtooth = 81,
    Lead3Calliope = 82,
    Lead4Chiff = 83,
    Lead5Charang = 84,
    Lead6Voice = 85,
    Lead7Fifths = 86,
    Lead8BassLead = 87,
    Pad1NewAge = 88,
    Pad2Warm = 89,
    Pad3Polysynth = 90,
    Pad4Choir = 91,
    Pad5Bowed = 92,
    Pad6Metallic = 93,
    Pad7Halo = 94,
    Pad8Sweep = 95,
    Fx1Rain = 96,
    Fx2Soundtrack = 97,
    Fx3Crystal = 98,
    Fx4Atmosphere = 99,
    Fx5Brightness = 100,
    Fx6Goblins = 101,
    Fx7Echoes = 102,
    Fx8SciFi = 103,
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
        Self::new(sample_rate, Self::get_general_midi_preset(program))
    }

    pub fn get_general_midi_preset(program: GeneralMidiProgram) -> WelshSynthPreset {
        match program {
            GeneralMidiProgram::AcousticGrand => {
                // 1
                WelshSynthPreset::by_name(WelshPresetName::Piano)
            }
            GeneralMidiProgram::BrightAcoustic => {
                // 2
                WelshSynthPreset::by_name(WelshPresetName::Piano) // TODO dup
            }
            GeneralMidiProgram::ElectricGrand => {
                // 3
                WelshSynthPreset::by_name(WelshPresetName::ElectricPiano)
            }
            GeneralMidiProgram::HonkyTonk => {
                // 4
                panic!();
            }
            GeneralMidiProgram::ElectricPiano1 => {
                // 5
                WelshSynthPreset::by_name(WelshPresetName::ElectricPiano) // TODO dup
            }
            GeneralMidiProgram::ElectricPiano2 => {
                // 6
                WelshSynthPreset::by_name(WelshPresetName::ElectricPiano) // TODO dup
            }
            GeneralMidiProgram::Harpsichord => {
                // 7
                WelshSynthPreset::by_name(WelshPresetName::Harpsichord)
            }
            GeneralMidiProgram::Clav => {
                // 8
                WelshSynthPreset::by_name(WelshPresetName::Clavichord)
            }
            GeneralMidiProgram::Celesta => {
                // 9
                WelshSynthPreset::by_name(WelshPresetName::Celeste)
            }
            GeneralMidiProgram::Glockenspiel => {
                // 10
                WelshSynthPreset::by_name(WelshPresetName::Glockenspiel)
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
                WelshSynthPreset::by_name(WelshPresetName::Marimba)
            }
            GeneralMidiProgram::Xylophone => {
                // 14
                WelshSynthPreset::by_name(WelshPresetName::Xylophone)
            }
            GeneralMidiProgram::TubularBells => {
                // 15
                WelshSynthPreset::by_name(WelshPresetName::Bell)
            }
            GeneralMidiProgram::Dulcimer => {
                // 16
                WelshSynthPreset::by_name(WelshPresetName::Dulcimer)
            }
            GeneralMidiProgram::DrawbarOrgan => {
                // 17
                WelshSynthPreset::by_name(WelshPresetName::Organ) // TODO dup
            }
            GeneralMidiProgram::PercussiveOrgan => {
                // 18
                WelshSynthPreset::by_name(WelshPresetName::Organ) // TODO dup
            }
            GeneralMidiProgram::RockOrgan => {
                // 19
                WelshSynthPreset::by_name(WelshPresetName::Organ) // TODO dup
            }
            GeneralMidiProgram::ChurchOrgan => {
                // 20
                WelshSynthPreset::by_name(WelshPresetName::Organ) // TODO dup
            }
            GeneralMidiProgram::ReedOrgan => {
                // 21
                WelshSynthPreset::by_name(WelshPresetName::Organ) // TODO dup
            }
            GeneralMidiProgram::Accordion => {
                // 22
                WelshSynthPreset::by_name(WelshPresetName::Accordion)
            }
            GeneralMidiProgram::Harmonica => {
                // 23
                WelshSynthPreset::by_name(WelshPresetName::Harmonica)
            }
            GeneralMidiProgram::TangoAccordion => {
                // 24
                panic!();
            }
            GeneralMidiProgram::AcousticGuitarNylon => {
                // 25
                WelshSynthPreset::by_name(WelshPresetName::GuitarAcoustic)
            }
            GeneralMidiProgram::AcousticGuitarSteel => {
                // 26
                WelshSynthPreset::by_name(WelshPresetName::GuitarAcoustic) // TODO dup
            }
            GeneralMidiProgram::ElectricGuitarJazz => {
                // 27
                WelshSynthPreset::by_name(WelshPresetName::GuitarElectric) // TODO dup
            }
            GeneralMidiProgram::ElectricGuitarClean => {
                // 28
                WelshSynthPreset::by_name(WelshPresetName::GuitarElectric) // TODO dup
            }
            GeneralMidiProgram::ElectricGuitarMuted => {
                // 29
                WelshSynthPreset::by_name(WelshPresetName::GuitarElectric) // TODO dup
            }
            GeneralMidiProgram::OverdrivenGuitar => {
                // 30
                WelshSynthPreset::by_name(WelshPresetName::GuitarElectric) // TODO dup
            }
            GeneralMidiProgram::DistortionGuitar => {
                // 31
                WelshSynthPreset::by_name(WelshPresetName::GuitarElectric) // TODO dup
            }
            GeneralMidiProgram::GuitarHarmonics => {
                // 32
                WelshSynthPreset::by_name(WelshPresetName::GuitarElectric) // TODO dup
            }
            GeneralMidiProgram::AcousticBass => {
                // 33
                WelshSynthPreset::by_name(WelshPresetName::DoubleBass)
            }
            GeneralMidiProgram::ElectricBassFinger => {
                // 34
                WelshSynthPreset::by_name(WelshPresetName::StandupBass)
            }
            GeneralMidiProgram::ElectricBassPick => {
                // 35
                WelshSynthPreset::by_name(WelshPresetName::AcidBass)

            }
            GeneralMidiProgram::FretlessBass => {
                // 36
                WelshSynthPreset::by_name(WelshPresetName::DetroitBass) // TODO same?
            }
            GeneralMidiProgram::SlapBass1 => {
                // 37
                WelshSynthPreset::by_name(WelshPresetName::FunkBass)
            }
            GeneralMidiProgram::SlapBass2 => {
                // 38
                WelshSynthPreset::by_name(WelshPresetName::FunkBass)
            }
            GeneralMidiProgram::SynthBass1 => {
                // 39
                WelshSynthPreset::by_name(WelshPresetName::DigitalBass)
            }
            GeneralMidiProgram::SynthBass2 => {
                // 40
                WelshSynthPreset::by_name(WelshPresetName::DigitalBass)
            }
            GeneralMidiProgram::Violin => {
                // 41
                WelshSynthPreset::by_name(WelshPresetName::Violin)
            }
            GeneralMidiProgram::Viola => {
                // 42
                WelshSynthPreset::by_name(WelshPresetName::Viola)
            }
            GeneralMidiProgram::Cello => {
                // 43
                WelshSynthPreset::by_name(crate::preset::welsh::WelshPresetName::Cello)
            }
            GeneralMidiProgram::Contrabass => {
                // 44
                WelshSynthPreset::by_name(WelshPresetName::Contrabassoon)
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
                WelshSynthPreset::by_name(WelshPresetName::Harp)
            }
            GeneralMidiProgram::Timpani => {
                // 48
                WelshSynthPreset::by_name(WelshPresetName::Timpani)
            }
            GeneralMidiProgram::StringEnsemble1 => {
                // 49
                panic!();
            }
            GeneralMidiProgram::StringEnsemble2 => {
                // 50
                WelshSynthPreset::by_name(WelshPresetName::StringsPwm) // TODO same?

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
                WelshSynthPreset::by_name(WelshPresetName::Angels)
            }

            GeneralMidiProgram::VoiceOohs => {
                // 54
                WelshSynthPreset::by_name(WelshPresetName::Choir)
            }
            GeneralMidiProgram::SynthVoice => {
                // 55
                WelshSynthPreset::by_name(WelshPresetName::VocalFemale)
            }
            GeneralMidiProgram::OrchestraHit => {
                // 56
                panic!();
            }
            GeneralMidiProgram::Trumpet => {
                // 57
                WelshSynthPreset::by_name(WelshPresetName::Trumpet)
            }
            GeneralMidiProgram::Trombone => {
                // 58
                WelshSynthPreset::by_name(WelshPresetName::Trombone)
            }
            GeneralMidiProgram::Tuba => {
                // 59
                WelshSynthPreset::by_name(WelshPresetName::Tuba)
            }
            GeneralMidiProgram::MutedTrumpet => {
                // 60
                panic!();
            }
            GeneralMidiProgram::FrenchHorn => {
                // 61
                WelshSynthPreset::by_name(WelshPresetName::FrenchHorn)
            }
            GeneralMidiProgram::BrassSection => {
                // 62
                WelshSynthPreset::by_name(WelshPresetName::BrassSection)
            }
            GeneralMidiProgram::Synthbrass1 => {
                // 63
                WelshSynthPreset::by_name(WelshPresetName::BrassSection) // TODO dup
            }
            GeneralMidiProgram::Synthbrass2 => {
                // 64
                WelshSynthPreset::by_name(WelshPresetName::BrassSection) // TODO dup
            }
            GeneralMidiProgram::SopranoSax => {
                // 65
                WelshSynthPreset::by_name(WelshPresetName::Saxophone) // TODO dup
            }
            GeneralMidiProgram::AltoSax => {
                // 66
                WelshSynthPreset::by_name(WelshPresetName::Saxophone)
            }
            GeneralMidiProgram::TenorSax => {
                // 67
                WelshSynthPreset::by_name(WelshPresetName::Saxophone) // TODO dup
            }
            GeneralMidiProgram::BaritoneSax => {
                // 68
                WelshSynthPreset::by_name(WelshPresetName::Saxophone) // TODO dup
            }
            GeneralMidiProgram::Oboe => {
                // 69
                WelshSynthPreset::by_name(WelshPresetName::Oboe)
            }
            GeneralMidiProgram::EnglishHorn => {
                // 70
                WelshSynthPreset::by_name(WelshPresetName::EnglishHorn)
            }
            GeneralMidiProgram::Bassoon => {
                // 71
                WelshSynthPreset::by_name(WelshPresetName::Bassoon)
            }
            GeneralMidiProgram::Clarinet => {
                // 72
                WelshSynthPreset::by_name(WelshPresetName::Clarinet)
            }
            GeneralMidiProgram::Piccolo => {
                // 73
                WelshSynthPreset::by_name(WelshPresetName::Piccolo)
            }
            GeneralMidiProgram::Flute => {
                // 74
                WelshSynthPreset::by_name(WelshPresetName::Flute)
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
            GeneralMidiProgram::Lead1Square => {
                // 81
                WelshSynthPreset::by_name(WelshPresetName::MonoSolo) // TODO: same?
            }
            GeneralMidiProgram::Lead2Sawtooth => {
                // 82
                panic!();
            }
            GeneralMidiProgram::Lead3Calliope => {
                // 83
                panic!();
            }
            GeneralMidiProgram::Lead4Chiff => {
                // 84
                panic!();
            }
            GeneralMidiProgram::Lead5Charang => {
                // 85
                panic!();
            }
            GeneralMidiProgram::Lead6Voice => {
                // 86
                panic!();
            }
            GeneralMidiProgram::Lead7Fifths => {
                // 87
                panic!();
            }
            GeneralMidiProgram::Lead8BassLead => {
                // 88
                panic!();
            }
            GeneralMidiProgram::Pad1NewAge => {
                // 89
                WelshSynthPreset::by_name(WelshPresetName::NewAgeLead) // TODO pad or lead?
            }
            GeneralMidiProgram::Pad2Warm => {
                // 90
                panic!();
            }
            GeneralMidiProgram::Pad3Polysynth => {
                // 91
                panic!();
            }
            GeneralMidiProgram::Pad4Choir => {
                // 92
                panic!();
            }
            GeneralMidiProgram::Pad5Bowed => {
                // 93
                panic!();
            }
            GeneralMidiProgram::Pad6Metallic => {
                // 94
                panic!();
            }
            GeneralMidiProgram::Pad7Halo => {
                // 95
                panic!();
            }
            GeneralMidiProgram::Pad8Sweep => {
                // 96
                panic!();
            }
            GeneralMidiProgram::Fx1Rain => {
                // 97
                panic!();
            }
            GeneralMidiProgram::Fx2Soundtrack => {
                // 98
                panic!();
            }
            GeneralMidiProgram::Fx3Crystal => {
                // 99
                panic!();
            }
            GeneralMidiProgram::Fx4Atmosphere => {
                // 100
                panic!();
            }
            GeneralMidiProgram::Fx5Brightness => {
                // 101
                panic!();
            }
            GeneralMidiProgram::Fx6Goblins => {
                // 102
                panic!();
            }
            GeneralMidiProgram::Fx7Echoes => {
                // 103
                panic!();
            }
            GeneralMidiProgram::Fx8SciFi => {
                // 104
                panic!();
            }
            GeneralMidiProgram::Sitar => {
                // 105
                WelshSynthPreset::by_name(WelshPresetName::Sitar)
            }
            GeneralMidiProgram::Banjo => {
                // 106
                WelshSynthPreset::by_name(WelshPresetName::Banjo)
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
                WelshSynthPreset::by_name(WelshPresetName::Bagpipes)
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
                WelshSynthPreset::by_name(WelshPresetName::WheelsOfSteel) // TODO same?
            }
            GeneralMidiProgram::Woodblock => {
                // 116
                panic!();
            }
            GeneralMidiProgram::TaikoDrum => {
                // 117
                WelshSynthPreset::by_name(WelshPresetName::Timpani) // TODO substitute.....

            }
            GeneralMidiProgram::MelodicTom => {
                // 118
                panic!();
            }
            GeneralMidiProgram::SynthDrum => {
                // 119
                WelshSynthPreset::by_name(WelshPresetName::SnareDrum)

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
                WelshSynthPreset::by_name(WelshPresetName::OceanWavesWithFoghorn)
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
        }
    }
}
