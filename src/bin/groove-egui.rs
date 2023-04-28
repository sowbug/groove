// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! The egui app is an [egui](https://github.com/emilk/egui)-based DAW.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crossbeam_channel::{Receiver, Sender};
use eframe::egui::{self, CollapsingHeader, ComboBox, DragValue, RichText, Ui};
use groove_audio::{AudioInterfaceEvent, AudioInterfaceInput, AudioQueue, AudioStreamService};
use groove_core::{
    time::ClockNano,
    traits::{Performs, Resets, Shows},
    ParameterType, StereoSample, SAMPLE_BUFFER_SIZE,
};
use groove_midi::{
    MidiInterfaceEvent, MidiInterfaceInput, MidiInterfaceService, MidiPortDescriptor,
};
use groove_orchestration::Orchestrator;
use groove_settings::SongSettings;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
    time::Instant,
};

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1920.0, 1080.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Audio Prototype (egui)",
        options,
        Box::new(|_cc| Box::<GrooveApp>::default()),
    )
}

struct GrooveApp {
    orchestrator: Arc<Mutex<Orchestrator>>,
    bpm: ParameterType,
    sample_rate: Arc<Mutex<usize>>,

    #[allow(dead_code)]
    audio_stream_sender: Sender<AudioInterfaceInput>,

    control_bar: ControlBar,
    midi_panel: MidiPanel,

    tree: Tree,
}
impl Default for GrooveApp {
    fn default() -> Self {
        let clock_settings = ClockNano::default();

        let orchestrator = Arc::new(Mutex::new(Orchestrator::new_with(clock_settings)));
        let audio_stream_service = AudioStreamService::default();
        let audio_stream_sender = audio_stream_service.sender().clone();

        const SAMPLE_RATE: usize = 44100;
        let sample_rate = Arc::new(Mutex::new(SAMPLE_RATE));

        Self::start_audio_stream(
            Arc::clone(&orchestrator),
            audio_stream_service,
            Arc::clone(&sample_rate),
        );

        Self {
            bpm: Default::default(),
            orchestrator,

            sample_rate,
            audio_stream_sender,
            control_bar: ControlBar::default(),
            midi_panel: MidiPanel::new_with(),
            tree: Tree::demo(),
        }
    }
}

impl eframe::App for GrooveApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Ok(o) = self.orchestrator.lock() {
            self.bpm = o.bpm();
        }
        let top = egui::TopBottomPanel::top("control-bar");
        let bottom = egui::TopBottomPanel::bottom("orchestrator");
        let left = egui::SidePanel::left("left-sidebar");
        let center = egui::CentralPanel::default();

        top.show(ctx, |ui| {
            if let Ok(mut o) = self.orchestrator.lock() {
                self.control_bar.show(ui, &mut o);
            }
        });
        bottom.show(ctx, |ui| {
            if let Ok(o) = self.orchestrator.lock() {
                ui.label(format!("clock: {:?}", o.clock()));
            }
            if ui.button("load").clicked() {
                self.handle_load();
            }
        });
        left.show(ctx, |ui| {
            CollapsingHeader::new("File browser")
                .default_open(true)
                .show(ui, |ui| self.tree.ui(ui));
        });
        center.show(ctx, |ui| {
            if let Ok(mut o) = self.orchestrator.lock() {
                o.show(ui);
                self.midi_panel.show(ui);
            }
        });

        // TODO: this is how to keep redrawing when the system doesn't otherwise
        // know that a repaint is needed. This is fine for now, but it's
        // expensive, and we should be smarter about it.
        ctx.request_repaint();
    }
}
impl GrooveApp {
    fn start_audio_stream(
        orchestrator: Arc<Mutex<Orchestrator>>,
        audio_stream_service: AudioStreamService,
        sample_rate_clone: Arc<Mutex<usize>>,
    ) {
        std::thread::spawn(move || {
            let orchestrator = orchestrator;
            let mut queue_opt = None;
            loop {
                if let Ok(event) = audio_stream_service.receiver().recv() {
                    match event {
                        AudioInterfaceEvent::Reset(sample_rate, queue) => {
                            if let Ok(mut sr) = sample_rate_clone.lock() {
                                *sr = sample_rate;
                            }
                            if let Ok(mut o) = orchestrator.lock() {
                                o.reset(sample_rate);
                            }
                            queue_opt = Some(queue);
                        }
                        AudioInterfaceEvent::NeedsAudio(_when, count) => {
                            if let Some(queue) = queue_opt.as_ref() {
                                if let Ok(o) = orchestrator.lock() {
                                    Self::generate_audio(o, queue, (count / 64) as u8);
                                }
                            }
                        }
                        AudioInterfaceEvent::Quit => todo!(),
                    }
                }
            }
        });
    }

    fn generate_audio(
        mut orchestrator: MutexGuard<Orchestrator>,
        queue: &AudioQueue,
        buffer_count: u8,
    ) {
        let mut samples = [StereoSample::SILENCE; SAMPLE_BUFFER_SIZE];
        for _ in 0..buffer_count {
            let (response, ticks_completed) = orchestrator.tick(&mut samples);
            if ticks_completed < samples.len() {
                // self.stop_playback();
                // self.reached_end_of_playback = true;
            }

            for sample in samples {
                let _ = queue.push(sample);
            }

            match response.0 {
                groove_orchestration::messages::Internal::None => {}
                groove_orchestration::messages::Internal::Single(_event) => {
                    //                    self.handle_groove_event(event);
                }
                groove_orchestration::messages::Internal::Batch(events) => {
                    for _event in events {
                        //                      self.handle_groove_event(event)
                    }
                }
            }
        }
    }

