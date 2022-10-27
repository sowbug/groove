pub(crate) mod sequencer;
pub(crate) mod smf_reader;

use crate::{
    common::Ww,
    traits::{SinksMidi, SourcesMidi},
};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::HashMap};

#[derive(Clone, Copy, Debug, Default)]
pub enum MidiNote {
    None = 0,
    A0 = 21,
    D3 = 50,
    #[default]
    C4 = 60,
    G4 = 67,
    A4 = 69,
    G9 = 127,
}

pub type MidiChannel = u8;
pub const MIDI_CHANNEL_RECEIVE_NONE: MidiChannel = 254;
pub const MIDI_CHANNEL_RECEIVE_ALL: MidiChannel = 255;

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone, Serialize, Deserialize, Copy, Default)]
pub enum MidiMessageType {
    #[default] // there isn't any sensible default here, so we pick something loud
    NoteOn = 0b1001,
    NoteOff = 0b1000,
    ProgramChange = 0b1100,
}
#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Clone, Serialize, Deserialize, Copy, Default)]
pub struct MidiMessage {
    // status and channel are normally packed into one byte, but for ease of use
    // we're unpacking here.
    pub status: MidiMessageType,
    pub channel: MidiChannel,
    pub data1: u8,
    pub data2: u8,
}

impl MidiMessage {
    pub fn note_to_frequency(note: u8) -> f32 {
        2.0_f32.powf((note as f32 - 69.0) / 12.0) * 440.0
    }

    #[allow(dead_code)]
    pub fn note_type_to_frequency(midi_note: MidiNote) -> f32 {
        2.0_f32.powf((midi_note as u8 as f32 - 69.0) / 12.0) * 440.0
    }

    pub fn message_to_frequency(&self) -> f32 {
        Self::note_to_frequency(self.data1)
    }

    pub(crate) fn new_note_on(channel: MidiChannel, note: u8, vel: u8) -> Self {
        MidiMessage {
            status: MidiMessageType::NoteOn,
            channel,
            data1: note,
            data2: vel,
        }
    }

    pub(crate) fn new_note_off(channel: MidiChannel, note: u8, vel: u8) -> Self {
        MidiMessage {
            status: MidiMessageType::NoteOff,
            channel,
            data1: note,
            data2: vel,
        }
    }

    pub(crate) fn new_program_change(channel: MidiChannel, program: u8) -> Self {
        MidiMessage {
            status: MidiMessageType::ProgramChange,
            channel,
            data1: program,
            data2: 0,
        }
    }
}

#[derive(Eq, Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrderedMidiMessage {
    pub when: u32,
    pub message: MidiMessage,
}

impl Ord for OrderedMidiMessage {
    fn cmp(&self, other: &Self) -> Ordering {
        self.when.cmp(&other.when)
    }
}

impl PartialOrd for OrderedMidiMessage {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for OrderedMidiMessage {
    fn eq(&self, other: &Self) -> bool {
        self.when == other.when
    }
}

#[derive(Debug, Default)]
pub(crate) struct MidiBus {
    channels_to_sink_vecs: HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>>,
}
impl SinksMidi for MidiBus {
    fn midi_channel(&self) -> MidiChannel {
        MIDI_CHANNEL_RECEIVE_ALL
    }

    fn set_midi_channel(&mut self, _midi_channel: MidiChannel) {}

    fn handle_midi_for_channel(&mut self, clock: &crate::clock::Clock, message: &MidiMessage) {
        // send to everyone EXCEPT whoever sent it!
        self.issue_midi(clock, message);
    }
}
impl SourcesMidi for MidiBus {
    fn midi_sinks(&self) -> &HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>> {
        &self.channels_to_sink_vecs
    }

    fn midi_sinks_mut(&mut self) -> &mut HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>> {
        &mut self.channels_to_sink_vecs
    }

    fn midi_output_channel(&self) -> MidiChannel {
        MIDI_CHANNEL_RECEIVE_ALL
    }

    fn set_midi_output_channel(&mut self, _midi_channel: MidiChannel) {}
}
impl MidiBus {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

use enum_primitive_derive::Primitive;
use strum_macros::Display;

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

#[allow(dead_code)]
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
pub mod tests {
    use super::*;
    use crate::common::{StereoSample, MONO_SAMPLE_MAX, MONO_SAMPLE_MIN, MONO_SAMPLE_SILENCE};
    use assert_approx_eq::assert_approx_eq;

    #[allow(dead_code)]
    pub const STEREO_SAMPLE_SILENCE: StereoSample = (MONO_SAMPLE_SILENCE, MONO_SAMPLE_SILENCE);
    #[allow(dead_code)]
    pub const STEREO_SAMPLE_MAX: StereoSample = (MONO_SAMPLE_MAX, MONO_SAMPLE_MAX);
    #[allow(dead_code)]
    pub const STEREO_SAMPLE_MIN: StereoSample = (MONO_SAMPLE_MAX, MONO_SAMPLE_MIN);

    impl MidiMessage {
        pub fn note_on_c4() -> MidiMessage {
            MidiMessage::new_note_on(0, MidiNote::C4 as u8, 0)
        }

        pub fn note_off_c4() -> MidiMessage {
            MidiMessage::new_note_off(0, MidiNote::C4 as u8, 0)
        }
    }

    #[test]
    fn test_note_to_frequency() {
        assert_approx_eq!(
            MidiMessage::new_note_on(0, MidiNote::C4 as u8, 0).message_to_frequency(),
            261.625549
        );
        assert_approx_eq!(
            MidiMessage::new_note_on(0, 0, 0).message_to_frequency(),
            8.175798
        );
        assert_approx_eq!(
            MidiMessage::new_note_on(0, MidiNote::G9 as u8, 0).message_to_frequency(),
            12543.855
        );
    }
}
