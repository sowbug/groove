// Copyright (c) 2023 Mike Tsao. All rights reserved.

use anyhow::{anyhow, Result};
use atomic_counter::{AtomicCounter, RelaxedCounter};
use crossbeam_channel::{Receiver, Select, Sender};
use derive_more::Display;
use eframe::{
    egui::{
        self, Button, Context, CursorIcon, FontData, FontDefinitions, Frame, Id as EguiId,
        InnerResponse, LayerId, Layout, Margin, Order, Response, ScrollArea, Sense, TextStyle, Ui,
    },
    emath::{self, Align, Align2},
    epaint::{
        self, pos2, vec2, Color32, FontFamily, FontId, Pos2, Rect, RectShape, Rounding, Shape,
        Stroke, Vec2,
    },
    CreationContext,
};
use egui_toast::{Toast, ToastOptions, Toasts};
use groove::{
    app_version,
    egui_widgets::{
        AudioPanel2, AudioPanelEvent, ControlPanel, ControlPanelAction, MidiPanel, MidiPanelEvent,
        NeedsAudioFn,
    },
};
use groove_audio::AudioQueue;
use groove_core::{
    midi::{MidiChannel, MidiMessage},
    time::{MusicalTime, SampleRate, Tempo, TimeSignature},
    traits::{
        gui::Shows, Configurable, Generates, HandlesMidi, IsController, IsEffect, IsInstrument,
        Ticks,
    },
    StereoSample, Uid,
};
use groove_entities::{
    controllers::{Arpeggiator, ArpeggiatorParams},
    effects::{BiQuadFilterLowPass24db, BiQuadFilterLowPass24dbParams, Reverb, ReverbParams},
    instruments::{WelshSynth, WelshSynthParams},
    EntityMessage,
};
use groove_toys::{ToyInstrument, ToyInstrumentParams, ToySynth, ToySynthParams};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use strum_macros::EnumIter;

// Rules for communication among app components
//
// - If it's in the same thread, don't be fancy. Example: the app owns the
//   control bar, and the control bar always runs in the UI thread. The app
//   should talk directly to the control bar (update BPM or transport), and the
//   control bar can pass back an enum saying what happened (play button was
//   pressed).
// - If it's updated rarely but displayed frequently, the struct should push it
//   to the app, and the app should cache it. Example: BPM is displayed in the
//   control bar, so we're certain to need it on every redraw, but it rarely
//   changes (unless it's automated). Orchestrator should define a channel
//   message, and the app should handle it when it's received.
// - If it's updated more often than the UI framerate, let the UI pull it
//   directly from the struct. Example: an LFO signal or a real-time spectrum
//   analysis. These should be APIs directly on the struct, and we'll leave it
//   up to the app to lock the struct and get what it needs.

#[typetag::serde(tag = "type")]
trait NewIsController: IsController<Message = EntityMessage> {}

#[typetag::serde(tag = "type")]
trait NewIsInstrument: IsInstrument {}

#[typetag::serde(tag = "type")]
trait NewIsEffect: IsEffect {}

#[derive(Clone, Debug)]
enum MiniOrchestratorInput {
    Midi(MidiChannel, MidiMessage),
    ProjectLoad(PathBuf),
    ProjectNew,
    ProjectPlay,
    ProjectSave(PathBuf),
    ProjectStop,
    TrackDelete(usize),
    TrackDuplicate(usize),
    TrackNew,

    /// Request that the orchestrator service quit.
    Quit,
}

#[derive(Debug)]
enum MiniOrchestratorEvent {
    Tempo(Tempo),

    /// A new, empty project was created.
    New,

    Loaded(PathBuf, Option<String>),
    LoadError(PathBuf, anyhow::Error),

    Saved(PathBuf),
    SaveError(PathBuf, anyhow::Error),

    /// Acknowledge request to quit.
    Quit,
}

#[derive(Debug)]
struct ChannelPair<T> {
    sender: Sender<T>,
    receiver: Receiver<T>,
}
impl<T> Default for ChannelPair<T> {
    fn default() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        Self { sender, receiver }
    }
}

struct OrchestratorPanel {
    #[allow(dead_code)]
    factory: Arc<EntityFactory>,
    #[allow(dead_code)]
    drag_drop_manager: Arc<Mutex<DragDropManager>>,
    orchestrator: Arc<Mutex<MiniOrchestrator>>,
    input_channel_pair: ChannelPair<MiniOrchestratorInput>,
    event_channel_pair: ChannelPair<MiniOrchestratorEvent>,
}
impl OrchestratorPanel {
    fn new_with(
        factory: Arc<EntityFactory>,
        drag_drop_manager: Arc<Mutex<DragDropManager>>,
    ) -> Self {
        let mut r = Self {
            factory,
            drag_drop_manager,
            orchestrator: Default::default(),
            input_channel_pair: Default::default(),
            event_channel_pair: Default::default(),
        };
        r.start_thread();
        r
    }

    fn start_thread(&mut self) {
        let receiver = self.input_channel_pair.receiver.clone();
        let sender = self.event_channel_pair.sender.clone();
        self.introduce();
        let orchestrator = Arc::clone(&self.orchestrator);
        std::thread::spawn(move || loop {
            match receiver.recv() {
                Ok(input) => match input {
                    MiniOrchestratorInput::Midi(channel, message) => {
                        Self::handle_input_midi(&orchestrator, channel, message);
                    }
                    MiniOrchestratorInput::ProjectPlay => eprintln!("Play"),
                    MiniOrchestratorInput::ProjectStop => eprintln!("Stop"),
                    MiniOrchestratorInput::ProjectNew => {
                        let mut mo = MiniOrchestrator::default();
                        if let Ok(mut o) = orchestrator.lock() {
                            o.prepare_successor(&mut mo);
                            *o = mo;
                            let _ = sender.send(MiniOrchestratorEvent::New);
                        }
                    }
                    MiniOrchestratorInput::ProjectLoad(path) => {
                        match Self::handle_input_load(&path) {
                            Ok(mut mo) => {
                                if let Ok(mut o) = orchestrator.lock() {
                                    o.prepare_successor(&mut mo);
                                    *o = mo;
                                    let _ = sender.send(MiniOrchestratorEvent::Loaded(
                                        path,
                                        o.title().cloned(),
                                    ));
                                }
                            }
                            Err(err) => {
                                let _ = sender.send(MiniOrchestratorEvent::LoadError(path, err));
                            }
                        }
                        {}
                    }
                    MiniOrchestratorInput::ProjectSave(path) => {
                        match Self::handle_input_save(&orchestrator, &path) {
                            Ok(_) => {
                                let _ = sender.send(MiniOrchestratorEvent::Saved(path));
                            }
                            Err(err) => {
                                let _ = sender.send(MiniOrchestratorEvent::SaveError(path, err));
                            }
                        }
                    }
                    MiniOrchestratorInput::Quit => {
                        let _ = sender.send(MiniOrchestratorEvent::Quit);
                        break;
                    }
                    MiniOrchestratorInput::TrackNew => {
                        if let Ok(mut o) = orchestrator.lock() {
                            o.new_track();
                        }
                    }
                    MiniOrchestratorInput::TrackDelete(index) => {
                        if let Ok(mut o) = orchestrator.lock() {
                            o.delete_track(index);
                        }
                    }
                    MiniOrchestratorInput::TrackDuplicate(_index) => {
                        todo!();
                    }
                },
                Err(err) => {
                    eprintln!(
                        "unexpected failure of MiniOrchestratorInput channel: {:?}",
                        err
                    );
                    break;
                }
            }
        });
    }

    // Send any important initial messages after creation.
    fn introduce(&self) {
        if let Ok(o) = self.orchestrator.lock() {
            self.broadcast_tempo(o.tempo());
        }
    }

    pub fn selected_track(&self) -> Option<usize> {
        if let Ok(o) = self.orchestrator.lock() {
            o.selected_track
        } else {
            None
        }
    }

    fn broadcast_tempo(&self, tempo: Tempo) {
        self.broadcast(MiniOrchestratorEvent::Tempo(tempo));
    }

    fn broadcast(&self, event: MiniOrchestratorEvent) {
        let _ = self.event_channel_pair.sender.send(event);
    }

    fn sender(&self) -> &Sender<MiniOrchestratorInput> {
        &self.input_channel_pair.sender
    }

    fn receiver(&self) -> &Receiver<MiniOrchestratorEvent> {
        &self.event_channel_pair.receiver
    }

