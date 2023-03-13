// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! The [midi](crate::subscriptions::midi) module contains the
//! [Subscription](iced_native::subscription::Subscription) interface for the
//! [Groove](groove_core::Groove) MIDI engine.

use groove_core::midi::{u4, LiveEvent, MidiChannel, MidiMessage};
use groove_orchestration::messages::Response;
use iced::futures::channel::mpsc as iced_mpsc;
use iced::{subscription, Subscription};
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection, SendError};
use std::{fmt::Debug, sync::mpsc, thread::JoinHandle};

/// [MidiHandlerInput] messages allow the subscriber to communicate with the
/// MIDI engine through [MidiSubscription].
#[derive(Clone, Debug)]
pub enum MidiHandlerInput {
    /// Requests a rescan of the MIDI input/output ports.
    RefreshPorts,

    /// The user has picked a MIDI input. Switch to it.
    SelectMidiInput(MidiPortDescriptor),

    /// The user has picked a MIDI output. Switch to it.
    SelectMidiOutput(MidiPortDescriptor),

    /// We've been asked to send a MIDI message to external hardware. Normally
    /// these messages come from
    /// [EngineSubscription](crate::subscriptions::EngineSubscription).
    Midi(MidiChannel, MidiMessage),

    /// The app is ready to quit, so the [MidiSubscription] should end.
    QuitRequested,
}

/// The [MidiSubscription] subscriber receives [MidiHandlerEvent] messages from
/// the MIDI engine.
#[derive(Clone, Debug)]
pub enum MidiHandlerEvent {
    /// The subscription thread has successfully started, and sends this event
    /// first. It contains a message channel for further communication from the
    /// subscriber to the MIDI engine.
    Ready(mpsc::Sender<MidiHandlerInput>),

    /// The MIDI input ports have been updated.
    InputPorts(Vec<MidiPortDescriptor>),

    /// A new input port has been selected.
    InputPortSelected(Option<MidiPortDescriptor>),

    /// The MIDI output ports have been updated.
    OutputPorts(Vec<MidiPortDescriptor>),

    /// A new output port has been selected.
    OutputPortSelected(Option<MidiPortDescriptor>),

    /// A MIDI message has arrived from external hardware and should be handled
    /// in the app (probably by forwarding it to
    /// [EngineInput](crate::subscriptions::EngineInput::Midi)).
    Midi(MidiChannel, MidiMessage),

    /// The MIDI engine has successfully processed
    /// [MidiHandlerInput::QuitRequested], and it's OK to end the
    /// [MidiSubscription].
    Quit,
}

/// [MidiSubscription] provides an interface to the external MIDI world.
pub struct MidiSubscription {
    midi_handler: MidiHandler,
    receiver: mpsc::Receiver<MidiHandlerInput>,
}
impl MidiSubscription {
    /// Starts the subscription. The first message sent with the subscription
    /// will be [MidiHandlerEvent::Ready].
    pub fn subscription() -> Subscription<MidiHandlerEvent> {
        subscription::unfold(
            std::any::TypeId::of::<MidiSubscription>(),
            State::Start,
            |state| async move {
                match state {
                    State::Start => {
                        let (app_sender, app_receiver) = mpsc::channel::<MidiHandlerInput>();
                        let (thread_sender, thread_receiver) =
                            iced_mpsc::channel::<MidiHandlerEvent>(1024);

                        let handler = std::thread::spawn(move || {
                            // This could have been done inside the constructor,
                            // but it's legacy code and I don't think it makes a
                            // difference.
                            let mut midi_handler = MidiHandler::new_with(thread_sender);
                            let _ = midi_handler.start();

                            let mut subscription = Self::new_with(midi_handler, app_receiver);
                            subscription.do_loop();
                        });

                        (
                            Some(MidiHandlerEvent::Ready(app_sender)),
                            State::Ready(handler, thread_receiver),
                        )
                    }
                    State::Ready(handler, mut receiver) => {
                        use iced_native::futures::StreamExt;

                        let event = receiver.select_next_some().await;
                        if let MidiHandlerEvent::Quit = event {
                            (Some(event), State::Ending(handler))
                        } else {
                            (Some(event), State::Ready(handler, receiver))
                        }
                    }
                    State::Ending(handler) => {
                        let _ = handler.join();
                        // See https://github.com/iced-rs/iced/issues/1348
                        (None, State::Idle)
                    }
                    State::Idle => {
                        // I took this line from
                        // https://github.com/iced-rs/iced/issues/336, but I
                        // don't understand why it helps. I think it's necessary
                        // for the system to get a chance to process all the
                        // subscription results.
                        let _: () = iced::futures::future::pending().await;
                        (None, State::Idle)
                    }
                }
            },
        )
    }