    fn handle_load(&mut self) {
        let filename = "/home/miket/src/groove/projects/demos/controllers/stereo-automation.yaml";
        match SongSettings::new_from_yaml_file(filename) {
            Ok(s) => {
                let pb = PathBuf::from("/home/miket/src/groove/assets");
                match s.instantiate(&pb, false) {
                    Ok(instance) => {
                        if let Ok(mut o) = self.orchestrator.lock() {
                            if let Ok(sample_rate) = self.sample_rate.lock() {
                                *o = instance;
                                self.bpm = o.bpm();
                                o.reset(*sample_rate);
                            }
                        }
                    }
                    Err(err) => eprintln!("instantiate: {}", err),
                }
            }
            Err(err) => eprintln!("new_from_yaml: {}", err),
        }
    }
}

#[derive(Debug, Default)]
struct ControlBar {}
impl ControlBar {
    fn show(&self, ui: &mut egui::Ui, orchestrator: &mut Orchestrator) {
        ui.horizontal(|ui| {
            let mut bpm = orchestrator.bpm();
            ui.label("BPM");
            if ui.add(DragValue::new(&mut bpm).speed(0.1)).changed() {
                orchestrator.set_bpm(bpm);
            }
            if ui.button("start over").clicked() {
                orchestrator.skip_to_start();
            }
            if ui.button("play").clicked() {
                orchestrator.play();
            }
            if ui.button("pause").clicked() {
                orchestrator.stop();
            }

            let clock = orchestrator.clock();
            let minutes: u8 = (clock.seconds() / 60.0).floor() as u8;
            let seconds = clock.seconds() as usize % 60;
            let thousandths = (clock.seconds().fract() * 1000.0) as u16;
            ui.label(format!("{minutes:03}:{seconds:02}:{thousandths:03}"));
        });
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Action {
    Keep,
    Delete,
}

#[derive(Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct Tree(Vec<Tree>);

impl Tree {
    pub fn demo() -> Self {
        Self(vec![
            Tree(vec![Tree::default(); 4]),
            Tree(vec![Tree(vec![Tree::default(); 2]); 3]),
        ])
    }

    pub fn ui(&mut self, ui: &mut Ui) -> Action {
        self.ui_impl(ui, 0, "root")
    }
}

impl Tree {
    fn ui_impl(&mut self, ui: &mut Ui, depth: usize, name: &str) -> Action {
        CollapsingHeader::new(name)
            .default_open(depth < 1)
            .show(ui, |ui| self.children_ui(ui, depth))
            .body_returned
            .unwrap_or(Action::Keep)
    }

    fn children_ui(&mut self, ui: &mut Ui, depth: usize) -> Action {
        if depth > 0
            && ui
                .button(RichText::new("delete").color(ui.visuals().warn_fg_color))
                .clicked()
        {
            return Action::Delete;
        }

        self.0 = std::mem::take(self)
            .0
            .into_iter()
            .enumerate()
            .filter_map(|(i, mut tree)| {
                if tree.ui_impl(ui, depth + 1, &format!("child #{}", i)) == Action::Keep {
                    Some(tree)
                } else {
                    None
                }
            })
            .collect();

        if ui.button("+").clicked() {
            self.0.push(Tree::default());
        }

        Action::Keep
    }
}

#[derive(Debug)]
struct MidiPanel {
    sender: Sender<MidiInterfaceInput>,

    inputs: Arc<Mutex<Vec<MidiPortDescriptor>>>,
    selected_input: Arc<Mutex<Option<MidiPortDescriptor>>>,
    outputs: Arc<Mutex<Vec<MidiPortDescriptor>>>,
    selected_output: Arc<Mutex<Option<MidiPortDescriptor>>>,

    last_input_instant: Arc<Mutex<Instant>>,
    last_output_instant: Instant,
}
impl MidiPanel {
    pub(crate) fn new_with() -> Self {
        let midi_interface_service = MidiInterfaceService::default();
        let sender = midi_interface_service.sender().clone();

        let r = Self {
            sender,

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

    pub(crate) fn send(&mut self, input: MidiInterfaceInput) {
        if let MidiInterfaceInput::Midi(..) = input {
            self.last_output_instant = Instant::now();
        }

        let _ = self.sender.send(input);
    }

    fn start_midi_interface(&self, receiver: Receiver<MidiInterfaceEvent>) {
        let sender = self.sender.clone();
        let inputs = Arc::clone(&self.inputs);
        let selected_input = Arc::clone(&self.selected_input);
        let outputs = Arc::clone(&self.outputs);
        let selected_output = Arc::clone(&self.selected_output);
        let last_input_instant = Arc::clone(&self.last_input_instant);
        std::thread::spawn(move || loop {
            if let Ok(event) = receiver.recv() {
                match event {
                    groove_midi::MidiInterfaceEvent::Ready(_) => todo!(),
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
                    groove_midi::MidiInterfaceEvent::Midi(channel, message) => {
                        if let Ok(mut last_input_instant) = last_input_instant.lock() {
                            *last_input_instant = Instant::now();
                        }
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
                    (0, "None")
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
                    (0, "None")
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
    }
}