    fn orchestrator(&self) -> &Arc<Mutex<MiniOrchestrator>> {
        &self.orchestrator
    }

    fn handle_input_midi(
        orchestrator: &Arc<Mutex<MiniOrchestrator>>,
        channel: MidiChannel,
        message: MidiMessage,
    ) {
        if let Ok(mut o) = orchestrator.lock() {
            o.handle_midi(channel, message);
        }
    }

    fn handle_input_load(path: &PathBuf) -> Result<MiniOrchestrator> {
        match std::fs::read_to_string(path) {
            Ok(project_string) => match serde_json::from_str(&project_string) {
                Ok(mo) => {
                    return anyhow::Ok(mo);
                }
                Err(err) => {
                    return Err(anyhow!("Error while parsing: {}", err));
                }
            },
            Err(err) => {
                return Err(anyhow!("Error while reading: {}", err));
            }
        }
    }

    fn handle_input_save(
        orchestrator: &Arc<Mutex<MiniOrchestrator>>,
        path: &PathBuf,
    ) -> Result<()> {
        if let Ok(o) = orchestrator.lock() {
            let o: &MiniOrchestrator = &o;
            match serde_json::to_string_pretty(o)
                .map_err(|_| anyhow::format_err!("Unable to serialize prefs JSON"))
            {
                Ok(json) => match std::fs::write(path, json) {
                    Ok(_) => Ok(()),
                    Err(err) => Err(anyhow!("While writing project: {}", err)),
                },
                Err(err) => Err(anyhow!("While serializing project: {}", err)),
            }
        } else {
            Err(anyhow!("Couldn't get lock"))
        }
    }

    fn send_to_service(&self, input: MiniOrchestratorInput) {
        match self.sender().send(input) {
            Ok(_) => {}
            Err(err) => eprintln!("sending MiniOrchestratorInput failed with {:?}", err),
        }
    }

    fn exit(&self) {
        eprintln!("MiniOrchestratorInput::Quit");
        self.send_to_service(MiniOrchestratorInput::Quit);
    }
}
impl Shows for OrchestratorPanel {
    fn show(&mut self, ui: &mut Ui) {
        if let Ok(mut o) = self.orchestrator.lock() {
            o.show_with(ui, &self.factory);
        }
    }
}

#[derive(Debug, EnumIter)]
enum EntityType {
    None,
    Controller,
    Effect,
    Instrument,
}

#[derive(Serialize, Deserialize, Debug)]
struct Track {
    controllers: Vec<Box<dyn NewIsController>>,
    instruments: Vec<Box<dyn NewIsInstrument>>,
    effects: Vec<Box<dyn NewIsEffect>>,
}
impl Default for Track {
    fn default() -> Self {
        Self {
            controllers: Default::default(),
            instruments: Default::default(),
            effects: Default::default(),
        }
    }
}
impl Track {
    // TODO: this is getting cumbersome! Think about that uber-trait!

    #[allow(dead_code)]
    fn controller(&self, index: usize) -> Option<&Box<dyn NewIsController>> {
        self.controllers.get(index)
    }

    #[allow(dead_code)]
    fn controller_mut(&mut self, index: usize) -> Option<&mut Box<dyn NewIsController>> {
        self.controllers.get_mut(index)
    }

    #[allow(dead_code)]
    fn effect(&self, index: usize) -> Option<&Box<dyn NewIsEffect>> {
        self.effects.get(index)
    }

    #[allow(dead_code)]
    fn effect_mut(&mut self, index: usize) -> Option<&mut Box<dyn NewIsEffect>> {
        self.effects.get_mut(index)
    }

    #[allow(dead_code)]
    fn instrument(&self, index: usize) -> Option<&Box<dyn NewIsInstrument>> {
        self.instruments.get(index)
    }

    #[allow(dead_code)]
    fn instrument_mut(&mut self, index: usize) -> Option<&mut Box<dyn NewIsInstrument>> {
        self.instruments.get_mut(index)
    }

    fn append_controller(&mut self, e: Box<dyn NewIsController>) {
        self.controllers.push(e);
    }

    fn append_effect(&mut self, e: Box<dyn NewIsEffect>) {
        self.effects.push(e);
    }

    fn append_instrument(&mut self, e: Box<dyn NewIsInstrument>) {
        self.instruments.push(e);
    }

    fn remove_controller(&mut self, index: usize) -> Option<Box<dyn NewIsController>> {
        Some(self.controllers.remove(index))
    }

    fn remove_effect(&mut self, index: usize) -> Option<Box<dyn NewIsEffect>> {
        Some(self.effects.remove(index))
    }

    fn remove_instrument(&mut self, index: usize) -> Option<Box<dyn NewIsInstrument>> {
        Some(self.instruments.remove(index))
    }

    fn insert_controller(&mut self, index: usize, e: Box<dyn NewIsController>) -> Result<()> {
        if index > self.controllers.len() {
            return Err(anyhow!(
                "can't insert at {} in {}-length vec",
                index,
                self.controllers.len()
            ));
        }
        self.controllers.insert(index, e);
        Ok(())
    }

    fn insert_effect(&mut self, index: usize, e: Box<dyn NewIsEffect>) -> Result<()> {
        if index > self.effects.len() {
            return Err(anyhow!(
                "can't insert at {} in {}-length vec",
                index,
                self.effects.len()
            ));
        }
        self.effects.insert(index, e);
        Ok(())
    }

    fn insert_instrument(&mut self, index: usize, e: Box<dyn NewIsInstrument>) -> Result<()> {
        if index > self.instruments.len() {
            return Err(anyhow!(
                "can't insert at {} in {}-length vec",
                index,
                self.instruments.len()
            ));
        }
        self.instruments.insert(index, e);
        Ok(())
    }

    fn shift_controller_left(&mut self, index: usize) -> Result<()> {
        if index >= self.controllers.len() {
            return Err(anyhow!("Index {index} out of bounds."));
        }
        if index == 0 {
            return Err(anyhow!("Can't move leftmost item farther left."));
        }
        let element = self.controllers.remove(index);
        self.insert_controller(index - 1, element)
    }
    fn shift_controller_right(&mut self, index: usize) -> Result<()> {
        if index >= self.controllers.len() {
            return Err(anyhow!("Index {index} out of bounds."));
        }
        if index == self.controllers.len() - 1 {
            return Err(anyhow!("Can't move rightmost item farther right."));
        }
        let element = self.controllers.remove(index);
        self.insert_controller(index + 1, element)
    }

    fn shift_effect_left(&mut self, index: usize) -> Result<()> {
        if index >= self.effects.len() {
            return Err(anyhow!("Index {index} out of bounds."));
        }
        if index == 0 {
            return Err(anyhow!("Can't move leftmost item farther left."));
        }
        let element = self.effects.remove(index);
        self.insert_effect(index - 1, element)
    }
    fn shift_effect_right(&mut self, index: usize) -> Result<()> {
        if index >= self.effects.len() {
            return Err(anyhow!("Index {index} out of bounds."));
        }
        if index == self.effects.len() - 1 {
            return Err(anyhow!("Can't move rightmost item farther right."));
        }
        let element = self.effects.remove(index);
        self.insert_effect(index + 1, element)
    }

    fn shift_instrument_left(&mut self, index: usize) -> Result<()> {
        if index >= self.instruments.len() {
            return Err(anyhow!("Index {index} out of bounds."));
        }
        if index == 0 {
            return Err(anyhow!("Can't move leftmost item farther left."));
        }
        let element = self.instruments.remove(index);
        self.insert_instrument(index - 1, element)
    }
    fn shift_instrument_right(&mut self, index: usize) -> Result<()> {
        if index >= self.instruments.len() {
            return Err(anyhow!("Index {index} out of bounds."));
        }
        if index == self.instruments.len() - 1 {
            return Err(anyhow!("Can't move rightmost item farther right."));
        }
        let element = self.instruments.remove(index);
        self.insert_instrument(index + 1, element)
    }

    fn button_states(index: usize, len: usize) -> (bool, bool) {
        let left = index != 0;
        let right = len > 1 && index != len - 1;
        (left, right)
    }

