// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! The [midi](crate::subscriptions::midi) module contains the
//! [Subscription](iced_native::subscription::Subscription) interface for the
//! [Groove](groove_core::Groove) MIDI engine.

use groove_core::midi::{u4, LiveEvent, MidiChannel, MidiMessage};
use groove_orchestration::messages::Response;
use iced::futures::channel::mpsc as iced_mpsc;
use iced::{subscription, Subscription};
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection, SendError};
use std::{
    fmt::Debug,
    sync::{mpsc, Arc, Mutex},
    thread::JoinHandle,
};

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

    /// The MIDI input ports have been updated, and/or a new active port has been selected.
    InputPorts(Vec<MidiPortDescriptor>, Option<MidiPortDescriptor>),

    /// The MIDI output ports have been updated, and/or a new active port has been selected.
    OutputPorts(Vec<MidiPortDescriptor>, Option<MidiPortDescriptor>),

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
    midi_handler: Arc<Mutex<MidiHandler>>,
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

                        let mut t = MidiHandler::new_with(thread_sender);
                        let _ = t.start();
                        let midi_handler = Arc::new(Mutex::new(t));
                        let handler = std::thread::spawn(move || {
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

    fn new_with(
        midi_handler: Arc<Mutex<MidiHandler>>,
        app_receiver: mpsc::Receiver<MidiHandlerInput>,
    ) -> Self {
        Self {
            midi_handler,
            receiver: app_receiver,
        }
    }

    fn do_loop(&mut self) {
        loop {
            if let Ok(input) = self.receiver.recv() {
                let time_to_quit = if let MidiHandlerInput::QuitRequested = input {
                    true
                } else {
                    false
                };
                if let Ok(mut midi_handler) = self.midi_handler.lock() {
                    midi_handler.update(input);
                } else {
                    eprintln!("MidiSubscription channel sender has hung up. Exiting...");
                    break;
                }
                if time_to_quit {
                    break;
                }
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
        if self.midi.is_some() {
            let ports = self.midi.as_ref().unwrap().ports();
            let descriptors: Vec<MidiPortDescriptor> = ports
                .iter()
                .enumerate()
                .map(|(index, port)| MidiPortDescriptor {
                    index,
                    name: self
                        .midi
                        .as_ref()
                        .unwrap()
                        .port_name(port)
                        .unwrap_or("[unnamed input]".to_string()),
                })
                .collect();
            let _ = self.sender.try_send(MidiHandlerEvent::InputPorts(
                descriptors,
                self.active_port.clone(),
            ));
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

        let selected_port = &ports[index];
        let selected_port_name = &self
            .midi
            .as_ref()
            .unwrap()
            .port_name(&ports[index])
            .unwrap_or("[unknown]".to_string());
        let selected_port_label = MidiPortDescriptor {
            index,
            name: selected_port_name.clone(),
        };

        // We need to clone our copy because we don't want the thread holding
        // onto a self reference.
        let mut sender_clone = self.sender.clone();

        match self.midi.take().unwrap().connect(
            selected_port,
            "Groove input",
            move |_, event, _| {
                let event = LiveEvent::parse(event).unwrap();
                if let LiveEvent::Midi { channel, message } = event {
                    let _ = sender_clone
                        .try_send(MidiHandlerEvent::Midi(MidiChannel::from(channel), message));
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
        if self.midi.is_some() {
            let ports = self.midi.as_ref().unwrap().ports();
            let _ = self.sender.try_send(MidiHandlerEvent::OutputPorts(
                ports
                    .iter()
                    .enumerate()
                    .map(|(index, port)| MidiPortDescriptor {
                        index,
                        name: self
                            .midi
                            .as_ref()
                            .unwrap()
                            .port_name(port)
                            .unwrap_or("[unnamed output]".to_string()),
                    })
                    .collect(),
                self.active_port.clone(),
            ));
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

        let selected_port = &ports[index];
        let selected_port_name = &self
            .midi
            .as_ref()
            .unwrap()
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
                input.refresh_ports();
            }
        };
    }

    fn select_output(&mut self, which: MidiPortDescriptor) {
        if let Some(output) = self.midi_output.as_mut() {
            if let Ok(_) = output.select_port(which.index) {
                output.refresh_ports();
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
