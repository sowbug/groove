// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::Message;
use crossbeam_channel::{Receiver, Sender};
use eframe::egui::{self, CollapsingHeader, ComboBox};
use groove_core::traits::gui::Shows;
use groove_midi::{
    MidiInterfaceEvent, MidiInterfaceInput, MidiInterfaceService, MidiPortDescriptor,
};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

/// [MidiPanel] manages external MIDI hardware interfaces.
#[derive(Debug)]
pub struct MidiPanel {
    sender: Sender<MidiInterfaceInput>,
    app_sender: Sender<Message>,

    inputs: Arc<Mutex<Vec<MidiPortDescriptor>>>,
    selected_input: Arc<Mutex<Option<MidiPortDescriptor>>>,
    outputs: Arc<Mutex<Vec<MidiPortDescriptor>>>,
    selected_output: Arc<Mutex<Option<MidiPortDescriptor>>>,

    last_input_instant: Arc<Mutex<Instant>>,
    last_output_instant: Instant,
}
impl MidiPanel {
    /// Creates a new [MidiPanel].
    pub fn new_with(app_sender: Sender<Message>) -> Self {
        let midi_interface_service = MidiInterfaceService::default();
        let sender = midi_interface_service.sender().clone();

        let r = Self {
            sender,
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

    // TODO: figure out how we're going to get events back from Orchestrator.
    // This includes MIDI events that we'll then send to the MIDI interface.
    #[allow(dead_code)]
    pub(crate) fn send(&mut self, input: MidiInterfaceInput) {
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
        std::thread::spawn(move || loop {
            if let Ok(event) = receiver.recv() {
                match event {
                    groove_midi::MidiInterfaceEvent::Ready => {}
                    groove_midi::MidiInterfaceEvent::InputPorts(ports) => {
                        if let Ok(mut inputs) = inputs.lock() {
                            *inputs = ports.clone();
                        }
                    }
                    groove_midi::MidiInterfaceEvent::InputPortSelected(port) => {
                        if let Ok(mut selected_input) = selected_input.lock() {
                            *selected_input = port;
                        }
                    }
                    groove_midi::MidiInterfaceEvent::OutputPorts(ports) => {
                        if let Ok(mut outputs) = outputs.lock() {
                            *outputs = ports.clone();
                        }
                    }
                    groove_midi::MidiInterfaceEvent::OutputPortSelected(port) => {
                        if let Ok(mut selected_output) = selected_output.lock() {
                            *selected_output = port;
                        }
                    }
                    #[allow(unused_variables)]
                    groove_midi::MidiInterfaceEvent::Midi(channel, message) => {
                        if let Ok(mut last_input_instant) = last_input_instant.lock() {
                            *last_input_instant = Instant::now();
                        }
                        let _ = app_sender.send(Message::Midi(channel, message));
                    }
                    groove_midi::MidiInterfaceEvent::Quit => break,
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
                            }
                        }
                    });
                }
                ui.end_row();

                ui.label(if input_was_recent { "⬅" } else { " " });
                ui.label(if output_was_recent { "➡" } else { " " });
            });
    }
    // fn show(&mut self, ctx: &egui::Context) {
    // }
}
