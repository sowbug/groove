pub use subscription::{MidiHandlerEvent, MidiHandlerInput, MidiSubscription};

pub(crate) mod programmers;
pub(crate) mod subscription;

// TODO copy and conform MidiMessage to MessageBounds so it can be a trait
// associated type
use crate::messages::Response;
use crossbeam::deque::{Steal, Stealer, Worker};
use enum_primitive_derive::Primitive;
use groove_core::midi::{MidiChannel, MidiMessage};
use groove_core::{midi::u4, traits::MessageBounds};
use groove_macros::Uid;
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection, SendError};
pub use midly::live::LiveEvent;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, time::Instant};
use strum_macros::Display;

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone, Serialize, Deserialize, Copy, Default)]
pub enum MidiMessageType {
    #[default] // there isn't any sensible default here, so we pick something loud
    NoteOn = 0b1001,
    NoteOff = 0b1000,
    ProgramChange = 0b1100,
    Controller,
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
    fn update(&mut self, message: MidiHandlerMessage) -> Response<MidiHandlerMessage> {
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
    fn update(&mut self, message: MidiHandlerMessage) -> Response<MidiHandlerEvent> {
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
                    self.midi_output.as_mut().unwrap().update(message);
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