    fn new_with(midi_handler: MidiHandler, app_receiver: mpsc::Receiver<MidiHandlerInput>) -> Self {
        Self {
            midi_handler,
            receiver: app_receiver,
        }
    }

    // This could be moved into MidiHandler, and pretty much get rid of what
    // used to be the Runner structure, and later MidiSubscription's methods.
    // MidiHandler already knows about channels, so it's not isolated from the
    // subscription stuff. Maybe TODO
    fn do_loop(&mut self) {
        loop {
            if let Ok(input) = self.receiver.recv() {
                let time_to_quit = if let MidiHandlerInput::QuitRequested = input {
                    true
                } else {
                    false
                };
                self.midi_handler.update(input);
                if time_to_quit {
                    break;
                }
            } else {
                eprintln!("MidiSubscription channel sender has hung up. Exiting...");
                break;
            }
        }
    }
}

/// Provides user-friendly strings for displaying available MIDI ports.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MidiPortDescriptor {
    index: usize,
    name: String,
}
impl MidiPortDescriptor {
    /// The port descriptor's index.
    pub fn index(&self) -> usize {
        self.index
    }

    /// The port descriptor's human-readable name.
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }
}
impl std::fmt::Display for MidiPortDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

/// Handles MIDI input coming from outside [Groove](groove_core::Groove). For
/// example, if you have a MIDI keyboard plugged into your computer's USB, you
/// should be able to use that keyboard to input notes, and [MidiInputHandler]
/// manages that.
pub struct MidiInputHandler {
    midi: Option<MidiInput>,
    active_port: Option<MidiPortDescriptor>,
    connection: Option<MidiInputConnection<()>>,
    sender: iced_mpsc::Sender<MidiHandlerEvent>,
}
impl MidiInputHandler {
    pub fn new_with(sender: iced_mpsc::Sender<MidiHandlerEvent>) -> anyhow::Result<Self> {
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
                .try_send(MidiHandlerEvent::InputPorts(descriptors));
        }
    }

    // TODO: this has a race condition. The label indexes are not necessarily in
    // sync with the current list of ports. I need to investigate whether
    // there's a more stable way to refer to individual ports.
    //
    // I think the question boils down to how long a MidiInputPort is valid.
    pub fn select_port(&mut self, index: usize) -> anyhow::Result<()> {
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
            let mut sender_clone = self.sender.clone();

            // I don't know how this take() works when we've already gotten the
            // mutable midi at the top of this block. Maybe it's because we
            // don't refer to that local &mut midi after this point. If so, the
            // bounds checker is being pretty smart.
            match self.midi.take().unwrap().connect(
                selected_port,
                "Groove input",
                move |_, event, _| {
                    if let Ok(event) = LiveEvent::parse(event) {
                        if let LiveEvent::Midi { channel, message } = event {
                            let _ = sender_clone.try_send(MidiHandlerEvent::Midi(
                                MidiChannel::from(channel),
                                message,
                            ));
                        }
                    }
                },
                (),
            ) {
                // By this point, the self.midi is None, and the conn we just
                // got back is active.
                //
                // The thing that's super-weird about this API is that either
                // self.midi or self.connection has the MidiInput or MidiOutput,
                // but never both at the same time. It keeps getting passed
                // back/forth like a hot potato.
                Ok(conn) => {
                    self.connection = Some(conn);
                    self.active_port = Some(selected_port_label);
                    Ok(())
                }
                Err(err) => Err(anyhow::Error::msg(err.to_string())),
            }
        } else {
            // This shouldn't happen; if it did, it means we had a
            // Some(self.midi) and then a None immediately after.
            Ok(())
        }
    }

    pub fn stop(&mut self) {
        if self.connection.is_some() {
            let close_result = self.connection.take().unwrap().close();
            self.midi = Some(close_result.0);
        }
    }
}
impl std::fmt::Debug for MidiInputHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MidiInputHandler")
            .field("conn_in", &0i32)
            .finish()
    }
}

