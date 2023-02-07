pub(crate) mod patterns;
pub(crate) mod programmers;
pub(crate) mod smf_reader;
pub(crate) mod subscription;

// TODO copy and conform MidiMessage to MessageBounds so it can be a trait
// associated type
use self::subscription::MidiHandlerEvent;
use crate::{
    messages::MessageBounds,
    traits::{HasUid, Response, Terminates},
    Clock,
};
use crossbeam::deque::{Steal, Stealer, Worker};
use enum_primitive_derive::Primitive;
use groove_macros::Uid;
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection, SendError};
pub use midly::MidiMessage;
use midly::{live::LiveEvent, num::u4};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, time::Instant};
use strum_macros::Display;

/// There are two different mappings of piano notes to MIDI numbers. They both
/// agree that Midi note 0 is a C, but they otherwise differ by an octave. I
/// originally picked C4=60, because that was the top Google search result's
/// answer, but it seems like a slight majority thinks C3=60. I'm going to leave
/// it as-is so that I don't have to rename my test data files. I don't think it
/// matters because we're not actually mapping these to anything user-visible.
///
/// These also correspond to https://en.wikipedia.org/wiki/Piano_key_frequencies
#[derive(Clone, Copy, Debug, Default)]
#[allow(dead_code)]
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
    G9 = 127,
}

pub type MidiChannel = u8;

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

pub type MidiInputStealer = Stealer<(u64, u8, MidiMessage)>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MidiPortLabel {
    index: usize,
    name: String,
}

impl std::fmt::Display for MidiPortLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

/// Handles MIDI input coming from outside Groove. For example, if you have a
/// MIDI keyboard plugged into your computer's USB, you should be able to use
/// that keyboard to input notes into Groove, and MidiInputHandler manages that.
pub struct MidiInputHandler {
    midi: Option<MidiInput>,
    active_port: Option<MidiPortLabel>,
    labels: Vec<MidiPortLabel>,
    connection: Option<MidiInputConnection<()>>,
    stealer: Option<MidiInputStealer>,
}
impl MidiInputHandler {
    pub fn new() -> anyhow::Result<Self> {
        if let Ok(midi_input) = MidiInput::new("Groove MIDI input") {
            Ok(Self {
                midi: Some(midi_input),
                active_port: Default::default(),
                labels: Default::default(),
                connection: Default::default(),
                stealer: Default::default(),
            })
        } else {
            Err(anyhow::Error::msg("Couldn't create MIDI input"))
        }
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        self.refresh_ports();
        Ok(())
    }

    fn refresh_ports(&mut self) {
        if self.midi.is_some() {
            let ports = self.midi.as_ref().unwrap().ports();
            self.labels = ports
                .iter()
                .enumerate()
                .map(|(index, port)| MidiPortLabel {
                    index,
                    name: self
                        .midi
                        .as_ref()
                        .unwrap()
                        .port_name(port)
                        .unwrap_or("[unnamed input]".to_string()),
                })
                .collect();
        }
    }

    // TODO: there's a race condition here. The label indexes are not
    // necessarily in sync with the current list of ports. I need to investigate
    // whether there's a more stable way to refer to individual ports.
    pub fn select_port(&mut self, index: usize) -> anyhow::Result<()> {
        if self.midi.is_none() {
            self.stop();
            if self.midi.is_none() {
                return Err(anyhow::Error::msg("MIDI input is not active".to_string()));
            }
        }
        let ports = self.midi.as_ref().unwrap().ports();
        if index >= ports.len() {
            return Err(anyhow::Error::msg(format!(
                "MIDI input port #{index} is no longer valid"
            )));
        }
        self.stop();
        self.active_port = None;

        let worker = Worker::<(u64, u8, MidiMessage)>::new_fifo();
        self.stealer = Some(worker.stealer());
        let selected_port = &ports[index];
        let selected_port_name = &self
            .midi
            .as_ref()
            .unwrap()
            .port_name(&ports[index])
            .unwrap_or("[unknown]".to_string());
        let selected_port_label = MidiPortLabel {
            index,
            name: selected_port_name.clone(),
        };
        match self.midi.take().unwrap().connect(
            selected_port,
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
                self.connection = Some(conn);
                self.active_port = Some(selected_port_label);
                Ok(())
            }
            Err(err) => Err(anyhow::Error::msg(err.to_string())),
        }
    }

    pub fn stop(&mut self) {
        if self.connection.is_some() {
            let close_result = self.connection.take().unwrap().close();
            self.midi = Some(close_result.0);
        }
    }

    pub fn labels(&self) -> (&Option<MidiPortLabel>, Vec<MidiPortLabel>) {
        (&self.active_port, self.labels.clone()) // TODO aaaaargh
    }
}
impl std::fmt::Debug for MidiInputHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MidiInputHandler")
            .field("conn_in", &0i32)
            .field("stealer", &self.stealer)
            .finish()
    }
}

