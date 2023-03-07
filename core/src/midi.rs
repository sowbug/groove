// Copyright (c) 2023 Mike Tsao. All rights reserved.

use enum_primitive_derive::Primitive;
pub use midly::live::LiveEvent;
use std::fmt::Debug;
use strum_macros::Display;

use crate::ParameterType;
pub use midly::{
    num::{u4, u7},
    MidiMessage,
};

pub type MidiChannel = u8;

/// Takes standard MIDI messages. Implementers can ignore MidiChannel if it's
/// not important, as the virtual cabling model tries to route only relevant
/// traffic to individual devices.
pub trait HandlesMidi {
    #[allow(unused_variables)]
    fn handle_midi_message(
        &mut self,
        message: &MidiMessage,
    ) -> Option<Vec<(MidiChannel, MidiMessage)>> {
        None
    }
}

/// There are two different mappings of piano notes to MIDI numbers. They both
/// agree that Midi note 0 is a C, but they otherwise differ by an octave. I
/// originally picked C4=60, because that was the top Google search result's
/// answer, but it seems like a slight majority thinks C3=60. I'm going to leave
/// it as-is so that I don't have to rename my test data files. I don't think it
/// matters because we're not actually mapping these to anything user-visible.
///
/// These also correspond to <https://en.wikipedia.org/wiki/Piano_key_frequencies>
#[derive(Clone, Copy, Debug, Default)]
pub enum MidiNote {
    None = 0,
    C0 = 12,
    Cs0 = 13,
    D0 = 14,
    Ds0 = 15,
    E0 = 16,
    F0 = 17,
    Fs0 = 18,
    G0 = 19,
    Gs0 = 20,
    A0 = 21,
    As0 = 22,
    B0 = 23,
    C1 = 24,
    C2 = 36,
    C3 = 48,
    D3 = 50,
    #[default]
    C4 = 60,
    G4 = 67,
    A4 = 69,
    C5 = 72,
    D5 = 74,
    D6 = 86,
    G9 = 127,
}

pub fn note_to_frequency(note: u8) -> ParameterType {
    2.0_f64.powf((note as ParameterType - 69.0) / 12.0) * 440.0
}

pub fn note_type_to_frequency(midi_note: MidiNote) -> ParameterType {
    2.0_f64.powf((midi_note as u8 as ParameterType - 69.0) / 12.0) * 440.0
}

pub fn note_description_to_frequency(text: String, default: ParameterType) -> ParameterType {
    if !text.is_empty() {
        if text.contains('.') {
            let frequency = text.parse::<ParameterType>().unwrap_or(default);
            if frequency > 0.0 {
                return frequency;
            }
        } else if let Ok(note) = text.parse::<u8>() {
            return note_to_frequency(note);
        }
    }
    default
}

pub fn new_note_on(note: u8, vel: u8) -> MidiMessage {
    MidiMessage::NoteOn {
        key: u7::from(note),
        vel: u7::from(vel),
    }
}

pub fn new_note_off(note: u8, vel: u8) -> MidiMessage {
    MidiMessage::NoteOff {
        key: u7::from(note),
        vel: u7::from(vel),
    }
}

#[derive(Display, Primitive, Debug)]
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

#[derive(Clone, Copy)]
pub enum GeneralMidiPercussionProgram {
    AcousticBassDrum = 35,
    ElectricBassDrum = 36,
    SideStick = 37,
    AcousticSnare = 38,
    HandClap = 39,
    ElectricSnare = 40,
    LowFloorTom = 41,
    ClosedHiHat = 42,
    HighFloorTom = 43,
    PedalHiHat = 44,
    LowTom = 45,
    OpenHiHat = 46,
    LowMidTom = 47,
    HiMidTom = 48,
    CrashCymbal1 = 49,
    HighTom = 50,
    RideCymbal1 = 51,
    ChineseCymbal = 52,
    RideBell = 53,
    Tambourine = 54,
    SplashCymbal = 55,
    Cowbell = 56,
    CrashCymbal2 = 57,
    Vibraslap = 58,
    RideCymbal2 = 59,
    HighBongo = 60,
    LowBongo = 61,
    MuteHighConga = 62,
    OpenHighConga = 63,
    LowConga = 64,
    HighTimbale = 65,
    LowTimbale = 66,
    HighAgogo = 67,
    LowAgogo = 68,
    Cabasa = 69,
    Maracas = 70,
    ShortWhistle = 71,
    LongWhistle = 72,
    ShortGuiro = 73,
    LongGuiro = 74,
    Claves = 75,
    HighWoodblock = 76,
    LowWoodblock = 77,
    MuteCuica = 78,
    OpenCuica = 79,
    MuteTriangle = 80,
    OpenTriangle = 81,
}

#[cfg(test)]
mod tests {
    use super::note_description_to_frequency;
    use crate::midi::{note_type_to_frequency, MidiNote};

    #[test]
    fn note_to_frequency() {
        // https://www.colincrawley.com/midi-note-to-audio-frequency-calculator/
        assert_eq!(note_type_to_frequency(MidiNote::C0), 16.351_597_831_287_414);
        assert_eq!(note_type_to_frequency(MidiNote::C4), 261.625_565_300_598_6);
        assert_eq!(note_type_to_frequency(MidiNote::D5), 587.329_535_834_815_1);
        assert_eq!(
            note_type_to_frequency(MidiNote::D6),
            1_174.659_071_669_630_3
        );
        assert_eq!(note_type_to_frequency(MidiNote::G9), 12_543.853_951_415_975);
    }

    #[test]
    fn text_to_frequency() {
        assert_eq!(
            note_description_to_frequency("440.0".to_string(), 999.9),
            440.0,
            "A floating-point number should parse as a frequency"
        );
        assert_eq!(
            note_description_to_frequency("69".to_string(), 999.9),
            440.0,
            "An integer should parse as a MIDI note with 69 = 440.0Hz"
        );
        assert_eq!(
            note_description_to_frequency("0".to_string(), 999.9),
            8.175_798_915_643_707,
            "MIDI note zero is valid!"
        );
        assert_eq!(
            note_description_to_frequency("-4".to_string(), 999.9),
            999.9,
            "Negative note numbers are invalid"
        );
        assert_eq!(
            note_description_to_frequency("0.0".to_string(), 999.9),
            999.9,
            "Frequency zero is not valid (design decision)"
        );
        assert_eq!(
            note_description_to_frequency("-440.0".to_string(), 999.9),
            999.9,
            "Negative frequencies are invalid"
        );
        assert_eq!(
            note_description_to_frequency("1.2.3.4".to_string(), 999.9),
            999.9,
            "Gobbledygook should parse as default"
        );
        assert_eq!(
            note_description_to_frequency("chartreuse".to_string(), 999.9),
            999.9,
            "Gobbledygook should parse as default"
        );
        assert_eq!(
            note_description_to_frequency("".to_string(), 999.9),
            999.9,
            "Empty string should parse as default"
        );
    }
}
