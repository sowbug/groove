// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! This crate makes it easy to use an external MIDI interface. To use it,
//! create a [MidiInterfaceService], then use its sender and receiver channels
//! to exchange [MidiHandlerInput] and [MidiHandlerEvent] messages.

use crossbeam_channel::{unbounded, Receiver, Sender};
use ensnare::midi::{u4, LiveEvent, MidiChannel, MidiMessage};
use ensnare_midi_interface::MidiPortDescriptor;
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection, SendError};
use std::{fmt::Debug, thread::JoinHandle};

/// The client sends requests to the MIDI interface through [MidiInterfaceInput] messages.
#[derive(Clone, Debug)]
pub enum MidiInterfaceInput {
    /// Requests a rescan of the MIDI input/output ports.
    RefreshPorts,

    /// The user has picked a MIDI input. Switch to it.
    ///
    /// Inputs are sent by the PC to the interface.
    SelectMidiInput(MidiPortDescriptor),

    /// The user has picked a MIDI output. Switch to it.
    ///
    /// Outputs are sent by the interfaace to the PC.
    SelectMidiOutput(MidiPortDescriptor),

    /// The application wants to send a MIDI message to external hardware.
    Midi(MidiChannel, MidiMessage),

    /// The app is ready to quit, so the service should end.
    QuitRequested,

    /// Attempt to set the selected MIDI input by matching a text description.
    RestoreMidiInput(String),

    /// Attempt to set the selected MIDI output by matching a text description.
    RestoreMidiOutput(String),
}

/// The service provides updates to the client through [MidiInterfaceEvent]
/// messages.
#[derive(Clone, Debug)]
pub enum MidiInterfaceEvent {
    /// The service has successfully started. It sends this event first. It's
    /// not important to wait for this event, because anything sent on the input
    /// channel will queue up until the service is ready to handle it..
    Ready,

    /// The MIDI input ports have been updated.
    InputPorts(Vec<MidiPortDescriptor>),

    /// A new input port has been selected.
    InputPortSelected(Option<MidiPortDescriptor>),

    /// The MIDI output ports have been updated.
    OutputPorts(Vec<MidiPortDescriptor>),

    /// A new output port has been selected.
    OutputPortSelected(Option<MidiPortDescriptor>),

    /// A MIDI message has arrived from external hardware.
    Midi(MidiChannel, MidiMessage),

    /// The MIDI engine has successfully processed
    /// [MidiHandlerInput::QuitRequested], and the service will go away shortly.
    Quit,
}

/// [MidiInterfaceService] provides an interface to external MIDI hardware,
/// thanks to the `midir` crate.
pub struct MidiInterfaceService {
    input_sender: Sender<MidiInterfaceInput>,
    event_receiver: Receiver<MidiInterfaceEvent>,

    #[allow(dead_code)]
    handler: JoinHandle<()>,
}
impl Default for MidiInterfaceService {
    fn default() -> Self {
        Self::new()
    }
}
impl MidiInterfaceService {
    pub fn new() -> Self {
        // Sends input from the app to the service.
        let (input_sender, input_receiver) = unbounded();

        // Sends events from the service to the app.
        let (event_sender, event_receiver) = unbounded();

        let handler = std::thread::spawn(move || {
            let mut midi_interface = MidiInterface::new_with(event_sender.clone());
            let _ = midi_interface.start();
            let _ = event_sender.send(MidiInterfaceEvent::Ready);

            loop {
                if let Ok(input) = input_receiver.recv() {
                    match input {
                        MidiInterfaceInput::Midi(channel, message) => {
                            midi_interface.handle_midi(channel, message);
                        }
                        MidiInterfaceInput::SelectMidiInput(which) => {
                            midi_interface.select_input(which)
                        }
                        MidiInterfaceInput::SelectMidiOutput(which) => {
                            midi_interface.select_output(which)
                        }
                        MidiInterfaceInput::QuitRequested => {
                            midi_interface.stop();
                            break;
                        }
                        MidiInterfaceInput::RefreshPorts => midi_interface.refresh_ports(),
                        MidiInterfaceInput::RestoreMidiInput(port_name) => {
                            midi_interface.restore_input(port_name);
                        }
                        MidiInterfaceInput::RestoreMidiOutput(port_name) => {
                            midi_interface.restore_output(port_name);
                        }
                    }
                } else {
                    eprintln!("MidiInterfaceService channel sender has hung up. Exiting...");
                    break;
                }
            }
        });
        Self {
            input_sender,
            event_receiver,
            handler,
        }
    }