    fn show_arrangement(&mut self, ui: &mut Ui, is_selected: bool) -> Response {
        ui.ctx().request_repaint();
        let color = if ui.visuals().dark_mode {
            Color32::from_additive_luminance(196)
        } else {
            Color32::from_black_alpha(240)
        };

        let (response, painter) =
            ui.allocate_painter(vec2(ui.available_width(), 64.0), Sense::click());

        let time = ui.input(|i| i.time);
        let to_screen = emath::RectTransform::from_to(
            Rect::from_x_y_ranges(0.0..=1.0, -1.0..=1.0),
            response.rect,
        );

        let mut shapes = vec![];
        if is_selected {
            shapes.push(Shape::Rect(RectShape::filled(
                painter.clip_rect(),
                Rounding::none(),
                Color32::DARK_BLUE,
            )));
        }

        for &mode in &[2, 3, 5] {
            let mode = mode as f64;
            let n = 120;
            let speed = 1.5;

            let points: Vec<Pos2> = (0..=n)
                .map(|i| {
                    let t = i as f64 / (n as f64);
                    let amp = (time * speed * mode).sin() / mode;
                    let y = amp * (t * std::f64::consts::TAU / 2.0 * mode).sin();
                    to_screen * pos2(t as f32, y as f32)
                })
                .collect();

            let thickness = 10.0 / mode as f32;
            shapes.push(Shape::line(points, Stroke::new(thickness, color)));
        }

        shapes.push(Shape::LineSegment {
            points: [to_screen * pos2(0.0, 1.0), to_screen * pos2(1.0, 1.0)],
            stroke: Stroke { width: 1.0, color },
        });

        painter.extend(shapes);

        response
    }

    // TODO: ordering should be controllers, instruments, then effects. Within
    // those groups, the user can reorder as desired (but instrument order
    // doesn't matter because they're all simultaneous)
    fn show_detail(&mut self, ui: &mut Ui, factory: &EntityFactory, _track_index: usize) {
        let style = ui.visuals().widgets.inactive;

        ui.with_layout(
            egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
            |ui| {
                let desired_size = Vec2::new(ui.available_width(), 256.0 - style.fg_stroke.width);
                ui.set_min_size(desired_size);
                ui.set_max_size(desired_size);

                ui.horizontal(|ui| {
                    ui.menu_button("+", |ui| {
                        ui.menu_button("Controllers", |ui| {
                            factory.controller_keys().for_each(|k| {
                                if ui.button(k.to_string()).clicked() {
                                    if let Some(e) = factory.new_controller(k) {
                                        self.append_controller(e);
                                    }
                                    ui.close_menu();
                                }
                            });
                        });
                        ui.menu_button("Instruments", |ui| {
                            factory.instrument_keys().for_each(|k| {
                                if ui.button(k.to_string()).clicked() {
                                    if let Some(e) = factory.new_instrument(k) {
                                        self.append_instrument(e);
                                    }
                                    ui.close_menu();
                                }
                            });
                        });
                        ui.menu_button("Effects", |ui| {
                            factory.effect_keys().for_each(|k| {
                                if ui.button(k.to_string()).clicked() {
                                    if let Some(e) = factory.new_effect(k) {
                                        self.append_effect(e);
                                    }
                                    ui.close_menu();
                                }
                            });
                        });
                    });
                });
                ui.add(egui::Separator::default().grow(8.0));

                ui.horizontal_centered(|ui| {
                    let desired_size = Vec2::new(96.0, ui.available_height());

                    let mut action = None;

                    // controller
                    let len = self.controllers.len();
                    for (index, e) in self.controllers.iter_mut().enumerate() {
                        ui.allocate_ui(desired_size, |ui| {
                            let (show_left, show_right) = Self::button_states(index, len);
                            if let Some(a) = Self::add_track_element(
                                ui,
                                index,
                                EntityType::Controller,
                                show_left,
                                show_right,
                                true,
                                |ui| {
                                    e.show(ui);
                                },
                            ) {
                                action = Some(a);
                            };
                        });
                    }

                    // instrument
                    for (index, e) in self.instruments.iter_mut().enumerate() {
                        ui.allocate_ui(desired_size, |ui| {
                            // Instrument order in a track doesn't matter, so left/right are always off.
                            if let Some(a) = Self::add_track_element(
                                ui,
                                index,
                                EntityType::Instrument,
                                false,
                                false,
                                true,
                                |ui| {
                                    ui.set_min_size(desired_size);
                                    ui.set_max_size(desired_size);
                                    e.show(ui);
                                },
                            ) {
                                action = Some(a);
                            };
                        });
                    }

                    // effect
                    let len = self.effects.len();
                    for (index, e) in self.effects.iter_mut().enumerate() {
                        ui.allocate_ui(desired_size, |ui| {
                            let (show_left, show_right) = Self::button_states(index, len);
                            if let Some(a) = Self::add_track_element(
                                ui,
                                index,
                                EntityType::Effect,
                                show_left,
                                show_right,
                                true,
                                |ui| {
                                    ui.set_min_size(desired_size);
                                    ui.set_max_size(desired_size);
                                    e.show(ui);
                                },
                            ) {
                                action = Some(a);
                            };
                        });
                    }

                    // check action
                    if let Some(action) = action {
                        match action {
                            TrackElementAction::MoveControllerLeft(index) => {
                                let _ = self.shift_controller_left(index);
                            }
                            TrackElementAction::MoveControllerRight(index) => {
                                let _ = self.shift_controller_right(index);
                            }
                            TrackElementAction::RemoveController(index) => {
                                let _ = self.remove_controller(index);
                            }
                            TrackElementAction::MoveEffectLeft(index) => {
                                let _ = self.shift_effect_left(index);
                            }
                            TrackElementAction::MoveEffectRight(index) => {
                                let _ = self.shift_effect_right(index);
                            }
                            TrackElementAction::RemoveEffect(index) => {
                                let _ = self.remove_effect(index);
                            }
                            TrackElementAction::MoveInstrumentLeft(index) => {
                                let _ = self.shift_instrument_left(index);
                            }
                            TrackElementAction::MoveInstrumentRight(index) => {
                                let _ = self.shift_instrument_right(index);
                            }
                            TrackElementAction::RemoveInstrument(index) => {
                                let _ = self.remove_instrument(index);
                            }
                        }
                    }
                });
            },
        );
    }

    fn add_track_element(
        ui: &mut Ui,
        index: usize,
        entity_type: EntityType,
        show_left_button: bool,
        show_right_button: bool,
        show_delete_button: bool,
        add_contents: impl FnOnce(&mut Ui),
    ) -> Option<TrackElementAction> {
        let mut action = None;
        let style = ui.visuals().widgets.inactive;
        Frame::none()
            .stroke(style.fg_stroke)
            .inner_margin(Margin::same(2.0))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                        if show_left_button {
                            if ui.button("<").clicked() {
                                action = match entity_type {
                                    EntityType::Controller => {
                                        Some(TrackElementAction::MoveControllerLeft(index))
                                    }
                                    EntityType::Effect => {
                                        Some(TrackElementAction::MoveEffectLeft(index))
                                    }
                                    EntityType::Instrument => {
                                        Some(TrackElementAction::MoveInstrumentLeft(index))
                                    }
                                    EntityType::None => None,
                                };
                            }
                        }
                        if show_right_button {
                            if ui.button(">").clicked() {
                                action = match entity_type {
                                    EntityType::Controller => {
                                        Some(TrackElementAction::MoveControllerRight(index))
                                    }
                                    EntityType::Effect => {
                                        Some(TrackElementAction::MoveEffectRight(index))
                                    }
                                    EntityType::Instrument => {
                                        Some(TrackElementAction::MoveInstrumentRight(index))
                                    }
                                    EntityType::None => None,
                                };
                            }
                        }
                        if show_delete_button {
                            if ui.button("x").clicked() {
                                action = match entity_type {
                                    EntityType::Controller => {
                                        Some(TrackElementAction::RemoveController(index))
                                    }
                                    EntityType::Effect => {
                                        Some(TrackElementAction::RemoveEffect(index))
                                    }
                                    EntityType::Instrument => {
                                        Some(TrackElementAction::RemoveInstrument(index))
                                    }
                                    EntityType::None => None,
                                };
                            }
                        }
                    });
                    ui.vertical(|ui| {
                        add_contents(ui);
                    });
                });
            });
        action
    }
}
impl Generates<StereoSample> for Track {
    fn value(&self) -> StereoSample {
        StereoSample::SILENCE
    }

