// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crossbeam_channel::{Receiver, Sender};
use eframe::egui::{self, CollapsingHeader, ComboBox};
use groove_core::{
    midi::{MidiChannel, MidiMessage},
    traits::gui::Shows,
};
use groove_midi::{
    MidiInterfaceEvent, MidiInterfaceInput, MidiInterfaceService, MidiPortDescriptor,
};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

/// The panel provides updates to the app through [MidiPanelEvent] messages.
#[derive(Clone, Debug)]
pub enum MidiPanelEvent {
    /// A MIDI message arrived from the interface.
    Midi(MidiChannel, MidiMessage),

    /// The user has picked a MIDI input. Switch to it.
    ///
    /// Inputs are sent by the PC to the interface.
    SelectInput(MidiPortDescriptor),

    /// The user has picked a MIDI output. Switch to it.
    ///
    /// Outputs are sent by the interfaace to the PC.
    SelectOutput(MidiPortDescriptor),

    /// The requested port refresh is complete.
    PortsRefreshed,
}

/// [MidiPanel] manages external MIDI hardware interfaces.
#[derive(Debug)]
pub struct MidiPanel {
    sender: Sender<MidiInterfaceInput>, // for us to send to the interface
    app_receiver: Receiver<MidiPanelEvent>, // to give to the app to receive what we sent
    app_sender: Sender<MidiPanelEvent>, // for us to send to the app

    inputs: Arc<Mutex<Vec<MidiPortDescriptor>>>,
    selected_input: Arc<Mutex<Option<MidiPortDescriptor>>>,
    outputs: Arc<Mutex<Vec<MidiPortDescriptor>>>,
    selected_output: Arc<Mutex<Option<MidiPortDescriptor>>>,

    last_input_instant: Arc<Mutex<Instant>>,
    last_output_instant: Instant,
}
impl Default for MidiPanel {
    fn default() -> Self {
        let midi_interface_service = MidiInterfaceService::default();
        let sender = midi_interface_service.sender().clone();

        let (app_sender, app_receiver) = crossbeam_channel::unbounded();

        let r = Self {
            sender,
            app_receiver,
            app_sender,

            inputs: Default::default(),
            selected_input: Default::default(),

            outputs: Default::default(),
            selected_output: Default::default(),

            last_input_instant: Arc::new(Mutex::new(Instant::now())),
            last_output_instant: Instant::now(),
        };
        r.start_midi_interface(midi_interface_service.receiver().clone());
        r
    }
}
impl MidiPanel {
    /// Sends a [MidiInterfaceInput] message to the service.
    pub fn send(&mut self, input: MidiInterfaceInput) {
        if let MidiInterfaceInput::Midi(..) = input {
            self.last_output_instant = Instant::now();
        }

        let _ = self.sender.send(input);
    }

    // Sits in a loop, watching the receiving side of the event channel and
    // handling whatever comes through.
    fn start_midi_interface(&self, receiver: Receiver<MidiInterfaceEvent>) {
        let inputs = Arc::clone(&self.inputs);
        let selected_input = Arc::clone(&self.selected_input);
        let outputs = Arc::clone(&self.outputs);
        let selected_output = Arc::clone(&self.selected_output);
        let last_input_instant = Arc::clone(&self.last_input_instant);
        let app_sender = self.app_sender.clone();
        std::thread::spawn(move || {
            let mut inputs_refreshed = false;
            let mut outputs_refreshed = false;
            let mut refresh_sent = false;
            loop {
                if let Ok(event) = receiver.recv() {
                    match event {
                        MidiInterfaceEvent::Ready => {}
                        MidiInterfaceEvent::InputPorts(ports) => {
                            if let Ok(mut inputs) = inputs.lock() {
                                *inputs = ports.clone();
                                inputs_refreshed = true;
                            }
                        }
                        MidiInterfaceEvent::InputPortSelected(port) => {
                            if let Ok(mut selected_input) = selected_input.lock() {
                                *selected_input = port;
                            }
                        }
                        MidiInterfaceEvent::OutputPorts(ports) => {
                            if let Ok(mut outputs) = outputs.lock() {
                                *outputs = ports.clone();
                                outputs_refreshed = true;
                            }
                        }
                        MidiInterfaceEvent::OutputPortSelected(port) => {
                            if let Ok(mut selected_output) = selected_output.lock() {
                                *selected_output = port;
                            }
                        }
                        MidiInterfaceEvent::Midi(channel, message) => {
                            if let Ok(mut last_input_instant) = last_input_instant.lock() {
                                *last_input_instant = Instant::now();
                            }
                            let _ = app_sender.send(MidiPanelEvent::Midi(channel, message));
                        }
                        MidiInterfaceEvent::Quit => break,
                    }
                } else {
                    eprintln!("unexpected failure of MidiInterface channel");
                    break;
                }
                if !refresh_sent && inputs_refreshed && outputs_refreshed {
                    refresh_sent = true;
                    let _ = app_sender.send(MidiPanelEvent::PortsRefreshed);
                }
            }
        });
    }

