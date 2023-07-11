// Copyright (c) 2023 Mike Tsao. All rights reserved.

pub use midly::live::LiveEvent;
pub use midly::{
    num::{u4, u7},
    MidiMessage,
};

use crate::{FrequencyHz, ParameterType};
use bit_vec::BitVec;
use derive_more::Display as DeriveDisplay;
use enum_primitive_derive::Primitive;
use std::fmt::Debug;
use strum_macros::Display;

#[derive(Clone, Copy, Debug, Default, DeriveDisplay, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct MidiChannel(pub u8);
impl MidiChannel {
    pub const MAX: u8 = 16;

    pub const fn value(&self) -> u8 {
        self.0
    }

    pub const fn new(value: u8) -> Self {
        Self { 0: value }
    }
}
impl From<u4> for MidiChannel {
    fn from(value: u4) -> Self {
        Self(value.as_int())
    }
}
impl From<u8> for MidiChannel {
    fn from(value: u8) -> Self {
        Self(value)
    }
}
impl From<MidiChannel> for u8 {
    fn from(value: MidiChannel) -> Self {
        value.0
    }
}

pub type MidiMessagesFn<'a> = dyn FnMut(MidiChannel, MidiMessage) + 'a;

/// Takes standard MIDI messages. Implementers can ignore MidiChannel if it's
/// not important, as the virtual cabling model tries to route only relevant
/// traffic to individual devices.
pub trait HandlesMidi {
    #[allow(unused_variables)]
    fn handle_midi_message(
        &mut self,
        channel: MidiChannel,
        message: MidiMessage,
        midi_messages_fn: &mut MidiMessagesFn,
    ) {
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
    D4 = 62,
    E4 = 64,
    F4 = 65,
    G4 = 67,
    A4 = 69,
    B4 = 71,
    C5 = 72,
    D5 = 74,
    E5 = 76,
    D6 = 86,
    G9 = 127,
}

pub fn note_to_frequency(note: u8) -> FrequencyHz {
    (2.0_f64.powf((note as ParameterType - 69.0) / 12.0) * 440.0).into()
}

pub fn note_type_to_frequency(midi_note: MidiNote) -> FrequencyHz {
    FrequencyHz::from(2.0_f64.powf((midi_note as u8 as ParameterType - 69.0) / 12.0) * 440.0)
}

pub fn note_description_to_frequency(text: &str) -> Option<FrequencyHz> {
    if text.is_empty() {
        return None;
    }
    if text.contains('.') {
        if let Ok(parsed_float) = text.parse::<ParameterType>() {
            if parsed_float > 0.0 {
                return Some(FrequencyHz::from(parsed_float));
            }
        }
    } else if let Ok(note) = text.parse::<u8>() {
        return Some(note_to_frequency(note));
    }
    None
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

/// [MidiNoteMinder] watches a MIDI message stream and remembers which notes are
/// currently active (we've gotten a note-on without a note-off). Then, when
/// asked, it produces a list of MIDI message that turn off all active notes.
///
/// [MidiNoteMinder] doesn't know about [MidiChannel]s. It's up to the caller to
/// track channels, or else assume that if we got any message, it's for us, and
/// that the same is true for recipients of whatever we send.
#[derive(Debug)]
pub struct MidiNoteMinder {
    active_notes: BitVec,
}
impl Default for MidiNoteMinder {
    fn default() -> Self {
        Self {
            active_notes: BitVec::from_elem(128, false),
        }
    }
}
impl MidiNoteMinder {
    pub fn watch_message(&mut self, message: &MidiMessage) {
        #[allow(unused_variables)]
        match message {
            MidiMessage::NoteOff { key, vel } => {
                self.active_notes.set(key.as_int() as usize, false);
            }
            MidiMessage::NoteOn { key, vel } => {
                self.active_notes
                    .set(key.as_int() as usize, *vel != u7::from(0));
            }
            _ => {}
        }
    }

    pub fn generate_off_messages(&self) -> Vec<MidiMessage> {
        let mut v = Vec::default();
        for (i, active_note) in self.active_notes.iter().enumerate() {
            if active_note {
                v.push(MidiMessage::NoteOff {
                    key: u7::from_int_lossy(i as u8),
                    vel: u7::from(0),
                })
            }
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use midly::{num::u7, MidiMessage};

    use super::{note_description_to_frequency, MidiNoteMinder};
    use crate::midi::{new_note_off, new_note_on, note_type_to_frequency, MidiNote};

    #[test]
    fn note_to_frequency() {
        // https://www.colincrawley.com/midi-note-to-audio-frequency-calculator/
        assert_eq!(
            note_type_to_frequency(MidiNote::C0),
            16.351_597_831_287_414.into()
        );
        assert_eq!(
            note_type_to_frequency(MidiNote::C4),
            261.625_565_300_598_6.into()
        );
        assert_eq!(
            note_type_to_frequency(MidiNote::D5),
            587.329_535_834_815_1.into()
        );
        assert_eq!(
            note_type_to_frequency(MidiNote::D6),
            1_174.659_071_669_630_3.into()
        );
        assert_eq!(
            note_type_to_frequency(MidiNote::G9),
            12_543.853_951_415_975.into()
        );
    }

    #[test]
    fn text_to_frequency() {
        assert_eq!(
            note_description_to_frequency("440.0").unwrap().value(),
            440.0,
            "A floating-point number should parse as a frequency"
        );
        assert_eq!(
            note_description_to_frequency("69").unwrap().value(),
            440.0,
            "An integer should parse as a MIDI note with 69 = 440.0Hz"
        );
        assert_eq!(
            note_description_to_frequency("0").unwrap().value(),
            8.175_798_915_643_707,
            "MIDI note zero is valid!"
        );
        assert_eq!(
            note_description_to_frequency("-4"),
            None,
            "Negative note numbers are invalid"
        );
        assert_eq!(
            note_description_to_frequency("0.0"),
            None,
            "Frequency zero is not valid (design decision)"
        );
        assert_eq!(
            note_description_to_frequency("-440.0"),
            None,
            "Negative frequencies are invalid"
        );
        assert_eq!(
            note_description_to_frequency("1.2.3.4"),
            None,
            "Gobbledygook should fail to parse"
        );
        assert_eq!(
            note_description_to_frequency("chartreuse"),
            None,
            "Gobbledygook should fail to parse"
        );
        assert_eq!(
            note_description_to_frequency(""),
            None,
            "Empty string should fail to parse"
        );
    }

    #[test]
    fn midi_note_minder() {
        let mut mnm = MidiNoteMinder::default();

        assert!(mnm.generate_off_messages().is_empty());

        // Unexpected note-off doesn't explode
        mnm.watch_message(&new_note_off(42, 111));
        assert!(mnm.generate_off_messages().is_empty());

        // normal
        mnm.watch_message(&new_note_on(42, 99));
        let msgs = mnm.generate_off_messages();
        assert_eq!(msgs.len(), 1);
        assert_eq!(
            msgs[0],
            MidiMessage::NoteOff {
                key: u7::from(42),
                vel: u7::from(0)
            }
        );

        // duplicate on doesn't explode or add twice
        mnm.watch_message(&new_note_on(42, 88));
        let msgs = mnm.generate_off_messages();
        assert_eq!(msgs.len(), 1);
        assert_eq!(
            msgs[0],
            MidiMessage::NoteOff {
                key: u7::from(42),
                vel: u7::from(0)
            }
        );

        // normal
        mnm.watch_message(&new_note_off(42, 77));
        assert!(mnm.generate_off_messages().is_empty());

        // duplicate off doesn't explode
        mnm.watch_message(&new_note_off(42, 66));
        assert!(mnm.generate_off_messages().is_empty());

        // velocity zero treated same as note-off
        mnm.watch_message(&new_note_on(42, 99));
        assert_eq!(mnm.generate_off_messages().len(), 1);
        mnm.watch_message(&new_note_off(42, 99));
        assert_eq!(mnm.generate_off_messages().len(), 0);
        mnm.watch_message(&new_note_on(42, 99));
        assert_eq!(mnm.generate_off_messages().len(), 1);
        mnm.watch_message(&new_note_on(42, 0));
        assert_eq!(mnm.generate_off_messages().len(), 0);
    }
}