    fn batch_values(&mut self, values: &mut [StereoSample]) {
        for e in self.instruments.iter_mut() {
            e.batch_values(values);
        }
    }
}
impl Ticks for Track {
    fn tick(&mut self, _tick_count: usize) {
        todo!()
    }
}
impl Configurable for Track {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        for e in self.controllers.iter_mut() {
            e.update_sample_rate(sample_rate);
        }
        for e in self.effects.iter_mut() {
            e.update_sample_rate(sample_rate);
        }
        for e in self.instruments.iter_mut() {
            e.update_sample_rate(sample_rate);
        }
    }

    fn update_tempo(&mut self, _tempo: Tempo) {
        todo!()
    }

    fn update_time_signature(&mut self, _time_signature: TimeSignature) {
        todo!()
    }
}
impl HandlesMidi for Track {
    fn handle_midi_message(
        &mut self,
        message: &MidiMessage,
        messages_fn: &mut dyn FnMut(MidiChannel, MidiMessage),
    ) {
        for e in self.controllers.iter_mut() {
            e.handle_midi_message(&message, messages_fn);
        }
        for e in self.instruments.iter_mut() {
            e.handle_midi_message(&message, messages_fn);
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct MiniOrchestrator {
    title: Option<String>,
    time_signature: TimeSignature,
    tempo: Tempo,

    tracks: Vec<Track>,

    // TODO: This is wrong, but simple and fast. We should be allowing for
    // multiple item selection.
    selected_track: Option<usize>,

    //////////////////////////////////////////////////////
    // Nothing below this comment should be serialized. //
    //////////////////////////////////////////////////////
    //
    #[serde(skip)]
    sample_rate: SampleRate,

    #[serde(skip)]
    #[allow(dead_code)]
    frames: usize,

    #[serde(skip)]
    #[allow(dead_code)]
    musical_time: MusicalTime,
}
impl Default for MiniOrchestrator {
    fn default() -> Self {
        Self {
            title: None,
            time_signature: Default::default(),
            tempo: Default::default(),

            tracks: vec![Default::default(), Default::default(), Default::default()],

            sample_rate: Default::default(),
            frames: Default::default(),
            musical_time: Default::default(),
            selected_track: Default::default(),
        }
    }
}
impl MiniOrchestrator {
    #[allow(dead_code)]
    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    fn set_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        for track in self.tracks.iter_mut() {
            track.update_sample_rate(sample_rate);
        }
    }

    fn tempo(&self) -> Tempo {
        self.tempo
    }

    #[allow(dead_code)]
    fn set_tempo(&mut self, tempo: Tempo) {
        self.tempo = tempo;
    }

    // Fills in the given sample buffer with something simple and audible.
    #[allow(dead_code)]
    fn debug_sample_buffer(&mut self, samples: &mut [StereoSample]) {
        let len = samples.len() as f64;
        for (i, s) in samples.iter_mut().enumerate() {
            *s = StereoSample::from(i as f64 / len);
        }
    }

    fn provide_audio(&mut self, queue: &AudioQueue, samples_requested: usize) {
        const SAMPLE_BUFFER_SIZE: usize = 64;
        let mut samples = [StereoSample::SILENCE; SAMPLE_BUFFER_SIZE];

        // Round up
        let buffers_requested = (samples_requested + SAMPLE_BUFFER_SIZE - 1) / SAMPLE_BUFFER_SIZE;
        for _ in 0..buffers_requested {
            self.batch_values(&mut samples);
            for sample in samples {
                let _ = queue.push(sample);
            }
        }
    }

    // TODO: we're ignoring channels at the moment.
    #[allow(unused_variables)]
    fn handle_midi(&mut self, channel: MidiChannel, message: MidiMessage) {
        for track in self.tracks.iter_mut() {
            track.handle_midi_message(&message, &mut |channel, message| {
                eprintln!("TODO discarding {}/{:?}", channel, message)
            });
        }
    }

    /// After loading a new Self from disk, we want to copy all the appropriate
    /// ephemeral state from this one to the next one.
    fn prepare_successor(&self, new: &mut MiniOrchestrator) {
        new.set_sample_rate(self.sample_rate());
    }

    #[allow(dead_code)]
    fn add_controller(&mut self, track_index: usize, mut e: Box<dyn NewIsController>) -> Uid {
        e.update_sample_rate(self.sample_rate);
        let uid = e.uid();
        self.tracks[track_index].append_controller(e);
        uid
    }

    #[allow(dead_code)]
    fn add_effect(&mut self, track_index: usize, mut e: Box<dyn NewIsEffect>) -> Uid {
        e.update_sample_rate(self.sample_rate);
        let uid = e.uid();
        self.tracks[track_index].append_effect(e);
        uid
    }

    #[allow(dead_code)]
    fn add_instrument(
        &mut self,
        track_index: usize,
        mut e: Box<dyn NewIsInstrument>,
    ) -> Result<Uid> {
        e.update_sample_rate(self.sample_rate);
        let uid = e.uid();
        self.tracks[track_index].append_instrument(e);
        Ok(uid)
    }

    #[allow(dead_code)]
    fn move_controller(
        &mut self,
        old_track_index: usize,
        old_item_index: usize,
        new_track_index: usize,
        new_item_index: usize,
    ) -> Result<()> {
        if let Some(e) = self.tracks[old_track_index].remove_controller(old_item_index) {
            self.tracks[new_track_index].insert_controller(new_item_index, e)
        } else {
            Err(anyhow!("controller not found"))
        }
    }

    #[allow(dead_code)]
    fn move_effect(
        &mut self,
        old_track_index: usize,
        old_item_index: usize,
        new_track_index: usize,
        new_item_index: usize,
    ) -> Result<()> {
        if let Some(e) = self.tracks[old_track_index].remove_effect(old_item_index) {
            self.tracks[new_track_index].insert_effect(new_item_index, e)
        } else {
            Err(anyhow!("effect not found"))
        }
    }

    #[allow(dead_code)]
    fn move_instrument(
        &mut self,
        old_track_index: usize,
        old_item_index: usize,
        new_track_index: usize,
        new_item_index: usize,
    ) -> Result<()> {
        if let Some(e) = self.tracks[old_track_index].remove_instrument(old_item_index) {
            self.tracks[new_track_index].insert_instrument(new_item_index, e)
        } else {
            Err(anyhow!("instrument not found"))
        }
    }

    fn show_tracks(&mut self, ui: &mut Ui, _factory: &EntityFactory) -> Option<TrackAction> {
        let action = None;
        for (track_index, track) in self.tracks.iter_mut().enumerate() {
            let is_selected = if let Some(selected) = self.selected_track {
                track_index == selected
            } else {
                false
            };
            if track.show_arrangement(ui, is_selected).clicked() {
                self.selected_track = Some(track_index);
            }
        }
        action
    }

    fn show_with(&mut self, ui: &mut egui::Ui, factory: &EntityFactory) {
        if let Some(action) = self.show_tracks(ui, factory) {
            self.handle_track_action(factory, action);
        }
        if let Some(selected) = self.selected_track {
            let bottom = egui::TopBottomPanel::bottom("orchestrator-bottom-panel").resizable(true);
            bottom.show_inside(ui, |ui| {
                self.tracks[selected].show_detail(ui, factory, selected);
            });
        }
    }

    fn handle_track_action(&mut self, factory: &EntityFactory, action: TrackAction) {
        match action {
            TrackAction::NewController(track, key) => {
                // TODO: will instruments ever exist outside of tracks? If not,
                // then why go through the new/add/push sequence?
                if let Some(e) = factory.new_controller(&key) {
                    self.tracks[track].append_controller(e);
                }
            }
            TrackAction::NewEffect(track, key) => {
                if let Some(e) = factory.new_effect(&key) {
                    self.tracks[track].append_effect(e);
                }
            }
            TrackAction::NewInstrument(track, key) => {
                if let Some(e) = factory.new_instrument(&key) {
                    self.tracks[track].append_instrument(e);
                }
            }
        }
    }

    fn new_track(&mut self) {
        self.tracks.push(Default::default());
    }

    fn delete_track(&mut self, index: usize) {
        self.tracks.remove(index);
        if let Some(selected) = self.selected_track {
            if index == selected {
                self.selected_track = None;
            } else if index < selected {
                // If the user can delete only the selected track, then it seems
                // like it shouldn't be able to happen. But we're checking
                // anyway in case the UI evolves.
                self.selected_track = Some(selected - 1);
            }
        }
    }

    fn title(&self) -> Option<&String> {
        self.title.as_ref()
    }

    #[allow(dead_code)]
    fn set_title(&mut self, title: Option<String>) {
        self.title = title;
    }
}
impl Generates<StereoSample> for MiniOrchestrator {
    fn value(&self) -> StereoSample {
        StereoSample::SILENCE
    }

    fn batch_values(&mut self, values: &mut [StereoSample]) {
        let len = values.len();
        let frames = 0..len;
        for track in self.tracks.iter_mut() {
            let mut track_buffer = Vec::with_capacity(len);
            track_buffer.resize(frames.end, StereoSample::default());
            track.batch_values(&mut track_buffer);
            for (i, v) in track_buffer.iter().enumerate() {
                values[i] += *v;
            }
        }
    }
}
impl Ticks for MiniOrchestrator {
    fn tick(&mut self, _tick_count: usize) {
        panic!()
    }
}
impl Configurable for MiniOrchestrator {}
impl Shows for MiniOrchestrator {
    fn show(&mut self, ui: &mut egui::Ui) {
        ui.label("not used");
    }
}

/// A globally unique identifier for a kind of thing, such as an arpeggiator
/// controller, an FM synthesizer, or a reverb effect.
#[derive(Clone, Debug, Display, Eq, Hash, PartialEq)]
struct Key(String);
impl From<&String> for Key {
    fn from(value: &String) -> Self {
        Key(value.to_string())
    }
}
impl From<&str> for Key {
    fn from(value: &str) -> Self {
        Key(value.to_string())
    }
}

type ControllerEntityFactoryFn = fn() -> Box<dyn NewIsController>;
type InstrumentEntityFactoryFn = fn() -> Box<dyn NewIsInstrument>;
type EffectEntityFactoryFn = fn() -> Box<dyn NewIsEffect>;
#[derive(Debug, Default)]
struct EntityFactory {
    next_id: RelaxedCounter,

    controllers: HashMap<Key, ControllerEntityFactoryFn>,
    instruments: HashMap<Key, InstrumentEntityFactoryFn>,
    effects: HashMap<Key, EffectEntityFactoryFn>,
    keys: HashSet<Key>,
}
impl EntityFactory {
    pub fn register_controller(&mut self, key: Key, f: ControllerEntityFactoryFn) {
        if self.keys.insert(key.clone()) {
            self.controllers.insert(key, f);
        } else {
            panic!("register_controller({}): duplicate key. Exiting.", key);
        }
    }
    pub fn new_controller(&self, key: &Key) -> Option<Box<dyn NewIsController>> {
        if let Some(f) = self.controllers.get(key) {
            let mut r = f();
            r.set_uid(Uid(self.next_id.inc()));
            Some(r)
        } else {
            None
        }
    }
    pub fn register_instrument(&mut self, key: Key, f: InstrumentEntityFactoryFn) {
        if self.keys.insert(key.clone()) {
            self.instruments.insert(key, f);
        } else {
            panic!("register_instrument({}): duplicate key. Exiting.", key);
        }
    }
    pub fn new_instrument(&self, key: &Key) -> Option<Box<dyn NewIsInstrument>> {
        if let Some(f) = self.instruments.get(key) {
            let mut r = f();
            r.set_uid(Uid(self.next_id.inc()));
            Some(r)
        } else {
            None
        }
    }
    pub fn register_effect(&mut self, key: Key, f: EffectEntityFactoryFn) {
        if self.keys.insert(key.clone()) {
            self.effects.insert(key, f);
        } else {
            panic!("register_effect({}): duplicate key. Exiting.", key);
        }
    }
    pub fn new_effect(&self, key: &Key) -> Option<Box<dyn NewIsEffect>> {
        if let Some(f) = self.effects.get(key) {
            let mut r = f();
            r.set_uid(Uid(self.next_id.inc()));
            Some(r)
        } else {
            None
        }
    }

    pub fn controller_keys(
        &self,
    ) -> std::collections::hash_map::Keys<Key, fn() -> Box<dyn NewIsController>> {
        self.controllers.keys()
    }

    pub fn effect_keys(
        &self,
    ) -> std::collections::hash_map::Keys<Key, fn() -> Box<dyn NewIsEffect>> {
        self.effects.keys()
    }

    pub fn instrument_keys(
        &self,
    ) -> std::collections::hash_map::Keys<Key, fn() -> Box<dyn NewIsInstrument>> {
        self.instruments.keys()
    }
}

#[derive(Debug)]
enum TrackElementAction {
    MoveControllerLeft(usize),
    MoveControllerRight(usize),
    RemoveController(usize),
    MoveEffectLeft(usize),
    MoveEffectRight(usize),
    RemoveEffect(usize),
    MoveInstrumentLeft(usize),
    MoveInstrumentRight(usize),
    RemoveInstrument(usize),
}

#[allow(dead_code)]
#[derive(Debug)]
enum TrackAction {
    NewController(usize, Key),
    NewEffect(usize, Key),
    NewInstrument(usize, Key),
}

#[derive(Clone, Copy, Debug)]
enum MenuBarAction {
    Quit,
    TrackNew,
    TrackDuplicate,
    TrackDelete,
    ComingSoon,
}

#[derive(Debug)]
struct MenuBarItem {
    name: String,
    children: Option<Vec<MenuBarItem>>,
    action: Option<MenuBarAction>,
    enabled: bool,
}
impl MenuBarItem {
    fn node(name: &str, children: Vec<MenuBarItem>) -> Self {
        Self {
            name: name.to_string(),
            children: Some(children),
            action: None,
            enabled: true,
        }
    }
    fn leaf(name: &str, action: MenuBarAction, enabled: bool) -> Self {
        Self {
            name: name.to_string(),
            children: None,
            action: Some(action),
            enabled,
        }
    }
    fn show(&self, ui: &mut Ui) -> Option<MenuBarAction> {
        let mut action = None;
        if let Some(children) = self.children.as_ref() {
            ui.menu_button(&self.name, |ui| {
                for child in children.iter() {
                    if let Some(a) = child.show(ui) {
                        action = Some(a);
                    }
                }
            });
        } else if let Some(action_to_perform) = &self.action {
            if ui
                .add_enabled(self.enabled, Button::new(&self.name))
                .clicked()
            {
                ui.close_menu();
                action = Some(*action_to_perform);
            }
        }
        action
    }
}

#[derive(Debug, Default)]
struct MenuBar {}
impl MenuBar {
    fn show_with_action(&mut self, ui: &mut Ui, is_track_selected: bool) -> Option<MenuBarAction> {
        let mut action = None;

        // Menus should look like menus, not buttons
        ui.style_mut().visuals.button_frame = false;

        ui.horizontal(|ui| {
            let menus = vec![
                MenuBarItem::node(
                    "File",
                    vec![MenuBarItem::leaf("Quit", MenuBarAction::Quit, true)],
                ),
                MenuBarItem::node(
                    "Track",
                    vec![
                        MenuBarItem::leaf("New", MenuBarAction::TrackNew, true),
                        MenuBarItem::leaf("New Send", MenuBarAction::ComingSoon, true),
                        MenuBarItem::leaf(
                            "Duplicate",
                            MenuBarAction::TrackDuplicate,
                            is_track_selected,
                        ),
                        MenuBarItem::leaf("Delete", MenuBarAction::TrackDelete, is_track_selected),
                    ],
                ),
                MenuBarItem::node(
                    "Device",
                    vec![
                        MenuBarItem::leaf("New", MenuBarAction::ComingSoon, true),
                        MenuBarItem::leaf("Shift Left", MenuBarAction::ComingSoon, true),
                        MenuBarItem::leaf("Shift Right", MenuBarAction::ComingSoon, true),
                        MenuBarItem::leaf("Move Up", MenuBarAction::ComingSoon, true),
                        MenuBarItem::leaf("Move Down", MenuBarAction::ComingSoon, true),
                    ],
                ),
                MenuBarItem::node(
                    "Control",
                    vec![
                        MenuBarItem::leaf("Connect", MenuBarAction::ComingSoon, true),
                        MenuBarItem::leaf("Disconnect", MenuBarAction::ComingSoon, true),
                    ],
                ),
            ];
            for item in menus.iter() {
                if let Some(a) = item.show(ui) {
                    action = Some(a);
                }
            }
        });
        action
    }
}

#[derive(Debug)]
enum PaletteAction {
    NewController(Key),
    NewEffect(Key),
    NewInstrument(Key),
}
#[derive(Debug)]
struct PalettePanel {
    factory: Arc<EntityFactory>,
    drag_drop_manager: Arc<Mutex<DragDropManager>>,
}
impl Shows for PalettePanel {
    fn show(&mut self, ui: &mut egui::Ui) {
        for name in &self.factory.keys {
            ui.label(name.to_string());
        }
    }
}
impl PalettePanel {
    pub fn new_with(
        factory: Arc<EntityFactory>,
        drag_drop_manager: Arc<Mutex<DragDropManager>>,
    ) -> Self {
        Self {
            factory,
            drag_drop_manager,
        }
    }

    #[allow(dead_code)]
    fn show_with_action(&mut self, ui: &mut egui::Ui) -> Option<PaletteAction> {
        let mut action = None;
        if let Ok(mut dnd) = self.drag_drop_manager.lock() {
            for key in self.factory.controller_keys() {
                dnd.drag_source(
                    ui,
                    EguiId::new(key),
                    DragDropSource::NewController(key.clone()),
                    |ui| {
                        if ui.button(key.to_string()).clicked() {
                            action = Some(PaletteAction::NewController(key.clone()));
                        }
                    },
                );
            }
            for key in self.factory.effect_keys() {
                dnd.drag_source(
                    ui,
                    EguiId::new(key),
                    DragDropSource::NewEffect(key.clone()),
                    |ui| {
                        if ui.button(key.to_string()).clicked() {
                            action = Some(PaletteAction::NewEffect(key.clone()));
                        }
                    },
                );
            }
            for key in self.factory.instrument_keys() {
                dnd.drag_source(
                    ui,
                    EguiId::new(key),
                    DragDropSource::NewInstrument(key.clone()),
                    |ui| {
                        if ui.button(key.to_string()).clicked() {
                            action = Some(PaletteAction::NewInstrument(key.clone()));
                        }
                    },
                );
            }
        }
        action
    }
}

#[allow(dead_code)]
#[derive(Debug)]
enum DragDropSource {
    ControllerInTrack(usize, Uid),
    EffectInTrack(usize, Uid),
    InstrumentInTrack(usize, Uid),
    NewController(Key),
    NewEffect(Key),
    NewInstrument(Key),
}

// TODO: a way to express rules about what can and can't be dropped
#[derive(Debug, Default)]
struct DragDropManager {
    source: Option<DragDropSource>,
}
impl DragDropManager {
    fn reset(&mut self) {
        self.source = None;
    }

    // These two functions are based on egui_demo_lib/src/demo/drag_and_drop.rs
    #[allow(dead_code)]
    fn drag_source(
        &mut self,
        ui: &mut Ui,
        id: EguiId,
        dnd_id: DragDropSource,
        body: impl FnOnce(&mut Ui),
    ) {
        let is_being_dragged = ui.memory(|mem| mem.is_being_dragged(id));

        if is_being_dragged {
            self.source = Some(dnd_id);
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
            let layer_id = LayerId::new(Order::Tooltip, id);
            let response = ui.with_layer_id(layer_id, body).response;
            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                let delta = pointer_pos - response.rect.center();
                ui.ctx().translate_layer(layer_id, delta);
            }
        } else {
            let response = ui.scope(body).response;
            let response = ui.interact(response.rect, id, Sense::drag());
            if response.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::Grab);
            }
        }
    }

    #[allow(dead_code)]
    fn drop_target<R>(
        &mut self,
        ui: &mut Ui,
        can_accept_what_is_being_dragged: bool,
        body: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let is_being_dragged = ui.memory(|mem| mem.is_anything_being_dragged());

        let margin = Vec2::splat(2.0);

        let outer_rect_bounds = ui.available_rect_before_wrap();
        let inner_rect = outer_rect_bounds.shrink2(margin);
        let where_to_put_background = ui.painter().add(Shape::Noop);
        let mut content_ui = ui.child_ui(inner_rect, *ui.layout());
        let ret = body(&mut content_ui);
        let outer_rect =
            Rect::from_min_max(outer_rect_bounds.min, content_ui.min_rect().max + margin);
        let (rect, response) = ui.allocate_at_least(outer_rect.size(), Sense::hover());

        let style = if is_being_dragged && can_accept_what_is_being_dragged && response.hovered() {
            ui.visuals().widgets.active
        } else {
            ui.visuals().widgets.inactive
        };

        let mut fill = style.bg_fill;
        let mut stroke = style.bg_stroke;
        if is_being_dragged && !can_accept_what_is_being_dragged {
            fill = ui.visuals().gray_out(fill);
            stroke.color = ui.visuals().gray_out(stroke.color);
        }

        ui.painter().set(
            where_to_put_background,
            epaint::RectShape {
                rounding: style.rounding,
                fill,
                stroke,
                rect,
            },
        );

        InnerResponse::new(ret, response)
    }
}