/// Outputs MIDI messages to external MIDI devices.
#[derive(Uid)]
pub struct MidiOutputHandler {
    uid: usize,
    midi: Option<MidiOutput>,
    active_port: Option<MidiPortLabel>,
    labels: Vec<MidiPortLabel>,
    connection: Option<MidiOutputConnection>,
    stealer: Option<Stealer<(u64, u4, MidiMessage)>>,
    outputs: Vec<(usize, String)>,
}
impl std::fmt::Debug for MidiOutputHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({:?}, {:?})", self.stealer, self.outputs)
    }
}

impl Terminates for MidiOutputHandler {
    fn is_finished(&self) -> bool {
        true
    }
}

impl MidiOutputHandler {
    pub fn new() -> anyhow::Result<Self> {
        if let Ok(midi_out) = MidiOutput::new("Groove MIDI output") {
            Ok(Self {
                uid: Default::default(),
                midi: Some(midi_out),
                active_port: Default::default(),
                labels: Default::default(),
                connection: Default::default(),
                stealer: Default::default(),
                outputs: Default::default(),
            })
        } else {
            Err(anyhow::Error::msg("Couldn't create MIDI output"))
        }
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        self.refresh_ports();
        Ok(())
    }

    fn refresh_ports(&mut self) {
        if self.midi.is_some() {
            let ports = self.midi.as_ref().unwrap().ports();
            self.labels = ports
                .iter()
                .enumerate()
                .map(|(index, port)| MidiPortLabel {
                    index,
                    name: self
                        .midi
                        .as_ref()
                        .unwrap()
                        .port_name(port)
                        .unwrap_or("[unnamed output]".to_string()),
                })
                .collect();
        }
    }

    // TODO: race condition.
    pub fn select_port(&mut self, index: usize) -> anyhow::Result<()> {
        if self.midi.is_none() {
            self.stop();
            if self.midi.is_none() {
                return Err(anyhow::Error::msg("MIDI output is not active".to_string()));
            }
        }
        let ports = self.midi.as_ref().unwrap().ports();
        if index >= ports.len() {
            return Err(anyhow::Error::msg(format!(
                "MIDI output port #{index} is no longer valid"
            )));
        }
        self.stop();
        self.active_port = None;

        let worker = Worker::<(u64, u4, MidiMessage)>::new_fifo();
        self.stealer = Some(worker.stealer());
        let selected_port = &ports[index];
        let selected_port_name = &self
            .midi
            .as_ref()
            .unwrap()
            .port_name(&ports[index])
            .unwrap_or("[unknown]".to_string());
        let selected_port_label = MidiPortLabel {
            index,
            name: selected_port_name.clone(),
        };
        match self
            .midi
            .take()
            .unwrap()
            .connect(selected_port, "Groove output")
        {
            Ok(conn) => {
                self.connection = Some(conn);
                self.active_port = Some(selected_port_label);
                Ok(())
            }
            Err(err) => Err(anyhow::Error::msg(err.to_string())),
        }
    }

    pub fn send(&mut self, message: &[u8]) -> Result<(), SendError> {
        if self.connection.is_some() {
            self.connection.as_mut().unwrap().send(message)
        } else {
            Err(SendError::Other("couldn't send"))
        }
    }

    pub fn stop(&mut self) {
        if self.connection.is_some() {
            let close_result = self.connection.take().unwrap().close();
            self.midi = Some(close_result);
        }
    }

    pub fn labels(&self) -> (&Option<MidiPortLabel>, Vec<MidiPortLabel>) {
        (&self.active_port, self.labels.clone()) // TODO aaaaargh
    }

