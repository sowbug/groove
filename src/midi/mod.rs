pub(crate) mod patterns;
pub(crate) mod programmers;
pub(crate) mod sequencers;
pub(crate) mod smf_reader;

pub use midly::MidiMessage;

use crate::{
    common::{rrc, rrc_clone, rrc_downgrade, weak_new, Rrc, Ww},
    traits::{SinksMidi, SourcesMidi},
    OldOrchestrator,
};
use crossbeam::deque::{Stealer, Worker};
use midir::{Ignore, MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection, SendError};
use midly::{live::LiveEvent, num::u4};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Default)]
#[allow(dead_code)]
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
    Controller,
}

pub struct MidiUtils {}

impl MidiUtils {
    pub fn note_to_frequency(note: u8) -> f32 {
        2.0_f32.powf((note as f32 - 69.0) / 12.0) * 440.0
    }

    #[allow(dead_code)]
    pub fn note_type_to_frequency(midi_note: MidiNote) -> f32 {
        2.0_f32.powf((midi_note as u8 as f32 - 69.0) / 12.0) * 440.0
    }

    #[allow(unused_variables)]
    pub fn message_to_frequency(message: &MidiMessage) -> f32 {
        match message {
            MidiMessage::NoteOff { key, vel } => Self::note_to_frequency(u8::from(*key)),
            MidiMessage::NoteOn { key, vel } => Self::note_to_frequency(u8::from(*key)),
            MidiMessage::Aftertouch { key, vel } => todo!(),
            MidiMessage::Controller { controller, value } => todo!(),
            MidiMessage::ProgramChange { program } => todo!(),
            MidiMessage::ChannelAftertouch { vel } => todo!(),
            MidiMessage::PitchBend { bend } => todo!(),
        }
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

    fn handle_midi_for_channel(
        &mut self,
        clock: &crate::clock::Clock,
        channel: &MidiChannel,
        message: &MidiMessage,
    ) {
        // send to everyone EXCEPT whoever sent it!
        self.issue_midi(clock, channel, message);
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

/// Handles MIDI input coming from outside Groove. For example, if you have a
/// MIDI keyboard plugged into your computer's USB, you should be able to use
/// that keyboard to input notes into Groove, and MidiInputHandler manages that.
#[derive(Default)]
pub struct MidiInputHandler {
    conn_in: Option<MidiInputConnection<()>>,
    pub stealer: Option<Stealer<(u64, u8, MidiMessage)>>,
    inputs: Vec<(usize, String)>,
}

impl MidiInputHandler {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        if let Ok(mut midi_in) = MidiInput::new("Groove MIDI input") {
            midi_in.ignore(Ignore::None);

            let in_ports = midi_in.ports();
            let in_port = match in_ports.len() {
                0 => return Err(anyhow::Error::msg("no input port found")),
                1 => {
                    println!(
                        "Choosing the only available input port: {}",
                        midi_in.port_name(&in_ports[0]).unwrap()
                    );
                    &in_ports[0]
                }
                _ => {
                    for port in in_ports.iter() {
                        println!("{}", midi_in.port_name(port).unwrap());
                    }
                    //                panic!("there are multiple MIDI input
                    //                devices, and that's scary");
                    &in_ports[1]
                }
            };

            self.inputs.clear();
            for (i, port) in in_ports.iter().enumerate() {
                self.inputs.push((i, midi_in.port_name(port).unwrap()));
            }

            let _in_port_name = midi_in.port_name(in_port)?;

            let worker = Worker::<(u64, u8, MidiMessage)>::new_fifo();
            self.stealer = Some(worker.stealer());
            match midi_in.connect(
                in_port,
                "Groove input",
                move |stamp, event, _| {
                    let event = LiveEvent::parse(event).unwrap();
                    #[allow(clippy::single_match)]
                    match event {
                        LiveEvent::Midi { channel, message } => {
                            worker.push((stamp, u8::from(channel), message));
                        }
                        _ => {}
                    }
                },
                (),
            ) {
                Ok(conn) => {
                    self.conn_in = Some(conn);
                    Ok(())
                }
                Err(err) => Err(anyhow::Error::msg(err.to_string())),
            }
        } else {
            Err(anyhow::Error::msg("couldn't create MidiInput"))
        }
    }

    pub fn stop(&mut self) {
        self.conn_in = None;
    }

    // TODO: no idea when this gets refreshed
    pub fn inputs(&self) -> &[(usize, String)] {
        &self.inputs
    }
}

pub struct MidiOutputHandler {
    pub(crate) me: Ww<Self>,
    conn_out: Option<MidiOutputConnection>,
    stealer: Option<Stealer<(u64, u4, MidiMessage)>>,
    outputs: Vec<(usize, String)>,
}

impl std::fmt::Debug for MidiOutputHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({:?}, {:?})", self.stealer, self.outputs)
    }
}

impl Default for MidiOutputHandler {
    fn default() -> Self {
        Self {
            me: weak_new(),
            conn_out: None,
            stealer: None,
            outputs: Vec::default(),
        }
    }
}