struct MiniDaw {
    mini_orchestrator: Arc<Mutex<MiniOrchestrator>>,

    menu_bar: MenuBar,
    control_panel: ControlPanel,
    orchestrator_panel: OrchestratorPanel,
    audio_panel: AudioPanel2,
    midi_panel: MidiPanel,
    palette_panel: PalettePanel,

    first_update_done: bool,
    exit_requested: bool,
    drag_drop_manager: Arc<Mutex<DragDropManager>>,

    #[allow(dead_code)]
    regular_font_id: FontId,
    #[allow(dead_code)]
    mono_font_id: FontId,
    #[allow(dead_code)]
    bold_font_id: FontId,
    bold_font_height: f32,

    toasts: Toasts,
}
impl MiniDaw {
    pub const FONT_REGULAR: &str = "font-regular";
    pub const FONT_BOLD: &str = "font-bold";
    pub const FONT_MONO: &str = "font-mono";
    pub const APP_NAME: &str = "MiniDAW";
    pub const DEFAULT_PROJECT_NAME: &str = "Untitled";

    pub fn new(cc: &CreationContext) -> Self {
        Self::initialize_fonts(cc);
        Self::initialize_style(&cc.egui_ctx);

        let mut factory = EntityFactory::default();
        Self::register_entities(&mut factory);
        let factory = Arc::new(factory);

        let drag_drop_manager = Arc::new(Mutex::new(DragDropManager::default()));
        let orchestrator_panel =
            OrchestratorPanel::new_with(Arc::clone(&factory), Arc::clone(&drag_drop_manager));
        let mini_orchestrator = Arc::clone(orchestrator_panel.orchestrator());

        let mini_orchestrator_for_fn = Arc::clone(&mini_orchestrator);
        let needs_audio: NeedsAudioFn = Box::new(move |audio_queue, samples_requested| {
            if let Ok(mut o) = mini_orchestrator_for_fn.lock() {
                o.provide_audio(audio_queue, samples_requested);
            }
        });

        let mut r = Self {
            mini_orchestrator,
            menu_bar: Default::default(),
            control_panel: Default::default(),
            orchestrator_panel,
            audio_panel: AudioPanel2::new_with(Box::new(needs_audio)),
            midi_panel: Default::default(),
            palette_panel: PalettePanel::new_with(factory, Arc::clone(&drag_drop_manager)),

            first_update_done: Default::default(),
            exit_requested: Default::default(),
            drag_drop_manager,

            regular_font_id: FontId::proportional(14.0),
            bold_font_id: FontId::new(12.0, FontFamily::Name(Self::FONT_BOLD.into())),
            bold_font_height: Default::default(),
            mono_font_id: FontId::monospace(14.0),

            toasts: Toasts::new()
                .anchor(Align2::RIGHT_BOTTOM, (-10.0, -10.0))
                .direction(egui::Direction::BottomUp),
        };
        r.spawn_channel_watcher(cc.egui_ctx.clone());
        r
    }