// TODO: these shouldn't need to be public.
/// Outputs MIDI messages to external MIDI devices.
pub struct MidiOutputHandler {
    midi: Option<MidiOutput>,
    active_port: Option<MidiPortDescriptor>,
    connection: Option<MidiOutputConnection>,
    outputs: Vec<(usize, String)>,
    sender: iced_mpsc::Sender<MidiHandlerEvent>,
}
impl std::fmt::Debug for MidiOutputHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.outputs)
    }
}
impl MidiOutputHandler {
    pub fn new_with(sender: iced_mpsc::Sender<MidiHandlerEvent>) -> anyhow::Result<Self> {
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
            let _ = self.sender.try_send(MidiHandlerEvent::OutputPorts(
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
    pub fn select_port(&mut self, index: usize) -> anyhow::Result<()> {
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
                    self.active_port = Some(selected_port_label);
                    Ok(())
                }
                Err(err) => Err(anyhow::Error::msg(err.to_string())),
            }
        } else {
            Ok(())
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
}

/// Manages the external MIDI interface.
#[derive(Debug)]
struct MidiHandler {
    midi_input: Option<MidiInputHandler>,
    midi_output: Option<MidiOutputHandler>,

    sender: iced_mpsc::Sender<MidiHandlerEvent>,
}
impl MidiHandler {
    /// Creates the [MidiHandler], providing a [WakeupSignaler] that it should
    /// use to inform the event handler of new events.
    pub fn new_with(sender: iced_mpsc::Sender<MidiHandlerEvent>) -> Self {
        let midi_input = MidiInputHandler::new_with(sender.clone()).ok();
        let midi_output = MidiOutputHandler::new_with(sender.clone()).ok();
        Self {
            midi_input,
            midi_output,
            sender,
        }
    }
    fn update(&mut self, message: MidiHandlerInput) -> Response<MidiHandlerEvent> {
        match message {
            MidiHandlerInput::Midi(channel, message) => {
                self.handle_midi(channel, message);
            }
            MidiHandlerInput::SelectMidiInput(which) => self.select_input(which),
            MidiHandlerInput::SelectMidiOutput(which) => self.select_output(which),
            MidiHandlerInput::QuitRequested => self.stop(),
            MidiHandlerInput::RefreshPorts => self.refresh_ports(),
        }
        Response::none()
    }

    fn handle_midi(&mut self, channel: MidiChannel, message: MidiMessage) {
        if let Some(midi) = self.midi_output.as_mut() {
            midi.handle_midi(channel, message);
        }
    }

    fn start(&mut self) -> anyhow::Result<()> {
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
        let _ = self.sender.try_send(MidiHandlerEvent::Quit);
    }

    fn select_input(&mut self, which: MidiPortDescriptor) {
        if let Some(input) = self.midi_input.as_mut() {
            if let Ok(_) = input.select_port(which.index) {
                let _ = self.sender.try_send(MidiHandlerEvent::InputPortSelected(
                    input.active_port.clone(),
                ));
            }
        };
    }

    fn select_output(&mut self, which: MidiPortDescriptor) {
        if let Some(output) = self.midi_output.as_mut() {
            if let Ok(_) = output.select_port(which.index) {
                let _ = self.sender.try_send(MidiHandlerEvent::OutputPortSelected(
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
}

/// This is used by [MidiSubscription]. I put it way down here so it doesn't
/// distract.
enum State {
    Start,
    Ready(JoinHandle<()>, iced_mpsc::Receiver<MidiHandlerEvent>),
    Ending(JoinHandle<()>),
    Idle,
}