    // TODO: this looks like old Updateable::update() because it was one. It's
    // free to evolve independently.
    fn update(
        &mut self,
        _clock: &Clock,
        message: MidiHandlerMessage,
    ) -> Response<MidiHandlerMessage> {
        match message {
            MidiHandlerMessage::Midi(channel, message) => {
                let event = LiveEvent::Midi {
                    channel: u4::from(channel),
                    message,
                };

                // TODO: this seems like a lot of work
                let mut buf = Vec::new();
                event.write(&mut buf).unwrap();
                if self.send(&buf).is_err() {
                    // TODO
                }
            }
            _ => todo!(),
        }
        Response::none()
    }
}

#[derive(Clone, Debug, Default)]
pub enum MidiHandlerMessage {
    /// It's time to do periodic work.
    #[default]
    Tick,

    /// A MIDI message sent by Groove to MidiHandler for output to external MIDI
    /// devices.
    Midi(MidiChannel, MidiMessage),

    /// A new MIDI input or output has been selected in the UI.
    InputSelected(MidiPortLabel),
    OutputSelected(MidiPortLabel),
}
impl MessageBounds for MidiHandlerMessage {}

#[derive(Debug, Uid)]
pub struct MidiHandler {
    uid: usize,
    midi_input: Option<MidiInputHandler>,
    midi_output: Option<MidiOutputHandler>,

    activity_tick: Instant,
}
impl Default for MidiHandler {
    fn default() -> Self {
        let midi_input = MidiInputHandler::new().ok();
        let midi_output = MidiOutputHandler::new().ok();
        Self {
            uid: Default::default(),
            midi_input,
            midi_output,
            activity_tick: Instant::now(),
        }
    }
}
impl MidiHandler {
    fn update(&mut self, clock: &Clock, message: MidiHandlerMessage) -> Response<MidiHandlerEvent> {
        match message {
            MidiHandlerMessage::Tick => {
                if let Some(midi_input) = &self.midi_input {
                    if let Some(input_stealer) = &midi_input.stealer {
                        let mut commands = Vec::new();
                        while !input_stealer.is_empty() {
                            if let Steal::Success((_stamp, channel, message)) =
                                input_stealer.steal()
                            {
                                self.activity_tick = Instant::now();
                                commands.push(Response::single(MidiHandlerEvent::Midi(
                                    channel, message,
                                )));
                            }
                        }
                        if !commands.is_empty() {
                            return Response::batch(commands);
                        }
                    }
                }
            }
            MidiHandlerMessage::Midi(_, _) => {
                if self.midi_output.is_some() {
                    self.midi_output.as_mut().unwrap().update(clock, message);
                }
            }
            _ => {}
        }
        Response::none()
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        if self.midi_input.is_some() {
            self.midi_input.as_mut().unwrap().start()?;
        }
        if self.midi_output.is_some() {
            self.midi_output.as_mut().unwrap().start()?;
        }
        Ok(())
    }

    pub fn stop(&mut self) {
        if self.midi_input.is_some() {
            self.midi_input.as_mut().unwrap().stop();
        }
        if self.midi_output.is_some() {
            self.midi_output.as_mut().unwrap().stop();
        }
    }

    pub fn select_input(&mut self, which: MidiPortLabel) {
        if self.midi_input.is_some()
            && self
                .midi_input
                .as_mut()
                .unwrap()
                .select_port(which.index)
                .is_ok()
        {
            // swallow failure
        }
    }

    pub fn select_output(&mut self, which: MidiPortLabel) {
        if self.midi_output.is_some()
            && self
                .midi_output
                .as_mut()
                .unwrap()
                .select_port(which.index)
                .is_ok()
        {
            // swallow failure
        }
    }

    pub fn activity_tick(&self) -> Instant {
        self.activity_tick
    }

    pub fn midi_input(&self) -> Option<&MidiInputHandler> {
        self.midi_input.as_ref()
    }

    pub fn midi_output(&self) -> Option<&MidiOutputHandler> {
        self.midi_output.as_ref()
    }
}
impl Terminates for MidiHandler {
    fn is_finished(&self) -> bool {
        true
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use assert_approx_eq::assert_approx_eq;
    use midly::num::u7;

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
        assert_approx_eq!(MidiUtils::note_type_to_frequency(MidiNote::C0), 16.351_597);
        assert_approx_eq!(MidiUtils::note_type_to_frequency(MidiNote::C4), 261.625_55);
        assert_approx_eq!(MidiUtils::note_type_to_frequency(MidiNote::G9), 12_543.855);
    }
}