    fn initialize_fonts(cc: &CreationContext) {
        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            Self::FONT_REGULAR.to_owned(),
            FontData::from_static(include_bytes!("../res/fonts/inter/Inter-Regular.ttf")),
        );
        fonts.font_data.insert(
            Self::FONT_BOLD.to_owned(),
            FontData::from_static(include_bytes!("../res/fonts/inter/Inter-Bold.ttf")),
        );
        fonts.font_data.insert(
            Self::FONT_MONO.to_owned(),
            FontData::from_static(include_bytes!("../res/fonts/cousine/Cousine-Regular.ttf")),
        );
        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, Self::FONT_REGULAR.to_owned());
        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .insert(0, Self::FONT_MONO.to_owned());
        fonts
            .families
            .entry(FontFamily::Name(Self::FONT_BOLD.into()))
            .or_default()
            .insert(0, Self::FONT_BOLD.to_owned());

        cc.egui_ctx.set_fonts(fonts);
    }

    fn initialize_style(ctx: &Context) {
        let mut style = (*ctx.style()).clone();

        style.visuals.override_text_color = Some(Color32::LIGHT_GRAY);

        style.text_styles = [
            (
                TextStyle::Heading,
                FontId::new(14.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Name("Heading2".into()),
                FontId::new(25.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Name("Context".into()),
                FontId::new(23.0, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(12.0, FontFamily::Proportional)),
            (
                TextStyle::Monospace,
                FontId::new(12.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Button,
                FontId::new(12.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Small,
                FontId::new(10.0, FontFamily::Proportional),
            ),
        ]
        .into();

        ctx.set_style(style);
    }

    fn handle_message_channels(&mut self) {
        // As long as any channel had a message in it, we'll keep handling them.
        // We don't expect a giant number of messages; otherwise we'd worry
        // about blocking the UI.
        loop {
            if !(self.handle_midi_panel_channel()
                || self.handle_audio_panel_channel()
                || self.handle_mini_orchestrator_channel())
            {
                break;
            }
        }
    }

    fn handle_midi_panel_channel(&mut self) -> bool {
        if let Ok(m) = self.midi_panel.receiver().try_recv() {
            match m {
                MidiPanelEvent::Midi(channel, message) => {
                    self.orchestrator_panel
                        .send_to_service(MiniOrchestratorInput::Midi(channel, message));
                }
                MidiPanelEvent::SelectInput(_) => {
                    // TODO: save selection in prefs
                }
                MidiPanelEvent::SelectOutput(_) => {
                    // TODO: save selection in prefs
                }
                MidiPanelEvent::PortsRefreshed => {
                    // TODO: remap any saved preferences to ports that we've found
                }
            }
            true
        } else {
            false
        }
    }

    fn handle_audio_panel_channel(&mut self) -> bool {
        if let Ok(m) = self.audio_panel.receiver().try_recv() {
            match m {
                AudioPanelEvent::InterfaceChanged => {
                    self.update_orchestrator_audio_interface_config();
                }
            }
            true
        } else {
            false
        }
    }

    fn handle_mini_orchestrator_channel(&mut self) -> bool {
        if let Ok(m) = self.orchestrator_panel.receiver().try_recv() {
            match m {
                MiniOrchestratorEvent::Tempo(tempo) => {
                    self.control_panel.set_tempo(tempo);
                }
                MiniOrchestratorEvent::Quit => {
                    eprintln!("MiniOrchestratorEvent::Quit")
                }
                MiniOrchestratorEvent::Loaded(path, title) => {
                    self.toasts.add(Toast {
                        kind: egui_toast::ToastKind::Success,
                        text: format!(
                            "Loaded {} from {}",
                            if let Some(title) = title {
                                title
                            } else {
                                Self::DEFAULT_PROJECT_NAME.to_string()
                            },
                            path.display()
                        )
                        .into(),
                        options: ToastOptions::default()
                            .duration_in_seconds(2.0)
                            .show_progress(false),
                    });
                }
                MiniOrchestratorEvent::LoadError(path, error) => {
                    self.toasts.add(Toast {
                        kind: egui_toast::ToastKind::Error,
                        text: format!("Error loading {}: {}", path.display(), error).into(),
                        options: ToastOptions::default().duration_in_seconds(5.0),
                    });
                }
                MiniOrchestratorEvent::Saved(path) => {
                    // TODO: this should happen only if the save operation was
                    // explicit. Autosaves should be invisible.
                    self.toasts.add(Toast {
                        kind: egui_toast::ToastKind::Success,
                        text: format!("Saved to {}", path.display()).into(),
                        options: ToastOptions::default()
                            .duration_in_seconds(1.0)
                            .show_progress(false),
                    });
                }
                MiniOrchestratorEvent::SaveError(path, error) => {
                    self.toasts.add(Toast {
                        kind: egui_toast::ToastKind::Error,
                        text: format!("Error saving {}: {}", path.display(), error).into(),
                        options: ToastOptions::default().duration_in_seconds(5.0),
                    });
                }
                MiniOrchestratorEvent::New => {
                    // No special UI needed for this.
                    eprintln!("asdfasd");
                }
            }
            true
        } else {
            false
        }
    }

    // Watches certain channels and asks for a repaint, which triggers the
    // actual channel receiver logic, when any of them has something receivable.
    //
    // https://docs.rs/crossbeam-channel/latest/crossbeam_channel/struct.Select.html#method.ready
    //
    // We call ready() rather than select() because select() requires us to
    // complete the operation that is ready, while ready() just tells us that a
    // recv() would not block.
    fn spawn_channel_watcher(&mut self, ctx: Context) {
        let r1 = self.midi_panel.receiver().clone();
        let r2 = self.audio_panel.receiver().clone();
        let r3 = self.orchestrator_panel.receiver().clone();
        let _ = std::thread::spawn(move || {
            let mut sel = Select::new();
            let _ = sel.recv(&r1);
            let _ = sel.recv(&r2);
            let _ = sel.recv(&r3);
            loop {
                let _ = sel.ready();
                ctx.request_repaint();
            }
        });
    }

    fn update_orchestrator_audio_interface_config(&mut self) {
        let sample_rate = self.audio_panel.sample_rate();
        if let Ok(mut o) = self.mini_orchestrator.lock() {
            o.set_sample_rate(SampleRate::from(sample_rate));
        }
    }

    fn handle_control_panel_action(&mut self, action: ControlPanelAction) {
        let input = match action {
            ControlPanelAction::Play => MiniOrchestratorInput::ProjectPlay,
            ControlPanelAction::Stop => MiniOrchestratorInput::ProjectStop,
            ControlPanelAction::New => MiniOrchestratorInput::ProjectNew,
            ControlPanelAction::Load(path) => MiniOrchestratorInput::ProjectLoad(path),
            ControlPanelAction::Save(path) => MiniOrchestratorInput::ProjectSave(path),
        };
        self.orchestrator_panel.send_to_service(input);
    }

    fn handle_menu_bar_action(&mut self, action: MenuBarAction) {
        let mut input = None;
        match action {
            MenuBarAction::Quit => self.exit_requested = true,
            MenuBarAction::TrackNew => input = Some(MiniOrchestratorInput::TrackNew),
            MenuBarAction::TrackDelete => {
                if let Some(index) = self.orchestrator_panel.selected_track() {
                    input = Some(MiniOrchestratorInput::TrackDelete(index))
                }
            }
            MenuBarAction::TrackDuplicate => {
                if let Some(index) = self.orchestrator_panel.selected_track() {
                    input = Some(MiniOrchestratorInput::TrackDuplicate(index))
                }
            }
            MenuBarAction::ComingSoon => {
                self.toasts.add(Toast {
                    kind: egui_toast::ToastKind::Info,
                    text: "Coming soon!".into(),
                    options: ToastOptions::default(),
                });
            }
        }
        if let Some(input) = input {
            self.orchestrator_panel.send_to_service(input);
        }
    }

    pub fn register_entities(factory: &mut EntityFactory) {
        // TODO: might be nice to move HasUid::name() to be a function... and
        // while we're at it, I guess make the mondo IsEntity trait that allows
        // discovery of IsInstrument/Effect/Controller.

        factory.register_controller(Key::from("arpeggiator"), || {
            Box::new(Arpeggiator::new_with(
                &ArpeggiatorParams::default(),
                MidiChannel::new(0),
            ))
        });
        factory.register_effect(Key::from("reverb"), || {
            Box::new(Reverb::new_with(&ReverbParams::default()))
        });
        factory.register_effect(Key::from("filter-low-pass-24db"), || {
            Box::new(BiQuadFilterLowPass24db::new_with(
                &BiQuadFilterLowPass24dbParams::default(),
            ))
        });
        factory.register_instrument(Key::from("toy-synth"), || {
            Box::new(ToySynth::new_with(&ToySynthParams::default()))
        });
        factory.register_instrument(Key::from("toy-instrument"), || {
            Box::new(ToyInstrument::new_with(&ToyInstrumentParams::default()))
        });
        factory.register_instrument(Key::from("welsh-synth"), || {
            Box::new(WelshSynth::new_with(&WelshSynthParams::default()))
        });
    }

    fn handle_palette_action(&mut self, _action: PaletteAction) {
        if let Ok(_o) = self.mini_orchestrator.lock() {
            // match action {
            //     PaletteAction::NewController(key) => {
            //         if let Some(controller) = self.factory.new_controller(&key) {
            //             let id = o.add_controller(controller);
            //             o.push_to_last_track(id);
            //         }
            //     }
            //     PaletteAction::NewEffect(key) => {
            //         if let Some(effect) = self.factory.new_effect(&key) {
            //             let id = o.add_effect(effect);
            //             o.push_to_last_track(id);
            //         }
            //     }
            //     PaletteAction::NewInstrument(key) => {
            //         if let Some(instrument) = self.factory.new_instrument(&key) {
            //             let id = o.add_instrument(instrument);
            //             o.push_to_last_track(id);
            //         }
            //     }
            // }
        }
    }

    fn show_top(&mut self, ui: &mut egui::Ui) {
        if let Some(action) = self
            .menu_bar
            .show_with_action(ui, self.orchestrator_panel.selected_track().is_some())
        {
            self.handle_menu_bar_action(action);
        }
        ui.separator();
        if let Some(action) = self.control_panel.show_with_action(ui) {
            self.handle_control_panel_action(action);
        }
    }

    fn show_bottom(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            egui::warn_if_debug_build(ui);
            ui.with_layout(Layout::right_to_left(eframe::emath::Align::Center), |ui| {
                ui.label(app_version())
            });
        });
    }

    fn show_left(&mut self, ui: &mut egui::Ui) {
        if let Some(action) = self.palette_panel.show_with_action(ui) {
            // these are inactive for now because we're skipping the drag/drop stuff.
            self.handle_palette_action(action);
        }
    }

    fn show_right(&mut self, ui: &mut egui::Ui) {
        self.audio_panel.show(ui);
        self.midi_panel.show(ui);
    }

    fn show_center(&mut self, ui: &mut egui::Ui) {
        self.orchestrator_panel.show(ui);
    }

    fn update_window_title(&mut self, frame: &mut eframe::Frame) {
        // TODO: it seems like the window remembers its title, so this isn't
        // something we should be doing on every frame.
        let full_title = format!(
            "{} - {}",
            Self::APP_NAME,
            if let Some(title) = {
                if let Ok(o) = self.orchestrator_panel.orchestrator().lock() {
                    o.title().cloned()
                } else {
                    None
                }
            } {
                title
            } else {
                Self::DEFAULT_PROJECT_NAME.to_string()
            }
        );
        frame.set_window_title(&full_title);
    }
}
impl eframe::App for MiniDaw {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.handle_message_channels();
        if !self.first_update_done {
            self.first_update_done = true;
            ctx.fonts(|f| self.bold_font_height = f.row_height(&self.bold_font_id));
        }
        if let Ok(mut dnd) = self.drag_drop_manager.lock() {
            dnd.reset();
        }

        self.update_window_title(frame);

        let top = egui::TopBottomPanel::top("top-panel")
            .resizable(false)
            .exact_height(64.0);
        let left = egui::SidePanel::left("left-panel")
            .resizable(true)
            .default_width(150.0)
            .width_range(80.0..=200.0);
        let right = egui::SidePanel::right("right-panel")
            .resizable(true)
            .default_width(150.0)
            .width_range(80.0..=200.0);
        let bottom = egui::TopBottomPanel::bottom("bottom-panel")
            .resizable(false)
            .exact_height(self.bold_font_height + 2.0);
        let center = egui::CentralPanel::default();

        top.show(ctx, |ui| {
            self.show_top(ui);
        });
        left.show(ctx, |ui| {
            self.show_left(ui);
        });
        right.show(ctx, |ui| {
            self.show_right(ui);
        });
        bottom.show(ctx, |ui| {
            self.show_bottom(ui);
        });
        center.show(ctx, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                self.show_center(ui);
            });
            self.toasts.show(ctx);
        });

        if self.exit_requested {
            frame.close();
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.audio_panel.exit();
        self.midi_panel.exit();
        self.orchestrator_panel.exit();
    }
}