impl MidiOutputHandler {
    fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn new_wrapped() -> Rrc<Self> {
        let wrapped = rrc(Self::new());
        wrapped.borrow_mut().me = rrc_downgrade(&wrapped);
        wrapped
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        if let Ok(midi_out) = MidiOutput::new("Groove MIDI output") {
            let out_ports = midi_out.ports();
            let out_port = match out_ports.len() {
                0 => return Err(anyhow::Error::msg("no output port found")),
                1 => {
                    println!(
                        "Choosing the only available output port: {}",
                        midi_out.port_name(&out_ports[0]).unwrap()
                    );
                    &out_ports[0]
                }
                _ => {
                    for port in out_ports.iter() {
                        println!("{}", midi_out.port_name(port).unwrap());
                    }
                    //                panic!("there are multiple MIDI output
                    //                devices, and that's scary");
                    &out_ports[1] // TODO
                }
            };

            // self.outputs.clear(); for (i, port) in
            // out_ports.iter().enumerate() { self.outputs .push((i,
            //     midi_out.port_name(port).unwrap())); }

            // let out_port_name = midi_out.port_name(out_port)?;

            let worker = Worker::<(u64, u4, MidiMessage)>::new_fifo();
            self.stealer = Some(worker.stealer());
            match midi_out.connect(out_port, "Groove output") {
                Ok(conn) => {
                    self.conn_out = Some(conn);
                    Ok(())
                }
                Err(err) => Err(anyhow::Error::msg(err.to_string())),
            }
        } else {
            Err(anyhow::Error::msg("couldn't create MidiOutput"))
        }
    }

    pub fn send(&mut self, message: &[u8]) -> Result<(), SendError> {
        if self.conn_out.is_some() {
            self.conn_out.as_mut().unwrap().send(message)
        } else {
            Err(SendError::Other("couldn't send"))
        }
    }

    pub fn stop(&mut self) {
        self.conn_out = None;
    }

    // TODO: no idea when this gets refreshed
    #[allow(dead_code)]
    pub fn outputs(&self) -> &[(usize, String)] {
        &self.outputs
    }
}

impl SinksMidi for MidiOutputHandler {
    fn midi_channel(&self) -> MidiChannel {
        MIDI_CHANNEL_RECEIVE_ALL
    }

    fn set_midi_channel(&mut self, _midi_channel: MidiChannel) {}

    fn handle_midi_for_channel(
        &mut self,
        _clock: &crate::clock::Clock,
        channel: &MidiChannel,
        message: &MidiMessage,
    ) {
        let event = LiveEvent::Midi {
            channel: u4::from(*channel),
            message: *message,
        };

        // TODO: this seems like a lot of work
        let mut buf = Vec::new();
        event.write(&mut buf).unwrap();
        if self.send(&buf).is_err() {
            // TODO
        }
    }
}

pub struct MidiHandler {
    midi_input: MidiInputHandler,
    midi_output: Rrc<MidiOutputHandler>,
}

// TODO - this is an ExternalMidi that implements SinksMidi and SourcesMidi, and
// IOHelper connects it to Orchestrator. I think.
impl MidiHandler {
    pub fn new_with(orchestrator: &mut OldOrchestrator) -> Self {
        let r = Self::default();

        let t = rrc_clone(&r.midi_output);
        let t: Rrc<dyn SinksMidi> = t;
        orchestrator.connect_to_downstream_midi_bus(MIDI_CHANNEL_RECEIVE_ALL, rrc_downgrade(&t));

        r
    }

    pub fn available_devices(&self) -> &[(usize, String)] {
        self.midi_input.inputs()
    }

    pub fn input_stealer(&self) -> &Option<Stealer<(u64, u8, MidiMessage)>> {
        &self.midi_input.stealer
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        self.midi_input.start()?;
        self.midi_output.borrow_mut().start()?;
        Ok(())
    }

    pub fn stop(&mut self) {
        self.midi_input.stop();
        self.midi_output.borrow_mut().stop();
    }
}

impl Default for MidiHandler {
    fn default() -> Self {
        Self {
            midi_input: MidiInputHandler::new(),
            midi_output: MidiOutputHandler::new_wrapped(),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::common::{StereoSample, MONO_SAMPLE_MAX, MONO_SAMPLE_MIN, MONO_SAMPLE_SILENCE};
    use assert_approx_eq::assert_approx_eq;
    use midly::num::u7;

    #[allow(dead_code)]
    pub const STEREO_SAMPLE_SILENCE: StereoSample = (MONO_SAMPLE_SILENCE, MONO_SAMPLE_SILENCE);
    #[allow(dead_code)]
    pub const STEREO_SAMPLE_MAX: StereoSample = (MONO_SAMPLE_MAX, MONO_SAMPLE_MAX);
    #[allow(dead_code)]
    pub const STEREO_SAMPLE_MIN: StereoSample = (MONO_SAMPLE_MAX, MONO_SAMPLE_MIN);

    impl MidiUtils {
        pub(crate) fn new_note_on(note: u8, vel: u8) -> MidiMessage {
            MidiMessage::NoteOn {
                key: u7::from(note),
                vel: u7::from(vel),
            }
        }

        pub(crate) fn new_note_off(note: u8, vel: u8) -> MidiMessage {
            MidiMessage::NoteOff {
                key: u7::from(note),
                vel: u7::from(vel),
            }
        }

        pub fn note_on_c4() -> MidiMessage {
            Self::new_note_on(MidiNote::C4 as u8, 0)
        }

        pub fn note_off_c4() -> MidiMessage {
            Self::new_note_off(MidiNote::C4 as u8, 0)
        }
    }

    #[test]
    fn test_note_to_frequency() {
        let message = MidiUtils::new_note_on(MidiNote::C4 as u8, 0);
        assert_approx_eq!(MidiUtils::message_to_frequency(&message), 261.625_55);
        let message = MidiUtils::new_note_on(0, 0);
        assert_approx_eq!(MidiUtils::message_to_frequency(&message), 8.175798);
        let message = MidiUtils::new_note_on(MidiNote::G9 as u8, 0);
        assert_approx_eq!(MidiUtils::message_to_frequency(&message), 12543.855);
    }
}
