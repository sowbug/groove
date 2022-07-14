use crate::preset::welsh::{WelshPresetName, WelshSynthPreset};

use super::synthesizers::SuperSynth;

#[derive(FromPrimitive)]
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
            GeneralMidiProgram::AcousticGrand => WelshSynthPreset::by_name(WelshPresetName::Piano),
            GeneralMidiProgram::BrightAcoustic => {
                WelshSynthPreset::by_name(WelshPresetName::Piano) // TODO dup
            }
            GeneralMidiProgram::ElectricGrand => {
                WelshSynthPreset::by_name(WelshPresetName::ElectricPiano)
            }
            GeneralMidiProgram::HonkyTonk => {
                panic!();
            }
            GeneralMidiProgram::ElectricPiano1 => {
                WelshSynthPreset::by_name(WelshPresetName::ElectricPiano) // TODO dup
            }
            GeneralMidiProgram::ElectricPiano2 => {
                WelshSynthPreset::by_name(WelshPresetName::ElectricPiano) // TODO dup
            }
            GeneralMidiProgram::Harpsichord => {
                WelshSynthPreset::by_name(WelshPresetName::Harpsichord)
            }
            GeneralMidiProgram::Clav => WelshSynthPreset::by_name(WelshPresetName::Clavichord),
            GeneralMidiProgram::Celesta => WelshSynthPreset::by_name(WelshPresetName::Celeste),
            GeneralMidiProgram::Glockenspiel => {
                WelshSynthPreset::by_name(WelshPresetName::Glockenspiel)
            }
            GeneralMidiProgram::MusicBox => {
                panic!();
            }
            GeneralMidiProgram::Vibraphone => {
                panic!();
            }
            GeneralMidiProgram::Marimba => WelshSynthPreset::by_name(WelshPresetName::Marimba),
            GeneralMidiProgram::Xylophone => WelshSynthPreset::by_name(WelshPresetName::Xylophone),
            GeneralMidiProgram::TubularBells => WelshSynthPreset::by_name(WelshPresetName::Bell),
            GeneralMidiProgram::Dulcimer => WelshSynthPreset::by_name(WelshPresetName::Dulcimer),
            GeneralMidiProgram::DrawbarOrgan => {
                WelshSynthPreset::by_name(WelshPresetName::Organ) // TODO dup
            }
            GeneralMidiProgram::PercussiveOrgan => {
                WelshSynthPreset::by_name(WelshPresetName::Organ) // TODO dup
            }
            GeneralMidiProgram::RockOrgan => {
                WelshSynthPreset::by_name(WelshPresetName::Organ) // TODO dup
            }
            GeneralMidiProgram::ChurchOrgan => {
                WelshSynthPreset::by_name(WelshPresetName::Organ) // TODO dup
            }
            GeneralMidiProgram::ReedOrgan => {
                WelshSynthPreset::by_name(WelshPresetName::Organ) // TODO dup
            }
            GeneralMidiProgram::Accordion => WelshSynthPreset::by_name(WelshPresetName::Accordion),
            GeneralMidiProgram::Harmonica => WelshSynthPreset::by_name(WelshPresetName::Harmonica),
            GeneralMidiProgram::TangoAccordion => {
                panic!();
            }
            GeneralMidiProgram::AcousticGuitarNylon => {
                WelshSynthPreset::by_name(WelshPresetName::GuitarAcoustic)
            }
            GeneralMidiProgram::AcousticGuitarSteel => {
                WelshSynthPreset::by_name(WelshPresetName::GuitarAcoustic) // TODO dup
            }
            GeneralMidiProgram::ElectricGuitarJazz => {
                WelshSynthPreset::by_name(WelshPresetName::GuitarElectric) // TODO dup
            }
            GeneralMidiProgram::ElectricGuitarClean => {
                WelshSynthPreset::by_name(WelshPresetName::GuitarElectric) // TODO dup
            }
            GeneralMidiProgram::ElectricGuitarMuted => {
                WelshSynthPreset::by_name(WelshPresetName::GuitarElectric) // TODO dup
            }
            GeneralMidiProgram::OverdrivenGuitar => {
                WelshSynthPreset::by_name(WelshPresetName::GuitarElectric) // TODO dup
            }
            GeneralMidiProgram::DistortionGuitar => {
                WelshSynthPreset::by_name(WelshPresetName::GuitarElectric) // TODO dup
            }
            GeneralMidiProgram::GuitarHarmonics => {
                WelshSynthPreset::by_name(WelshPresetName::GuitarElectric) // TODO dup
            }
            GeneralMidiProgram::AcousticBass => {
                WelshSynthPreset::by_name(WelshPresetName::DoubleBass)
            }
            GeneralMidiProgram::ElectricBassFinger => {
                WelshSynthPreset::by_name(WelshPresetName::StandupBass)
            }
            GeneralMidiProgram::ElectricBassPick => {
                WelshSynthPreset::by_name(WelshPresetName::AcidBass)
            }
            GeneralMidiProgram::FretlessBass => {
                WelshSynthPreset::by_name(WelshPresetName::DetroitBass) // TODO same?
            }
            GeneralMidiProgram::SlapBass1 => WelshSynthPreset::by_name(WelshPresetName::FunkBass),
            GeneralMidiProgram::SlapBass2 => WelshSynthPreset::by_name(WelshPresetName::FunkBass),
            GeneralMidiProgram::SynthBass1 => {
                WelshSynthPreset::by_name(WelshPresetName::DigitalBass)
            }
            GeneralMidiProgram::SynthBass2 => {
                WelshSynthPreset::by_name(WelshPresetName::DigitalBass)
            }
            GeneralMidiProgram::Violin => WelshSynthPreset::by_name(WelshPresetName::Violin),
            GeneralMidiProgram::Viola => WelshSynthPreset::by_name(WelshPresetName::Viola),
            GeneralMidiProgram::Cello => {
                WelshSynthPreset::by_name(crate::preset::welsh::WelshPresetName::Cello)
            }
            GeneralMidiProgram::Contrabass => {
                WelshSynthPreset::by_name(WelshPresetName::Contrabassoon)
            }
            GeneralMidiProgram::TremoloStrings => {
                panic!();
            }
            GeneralMidiProgram::PizzicatoStrings => {
                panic!();
            }
            GeneralMidiProgram::OrchestralHarp => WelshSynthPreset::by_name(WelshPresetName::Harp),
            GeneralMidiProgram::Timpani => WelshSynthPreset::by_name(WelshPresetName::Timpani),
            GeneralMidiProgram::StringEnsemble1 => {
                panic!();
            }
            GeneralMidiProgram::StringEnsemble2 => {
                WelshSynthPreset::by_name(WelshPresetName::StringsPwm) // TODO same?
            }
            GeneralMidiProgram::Synthstrings1 => {
                WelshSynthPreset::by_name(WelshPresetName::StringsPwm)
            } // TODO same?

            GeneralMidiProgram::Synthstrings2 => {
                panic!();
            }
            GeneralMidiProgram::ChoirAahs => WelshSynthPreset::by_name(WelshPresetName::Angels),

            GeneralMidiProgram::VoiceOohs => WelshSynthPreset::by_name(WelshPresetName::Choir),
            GeneralMidiProgram::SynthVoice => {
                WelshSynthPreset::by_name(WelshPresetName::VocalFemale)
            }

            GeneralMidiProgram::OrchestraHit => {
                panic!();
            }
            GeneralMidiProgram::Trumpet => WelshSynthPreset::by_name(WelshPresetName::Trumpet),
            GeneralMidiProgram::Trombone => WelshSynthPreset::by_name(WelshPresetName::Trombone),
            GeneralMidiProgram::Tuba => WelshSynthPreset::by_name(WelshPresetName::Tuba),
            GeneralMidiProgram::MutedTrumpet => {
                panic!();
            }
            GeneralMidiProgram::FrenchHorn => {
                WelshSynthPreset::by_name(WelshPresetName::FrenchHorn)
            }

            GeneralMidiProgram::BrassSection => {
                WelshSynthPreset::by_name(WelshPresetName::BrassSection)
            }

            GeneralMidiProgram::Synthbrass1 => {
                WelshSynthPreset::by_name(WelshPresetName::BrassSection) // TODO dup
            }
            GeneralMidiProgram::Synthbrass2 => {
                WelshSynthPreset::by_name(WelshPresetName::BrassSection) // TODO dup
            }
            GeneralMidiProgram::SopranoSax => {
                WelshSynthPreset::by_name(WelshPresetName::Saxophone) // TODO dup
            }
            GeneralMidiProgram::AltoSax => WelshSynthPreset::by_name(WelshPresetName::Saxophone),
            GeneralMidiProgram::TenorSax => {
                WelshSynthPreset::by_name(WelshPresetName::Saxophone) // TODO dup
            }
            GeneralMidiProgram::BaritoneSax => {
                WelshSynthPreset::by_name(WelshPresetName::Saxophone) // TODO dup
            }
            GeneralMidiProgram::Oboe => WelshSynthPreset::by_name(WelshPresetName::Oboe),
            GeneralMidiProgram::EnglishHorn => {
                WelshSynthPreset::by_name(WelshPresetName::EnglishHorn)
            }
            GeneralMidiProgram::Bassoon => WelshSynthPreset::by_name(WelshPresetName::Bassoon),
            GeneralMidiProgram::Clarinet => WelshSynthPreset::by_name(WelshPresetName::Clarinet),
            GeneralMidiProgram::Piccolo => WelshSynthPreset::by_name(WelshPresetName::Piccolo),
            GeneralMidiProgram::Flute => WelshSynthPreset::by_name(WelshPresetName::Flute),
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
                WelshSynthPreset::by_name(WelshPresetName::MonoSolo) // TODO: same?
            }
            GeneralMidiProgram::Lead2Sawtooth => {
                WelshSynthPreset::by_name(WelshPresetName::Trance5th) // TODO: same?
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
                WelshSynthPreset::by_name(WelshPresetName::NewAgeLead) // TODO pad or lead?
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
            GeneralMidiProgram::Sitar => WelshSynthPreset::by_name(WelshPresetName::Sitar),
            GeneralMidiProgram::Banjo => WelshSynthPreset::by_name(WelshPresetName::Banjo),
            GeneralMidiProgram::Shamisen => {
                panic!();
            }
            GeneralMidiProgram::Koto => {
                panic!();
            }
            GeneralMidiProgram::Kalimba => {
                panic!();
            }
            GeneralMidiProgram::Bagpipe => WelshSynthPreset::by_name(WelshPresetName::Bagpipes),
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
                WelshSynthPreset::by_name(WelshPresetName::WheelsOfSteel) // TODO same?
            }
            GeneralMidiProgram::Woodblock => WelshSynthPreset::by_name(WelshPresetName::SideStick),
            GeneralMidiProgram::TaikoDrum => {
                // XXXXXXXXXXXXX TMP
                WelshSynthPreset::by_name(WelshPresetName::Cello) // TODO substitute.....
            }
            GeneralMidiProgram::MelodicTom => WelshSynthPreset::by_name(WelshPresetName::Bongos),
            GeneralMidiProgram::SynthDrum => WelshSynthPreset::by_name(WelshPresetName::SnareDrum),
            GeneralMidiProgram::ReverseCymbal => WelshSynthPreset::by_name(WelshPresetName::Cymbal),
            GeneralMidiProgram::GuitarFretNoise => {
                panic!();
            }
            GeneralMidiProgram::BreathNoise => {
                panic!();
            }
            GeneralMidiProgram::Seashore => {
                WelshSynthPreset::by_name(WelshPresetName::OceanWavesWithFoghorn)
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