#[typetag::serde]
impl NewIsController for Arpeggiator {}
#[typetag::serde]
impl NewIsInstrument for WelshSynth {}
#[typetag::serde]
impl NewIsInstrument for ToySynth {}
#[typetag::serde]
impl NewIsInstrument for ToyInstrument {}
#[typetag::serde]
impl NewIsEffect for BiQuadFilterLowPass24db {}
#[typetag::serde]
impl NewIsEffect for Reverb {}

fn main() -> anyhow::Result<(), eframe::Error> {
    env_logger::init();
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1024.0, 768.0)),
        ..Default::default()
    };

    eframe::run_native(
        MiniDaw::APP_NAME,
        options,
        Box::new(|cc| Box::new(MiniDaw::new(cc))),
    )
}

#[cfg(test)]
mod tests {
    use crate::{EntityFactory, Key, MiniDaw, MiniOrchestrator, Track, Uid};
    use groove_core::traits::HasUid;
    use groove_toys::{ToyInstrument, ToyInstrumentParams};
    use std::collections::HashSet;

    #[test]
    fn entity_creation() {
        let mut factory = EntityFactory::default();
        assert!(factory.controllers.is_empty());
        assert!(factory.instruments.is_empty());
        assert!(factory.effects.is_empty());

        // Register, then rebind as immutable
        MiniDaw::register_entities(&mut factory);
        let factory = factory;

        assert!(!factory.controllers.is_empty());
        assert!(!factory.instruments.is_empty());
        assert!(!factory.effects.is_empty());

        assert!(factory.new_instrument(&Key::from(".9-#$%)@#)")).is_none());

        let mut ids: HashSet<Uid> = HashSet::default();
        for key in factory.instrument_keys() {
            let e = factory.new_instrument(key);
            assert!(e.is_some());
            if let Some(e) = e {
                assert!(!e.name().is_empty());
                assert!(!ids.contains(&Uid(e.uid())));
                ids.insert(Uid(e.uid()));
            }
        }

        // TODO: expand with other entity types, and create the uber-trait that
        // lets us create an entity and then grab the specific IsWhatever trait.
    }