    fn inputs(&self) -> &Mutex<Vec<MidiPortDescriptor>> {
        self.inputs.as_ref()
    }

    fn outputs(&self) -> &Mutex<Vec<MidiPortDescriptor>> {
        self.outputs.as_ref()
    }

    /// The receive side of the [MidiPanelEvent] channel
    pub fn receiver(&self) -> &Receiver<MidiPanelEvent> {
        &self.app_receiver
    }
}
impl Shows for MidiPanel {
    fn show(&mut self, ui: &mut egui::Ui) {
        CollapsingHeader::new("MIDI")
            .default_open(true)
            .show(ui, |ui| {
                let now = Instant::now();
                let last_input_instant = *self.last_input_instant.lock().unwrap();
                let input_was_recent = (now - last_input_instant).as_millis() < 250;
                let output_was_recent = (now - self.last_output_instant).as_millis() < 250;

                ui.label(format!(
                    "in: {} out: {}",
                    if input_was_recent { "•" } else { "◦" },
                    if output_was_recent { "•" } else { "◦" }
                ));

                if let Ok(ports) = &self.inputs().lock() {
                    let mut cb = ComboBox::from_label("MIDI in");
                    let (mut selected_index, _selected_text) =
                        if let Some(selected) = &(*self.selected_input.lock().unwrap()) {
                            cb = cb.selected_text(selected.name());
                            (selected.index(), selected.name())
                        } else {
                            (usize::MAX, "None")
                        };
                    cb.show_ui(ui, |ui| {
                        for port in ports.iter() {
                            if ui
                                .selectable_value(&mut selected_index, port.index(), port.name())
                                .changed()
                            {
                                let _ = self
                                    .sender
                                    .send(MidiInterfaceInput::SelectMidiInput(port.clone()));
                                let _ = self
                                    .app_sender
                                    .send(MidiPanelEvent::SelectInput(port.clone()));
                            }
                        }
                    });
                }
                ui.end_row();

                if let Ok(ports) = &self.outputs().lock() {
                    let mut cb = ComboBox::from_label("MIDI out");
                    let (mut selected_index, _selected_text) =
                        if let Some(selected) = &(*self.selected_output.lock().unwrap()) {
                            cb = cb.selected_text(selected.name());
                            (selected.index(), selected.name())
                        } else {
                            (usize::MAX, "None")
                        };
                    cb.show_ui(ui, |ui| {
                        for port in ports.iter() {
                            if ui
                                .selectable_value(&mut selected_index, port.index(), port.name())
                                .changed()
                            {
                                let _ = self
                                    .sender
                                    .send(MidiInterfaceInput::SelectMidiOutput(port.clone()));
                                let _ = self
                                    .app_sender
                                    .send(MidiPanelEvent::SelectOutput(port.clone()));
                            }
                        }
                    });
                }
                ui.end_row();
            });
    }
}