    pub fn sender(&self) -> &Sender<MidiInterfaceInput> {
        &self.input_sender
    }

    pub fn receiver(&self) -> &Receiver<MidiInterfaceEvent> {
        &self.event_receiver
    }
}

/// Handles MIDI input arriving via `midir` (e.g., via a MIDI keyboard plugged
/// into your computer's USB).
struct MidiInputHandler {
    midi: Option<MidiInput>,
    active_port: Option<MidiPortDescriptor>,
    connection: Option<MidiInputConnection<()>>,
    sender: Sender<MidiInterfaceEvent>,
}
impl MidiInputHandler {
    pub fn new_with(sender: Sender<MidiInterfaceEvent>) -> anyhow::Result<Self> {
        if let Ok(midi_input) = MidiInput::new("Groove MIDI input") {
            Ok(Self {
                midi: Some(midi_input),
                active_port: Default::default(),
                connection: Default::default(),
                sender,
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
        // This won't work if we have an active connection. I think that's by
        // design. So we have to be careful, because if we ever want to refresh
        // the ports, we'll need to disconnect -- which means we can't piggyback
        // the active port on the InputPorts/OutputPorts messages, because that
        // would mean disconnecting the connection, which would mean the active
        // port is no longer active!
        if let Some(midi) = self.midi.as_mut() {
            let ports = midi.ports();
            let descriptors: Vec<MidiPortDescriptor> = ports
                .iter()
                .enumerate()
                .map(|(index, port)| MidiPortDescriptor {
                    index,
                    name: midi
                        .port_name(port)
                        .unwrap_or("[unnamed input]".to_string()),
                })
                .collect();
            let _ = self
                .sender
                .try_send(MidiInterfaceEvent::InputPorts(descriptors));
        }
    }

    // TODO: this has a race condition. The label indexes are not necessarily in
    // sync with the current list of ports. I need to investigate whether
    // there's a more stable way to refer to individual ports.
    //
    // I think the question boils down to how long a MidiInputPort is valid.
    pub fn select_port(
        &mut self,
        index: usize,
    ) -> anyhow::Result<MidiPortDescriptor, anyhow::Error> {
        if self.midi.is_none() {
            // self.connection is probably Some()
            self.stop();
            // so now self.midi should be Some()
            if self.midi.is_none() {
                return Err(anyhow::Error::msg("MIDI input is not active".to_string()));
            }
        }

        // The connection is closed, so self.midi should be Some()
        if let Some(midi) = self.midi.as_mut() {
            let ports = midi.ports();
            if index >= ports.len() {
                return Err(anyhow::Error::msg(format!(
                    "MIDI input port #{index} is no longer valid"
                )));
            }

            // This was here, but I don't think it can do anything at this point.
            //        self.stop();

            self.active_port = None;

            let selected_port = &ports[index];
            let selected_port_name = &midi
                .port_name(&ports[index])
                .unwrap_or("[unknown]".to_string());
            let selected_port_label = MidiPortDescriptor {
                index,
                name: selected_port_name.clone(),
            };

            // We need to clone our copy because we don't want the thread holding
            // onto a self reference.
            let sender_clone = self.sender.clone();

            // I don't know how this take() works when we've already gotten the
            // mutable midi at the top of this block. Maybe it's because we
            // don't refer to that local &mut midi after this point. If so, the
            // bounds checker is being pretty smart.
            match self.midi.take().unwrap().connect(
                selected_port,
                "Groove input",
                move |_, event, _| {
                    if let Ok(LiveEvent::Midi { channel, message }) = LiveEvent::parse(event) {
                        let _ = sender_clone.try_send(MidiInterfaceEvent::Midi(
                            MidiChannel::from(channel),
                            message,
                        ));
                    }
                },
                (),
            ) {
                // By this point, the self.midi is None, and the conn we just
                // got back is active.
                //
                // What's super-weird about this API is that either self.midi or
                // self.connection has the MidiInput or MidiOutput, but never
                // both at the same time. It keeps getting passed back/forth
                // like a hot potato.
                Ok(conn) => {
                    self.connection = Some(conn);
                    self.active_port = Some(selected_port_label.clone());
                    Ok(selected_port_label)
                }
                Err(err) => Err(anyhow::Error::msg(err.to_string())),
            }
        } else {
            // This shouldn't happen; if it did, it means we had a
            // Some(self.midi) and then a None immediately after.
            Err(anyhow::format_err!("not sure what happened"))
        }
    }

    pub fn stop(&mut self) {
        if self.connection.is_some() {
            let close_result = self.connection.take().unwrap().close();
            self.midi = Some(close_result.0);
        }
    }

    fn restore_port(&mut self, port_name: String) -> Result<MidiPortDescriptor, anyhow::Error> {
        if let Some(midi) = self.midi.as_ref() {
            for (index, port) in midi.ports().iter().enumerate() {
                if let Ok(name) = midi.port_name(port) {
                    if name == port_name {
                        return self.select_port(index);
                    }
                }
            }
        }
        Err(anyhow::format_err!("failed to restore input port"))
    }
}
impl std::fmt::Debug for MidiInputHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MidiInputHandler")
            .field("conn_in", &0i32)
            .finish()
    }
}

/// Outputs MIDI messages to external MIDI devices.
struct MidiOutputHandler {
    midi: Option<MidiOutput>,
    active_port: Option<MidiPortDescriptor>,
    connection: Option<MidiOutputConnection>,
    outputs: Vec<(usize, String)>,
    sender: Sender<MidiInterfaceEvent>,
}
impl std::fmt::Debug for MidiOutputHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.outputs)
    }
}
impl MidiOutputHandler {
    fn new_with(sender: Sender<MidiInterfaceEvent>) -> anyhow::Result<Self> {
        if let Ok(midi_out) = MidiOutput::new("Groove MIDI output") {
            Ok(Self {
                midi: Some(midi_out),
                active_port: Default::default(),
                connection: Default::default(),
                outputs: Default::default(),
                sender,
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
        if let Some(midi) = self.midi.as_mut() {
            let ports = midi.ports();
            let _ = self.sender.try_send(MidiInterfaceEvent::OutputPorts(
                ports
                    .iter()
                    .enumerate()
                    .map(|(index, port)| MidiPortDescriptor {
                        index,
                        name: midi
                            .port_name(port)
                            .unwrap_or("[unnamed output]".to_string()),
                    })
                    .collect(),
            ));
        }
    }

    // TODO: race condition.
    pub fn select_port(
        &mut self,
        index: usize,
    ) -> anyhow::Result<MidiPortDescriptor, anyhow::Error> {
        if self.midi.is_none() {
            // self.connection is probably Some()
            self.stop();
            // so now self.midi should be Some()
            if self.midi.is_none() {
                return Err(anyhow::Error::msg("MIDI input is not active".to_string()));
            }
        }

        if let Some(midi) = self.midi.as_mut() {
            let ports = midi.ports();
            if index >= ports.len() {
                return Err(anyhow::Error::msg(format!(
                    "MIDI output port #{index} is no longer valid"
                )));
            }
            self.active_port = None;

            let selected_port = &ports[index];
            let selected_port_name = &midi
                .port_name(&ports[index])
                .unwrap_or("[unknown]".to_string());
            let selected_port_label = MidiPortDescriptor {
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
                    self.active_port = Some(selected_port_label.clone());
                    Ok(selected_port_label)
                }
                Err(err) => Err(anyhow::Error::msg(err.to_string())),
            }
        } else {
            Err(anyhow::format_err!("unexpected - output"))
        }
    }

    pub fn send(&mut self, message: &[u8]) -> Result<(), SendError> {
        if let Some(c) = self.connection.as_mut() {
            c.send(message)
        } else {
            Err(SendError::Other("couldn't send"))
        }
    }

    pub fn stop(&mut self) {
        // Note that take() -- this is weird. Leave it alone.
        if self.connection.is_some() {
            let close_result = self.connection.take().unwrap().close();
            self.midi = Some(close_result);
        }
    }

    fn handle_midi(&mut self, channel: MidiChannel, message: MidiMessage) {
        let event = LiveEvent::Midi {
            channel: u4::from(channel.0),
            message,
        };

        // TODO: this seems like a lot of work
        let mut buf = Vec::new();
        event.write(&mut buf).unwrap();
        if self.send(&buf).is_err() {
            // TODO
        }
    }

    fn restore_port(&mut self, port_name: String) -> Result<MidiPortDescriptor, anyhow::Error> {
        if let Some(midi) = self.midi.as_ref() {
            for (index, port) in midi.ports().iter().enumerate() {
                if let Ok(name) = midi.port_name(port) {
                    if name == port_name {
                        return self.select_port(index);
                    }
                }
            }
        }
        Err(anyhow::format_err!("failed to restore input port"))
    }
}

/// Manages the external MIDI interface.
#[derive(Debug)]
struct MidiInterface {
    midi_input: Option<MidiInputHandler>,
    midi_output: Option<MidiOutputHandler>,

    sender: Sender<MidiInterfaceEvent>,
}
impl MidiInterface {
    pub fn new_with(sender: Sender<MidiInterfaceEvent>) -> Self {
        let midi_input = MidiInputHandler::new_with(sender.clone()).ok();
        let midi_output = MidiOutputHandler::new_with(sender.clone()).ok();
        Self {
            midi_input,
            midi_output,
            sender,
        }
    }

    fn handle_midi(&mut self, channel: MidiChannel, message: MidiMessage) {
        if let Some(midi) = self.midi_output.as_mut() {
            midi.handle_midi(channel, message);
        }
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        if let Some(midi) = self.midi_input.as_mut() {
            midi.start()?;
        }
        if let Some(midi) = self.midi_output.as_mut() {
            midi.start()?;
        }
        Ok(())
    }

    fn stop(&mut self) {
        if let Some(midi) = self.midi_input.as_mut() {
            midi.stop();
        }
        if let Some(midi) = self.midi_output.as_mut() {
            midi.stop();
        }
        let _ = self.sender.try_send(MidiInterfaceEvent::Quit);
    }

    fn select_input(&mut self, which: MidiPortDescriptor) {
        if let Some(input) = self.midi_input.as_mut() {
            if input.select_port(which.index).is_ok() {
                let _ = self.sender.try_send(MidiInterfaceEvent::InputPortSelected(
                    input.active_port.clone(),
                ));
            }
        };
    }

    fn select_output(&mut self, which: MidiPortDescriptor) {
        if let Some(output) = self.midi_output.as_mut() {
            if output.select_port(which.index).is_ok() {
                let _ = self.sender.try_send(MidiInterfaceEvent::OutputPortSelected(
                    output.active_port.clone(),
                ));
            }
        };
    }

    fn refresh_ports(&mut self) {
        if let Some(input) = self.midi_input.as_mut() {
            input.refresh_ports();
        }
        if let Some(output) = self.midi_output.as_mut() {
            output.refresh_ports();
        }
    }

    fn restore_input(&mut self, port_name: String) {
        if let Some(handler) = self.midi_input.as_mut() {
            if let Ok(which) = handler.restore_port(port_name) {
                self.select_input(which);
            }
        }
    }

    fn restore_output(&mut self, port_name: String) {
        if let Some(handler) = self.midi_output.as_mut() {
            if let Ok(which) = handler.restore_port(port_name) {
                self.select_output(which);
            }
        }
    }
}