    #[test]
    fn basic_track_operations() {
        let mut t = Track::default();
        assert!(t.controllers.is_empty());
        assert!(t.effects.is_empty());
        assert!(t.instruments.is_empty());

        // Create an instrument and add it to a track.
        let instrument = ToyInstrument::new_with(&ToyInstrumentParams::default());
        let id1 = Uid(instrument.uid());
        t.append_instrument(Box::new(instrument));

        // Add a second instrument to the track.
        let instrument = ToyInstrument::new_with(&ToyInstrumentParams::default());
        let id2 = Uid(instrument.uid());
        t.append_instrument(Box::new(instrument));

        // Ordering within track is correct, and we can move items around
        // depending on where they are.
        assert_eq!(Uid(t.instruments[0].uid()), id1);
        assert_eq!(Uid(t.instruments[1].uid()), id2);
        assert!(t.shift_instrument_left(0).is_err()); // Already leftmost.
        assert!(t.shift_instrument_right(1).is_err()); // Already rightmost.
        assert!(t.shift_instrument_left(1).is_ok());
        assert_eq!(Uid(t.instruments[0].uid()), id2);
        assert_eq!(Uid(t.instruments[1].uid()), id1);

        let instrument = t.remove_instrument(0).unwrap();
        assert_eq!(Uid(instrument.uid()), id2);
        assert_eq!(t.instruments.len(), 1);
    }

    #[test]
    fn mini_orchestrator_basic_operations() {
        let mut o = MiniOrchestrator::default();

        // A new orchestrator should have at least one track.
        assert!(!o.tracks.is_empty());

        let id1 = o
            .add_instrument(
                0,
                Box::new(ToyInstrument::new_with(&ToyInstrumentParams::default())),
            )
            .unwrap();
        let id2 = o
            .add_instrument(
                0,
                Box::new(ToyInstrument::new_with(&ToyInstrumentParams::default())),
            )
            .unwrap();
        assert_eq!(Uid(o.tracks[0].instruments[0].uid()), id1);
        assert_eq!(Uid(o.tracks[0].instruments[1].uid()), id2);

        assert!(o.tracks.len() > 1);
        let id3 = o
            .add_instrument(
                1,
                Box::new(ToyInstrument::new_with(&ToyInstrumentParams::default())),
            )
            .unwrap();

        // Moving something to another track works.
        assert_eq!(o.tracks[0].instruments.len(), 2);
        assert_eq!(o.tracks[1].instruments.len(), 1);
        assert!(o.move_instrument(1, 0, 0, 0).is_ok());
        assert_eq!(o.tracks[0].instruments.len(), 3);
        assert_eq!(o.tracks[1].instruments.len(), 0);
        assert_eq!(o.tracks[0].instruments[0].uid(), id3);
    }
}
