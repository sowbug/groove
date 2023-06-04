// Copyright (c) 2023 Mike Tsao. All rights reserved.

use anyhow::{anyhow, Result};
use crossbeam_channel::{Receiver, Select, Sender};
use derive_more::Display;
use eframe::{
    egui::{
        self, Context, CursorIcon, FontData, FontDefinitions, Frame, Id as EguiId, InnerResponse,
        LayerId, Layout, Order, RichText, ScrollArea, Sense, TextStyle, Ui,
    },
    emath::Align2,
    epaint::{self, Color32, FontFamily, FontId, Rect, Shape, Vec2},
    CreationContext,
};
use egui_toast::Toasts;
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
    traits::{gui::Shows, Configurable, Generates, IsController, IsEffect, IsInstrument, Ticks},
    StereoSample,
};
use groove_entities::{
    controllers::{Arpeggiator, ArpeggiatorParams},
    effects::{Reverb, ReverbParams},
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

#[derive(Copy, Clone, Serialize, Deserialize, Debug, Default, Display, Eq, PartialEq, Hash)]
struct Id(usize);
impl Id {
    fn increment(&mut self) {
        self.0 += 1;
    }
}

#[typetag::serde(tag = "type")]
trait NewIsController: IsController<Message = EntityMessage> {}

#[typetag::serde(tag = "type")]
trait NewIsInstrument: IsInstrument {}

#[typetag::serde(tag = "type")]
trait NewIsEffect: IsEffect {}

#[derive(Clone, Debug)]
enum MiniOrchestratorInput {
    Midi(MidiChannel, MidiMessage),
    Play,
    Stop,
    Load(PathBuf),
    Save(PathBuf),

    /// Request that the orchestrator service quit.
    Quit,
}

#[derive(Clone, Debug)]
enum MiniOrchestratorEvent {
    Tempo(Tempo),

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
                    MiniOrchestratorInput::Play => eprintln!("Play"),
                    MiniOrchestratorInput::Stop => eprintln!("Stop"),
                    MiniOrchestratorInput::Load(path) => {
                        match Self::handle_input_load(&path) {
                            Ok(mut mo) => {
                                if let Ok(mut o) = orchestrator.lock() {
                                    o.prepare_successor(&mut mo);
                                    *o = mo;
                                    eprintln!("loaded from {:?}", &path);
                                }
                            }
                            Err(_) => todo!(),
                        }
                        {}
                    }
                    MiniOrchestratorInput::Save(path) => {
                        match Self::handle_input_save(&orchestrator, &path) {
                            Ok(_) => {
                                eprintln!("saved to {:?}", &path)
                            }
                            Err(_) => todo!(),
                        }
                    }
                    MiniOrchestratorInput::Quit => {
                        let _ = sender.send(MiniOrchestratorEvent::Quit);
                        break;
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
                Ok(mo) => anyhow::Ok(mo),
                Err(err) => Err(anyhow!("Error while parsing {:?}: {}", path, err)),
            },
            Err(err) => Err(anyhow!("Error while reading {:?}: {}", path, err)),
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
                    Err(err) => Err(anyhow!("While writing project to {:?}: {}", path, err)),
                },
                Err(err) => Err(anyhow!(
                    "While serializing project to be written to {:?}: {}",
                    path,
                    err
                )),
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
            if let Ok(mut dnd) = self.drag_drop_manager.lock() {
                o.show_with(ui, &self.factory, &mut dnd);
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct MiniOrchestrator {
    time_signature: TimeSignature,
    tempo: Tempo,

    next_id: Id,
    controllers: HashMap<Id, Box<dyn NewIsController>>,
    instruments: HashMap<Id, Box<dyn NewIsInstrument>>,
    effects: HashMap<Id, Box<dyn NewIsEffect>>,

    tracks: Vec<Vec<Id>>,

    // Nothing below this comment should be serialized.
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
            time_signature: Default::default(),
            tempo: Default::default(),
            next_id: Id(1),
            controllers: Default::default(),
            instruments: Default::default(),
            effects: Default::default(),

            tracks: vec![Default::default()],

            sample_rate: Default::default(),
            frames: Default::default(),
            musical_time: Default::default(),
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
        for i in self.instruments.values_mut() {
            i.update_sample_rate(sample_rate);
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
        for i in self.instruments.values_mut() {
            i.handle_midi_message(&message, &mut |channel, message| {
                eprintln!("TODO discarding {}/{:?}", channel, message)
            });
        }
    }

    /// Returns the next unique [Id] to refer to a new entity.
    fn next_id(&mut self) -> Id {
        let r = self.next_id;
        self.next_id.increment();
        r
    }

    fn add_controller(&mut self, mut e: Box<dyn NewIsController>) -> Id {
        e.update_sample_rate(self.sample_rate);
        let id = self.next_id();
        self.controllers.insert(id, e);
        id
    }

    fn add_effect(&mut self, mut e: Box<dyn NewIsEffect>) -> Id {
        e.update_sample_rate(self.sample_rate);
        let id = self.next_id();
        self.effects.insert(id, e);
        id
    }

    fn add_instrument(&mut self, mut e: Box<dyn NewIsInstrument>) -> Id {
        e.update_sample_rate(self.sample_rate);
        let id = self.next_id();
        self.instruments.insert(id, e);
        id
    }

    /// After loading a new Self from disk, we want to copy all the appropriate
    /// ephemeral state from this one to the next one.
    fn prepare_successor(&self, new: &mut MiniOrchestrator) {
        new.set_sample_rate(self.sample_rate());
    }

    // TODO: ordering should be controllers, instruments, then effects. Within
    // those groups, the user can reorder as desired (but instrument order
    // doesn't matter because they're all simultaneous)
    fn push_to_last_track(&mut self, id: Id) {
        if self.tracks.is_empty() {
            self.tracks.push(Vec::default());
        }
        if let Some(track) = self.tracks.last_mut() {
            track.push(id);
        }
    }

    fn push_to_track(&mut self, track_index: usize, id: Id) {
        if track_index < self.tracks.len() {
            self.tracks[track_index].push(id);
        }
        // Did we just add the first item to the last track?
        if track_index == self.tracks.len() - 1 {
            if self.tracks[track_index].len() == 1 {
                self.push_new_track();
            }
        }
    }

    fn move_item_track(&mut self, old_track: usize, new_track: usize, id: Id) {
        self.tracks[old_track].retain(|i| i != &id);
        self.tracks[new_track].push(id);
    }

    fn move_item_position(&mut self, track: usize, id: Id, new_position: usize) {
        self.tracks[track].retain(|i| i != &id);
        self.tracks[track].insert(new_position, id);
    }

    // TODO: this is getting cumbersome! Think about that uber-trait!

    fn controller(&self, id: &Id) -> Option<&Box<dyn NewIsController>> {
        self.controllers.get(id)
    }

    fn controller_mut(&mut self, id: &Id) -> Option<&mut Box<dyn NewIsController>> {
        self.controllers.get_mut(id)
    }

    fn effect(&self, id: &Id) -> Option<&Box<dyn NewIsEffect>> {
        self.effects.get(id)
    }

    fn effect_mut(&mut self, id: &Id) -> Option<&mut Box<dyn NewIsEffect>> {
        self.effects.get_mut(id)
    }

    fn instrument(&self, id: &Id) -> Option<&Box<dyn NewIsInstrument>> {
        self.instruments.get(id)
    }

    fn instrument_mut(&mut self, id: &Id) -> Option<&mut Box<dyn NewIsInstrument>> {
        self.instruments.get_mut(id)
    }

    fn add_track_element(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) {
        let style = ui.visuals().widgets.inactive;
        Frame::none()
            .stroke(style.fg_stroke)
            .fill(style.bg_fill)
            .show(ui, |ui| {
                add_contents(ui);
            });
    }

    fn show_tracks(&mut self, ui: &mut Ui, factory: &EntityFactory, dnd: &mut DragDropManager) {
        let style = ui.visuals().widgets.inactive;
        for (track_index, track) in self.tracks.clone().iter().enumerate() {
            Frame::none()
                .stroke(style.fg_stroke)
                .fill(style.bg_fill)
                .show(ui, |ui| {
                    let desired_size =
                        Vec2::new(ui.available_width(), 64.0 - style.fg_stroke.width);
                    ui.set_min_size(desired_size);
                    ui.set_max_size(desired_size);

                    let mut drop_track_index = None;
                    ui.horizontal_centered(|ui| {
                        let desired_size = Vec2::new(96.0, ui.available_height());
                        for id in track.iter() {
                            Self::add_track_element(ui, |ui| {
                                ui.set_min_size(desired_size);
                                ui.set_max_size(desired_size);
                                if let Some(e) = self.controller_mut(id) {
                                    dnd.drag_source(
                                        ui,
                                        EguiId::new(ui.next_auto_id()),
                                        DragDropSource::ControllerInTrack(track_index, *id),
                                        |ui| {
                                            e.show(ui);
                                        },
                                    );
                                } else if let Some(e) = self.instrument_mut(id) {
                                    dnd.drag_source(
                                        ui,
                                        EguiId::new(ui.next_auto_id()),
                                        DragDropSource::InstrumentInTrack(track_index, *id),
                                        |ui| {
                                            e.show(ui);
                                        },
                                    );
                                } else if let Some(e) = self.effect_mut(id) {
                                    dnd.drag_source(
                                        ui,
                                        EguiId::new(ui.next_auto_id()),
                                        DragDropSource::EffectInTrack(track_index, *id),
                                        |ui| {
                                            e.show(ui);
                                        },
                                    );
                                }
                            });
                        }

                        // Drop target at the end for new stuff
                        ui.add_space(1.0);
                        let response = dnd
                            .drop_target(ui, true, |ui| {
                                Self::add_track_element(ui, |ui| {
                                    let desired_size =
                                        Vec2::new(desired_size.x / 4.0, desired_size.y - 8.0);
                                    ui.set_max_size(desired_size);
                                    ui.label(RichText::new("+").size(24.0));
                                });
                            })
                            .response;
                        if response.hovered() {
                            drop_track_index = Some(track_index);
                        }
                    });
                    if let Some(drop_track_index) = drop_track_index {
                        if ui.input(|i| i.pointer.any_released()) {
                            if let Some(source) = &dnd.source {
                                match source {
                                    DragDropSource::NewController(key) => {
                                        if let Some(controller) = factory.new_controller(key) {
                                            let id = self.add_controller(controller);
                                            self.push_to_track(drop_track_index, id);
                                        }
                                    }
                                    DragDropSource::NewEffect(key) => {
                                        if let Some(effect) = factory.new_effect(key) {
                                            let id = self.add_effect(effect);
                                            self.push_to_track(drop_track_index, id);
                                        }
                                    }
                                    DragDropSource::NewInstrument(key) => {
                                        if let Some(instrument) = factory.new_instrument(key) {
                                            let id = self.add_instrument(instrument);
                                            self.push_to_track(drop_track_index, id);
                                        }
                                    }
                                    DragDropSource::ControllerInTrack(old_track_index, id)
                                    | DragDropSource::InstrumentInTrack(old_track_index, id)
                                    | DragDropSource::EffectInTrack(old_track_index, id) => {
                                        if drop_track_index == *old_track_index {
                                            self.move_item_position(drop_track_index, *id, 0);
                                        } else {
                                            self.move_item_track(
                                                *old_track_index,
                                                drop_track_index,
                                                *id,
                                            );
                                        }
                                    }
                                }
                            } else {
                                eprintln!(
                                    "dropped on track {drop_track_index}, but source is missing!"
                                );
                            }
                        }
                    }
                });
        }
    }

    fn show_with(&mut self, ui: &mut egui::Ui, factory: &EntityFactory, dnd: &mut DragDropManager) {
        self.show_tracks(ui, factory, dnd);
    }

    fn push_new_track(&mut self) {
        self.tracks.push(Default::default());
    }
}
impl Generates<StereoSample> for MiniOrchestrator {
    fn value(&self) -> StereoSample {
        StereoSample::SILENCE
    }

    fn batch_values(&mut self, values: &mut [StereoSample]) {
        let frames = 0..values.len();
        for frame in frames {
            for i in self.instruments.values_mut() {
                i.tick(1);
                values[frame] = i.value();
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
            Some(f())
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
            Some(f())
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
            Some(f())
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

#[derive(Debug)]
enum DragDropSource {
    ControllerInTrack(usize, Id),
    EffectInTrack(usize, Id),
    InstrumentInTrack(usize, Id),
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
    factory: Arc<EntityFactory>,

    orchestrator_panel: OrchestratorPanel,
    control_panel: ControlPanel,
    audio_panel: AudioPanel2,
    midi_panel: MidiPanel,
    palette_panel: PalettePanel,

    first_update_done: bool,
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
            factory: Arc::clone(&factory),
            orchestrator_panel,
            control_panel: Default::default(),
            audio_panel: AudioPanel2::new_with(Box::new(needs_audio)),
            midi_panel: Default::default(),
            palette_panel: PalettePanel::new_with(factory, Arc::clone(&drag_drop_manager)),

            first_update_done: Default::default(),
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
        match action {
            ControlPanelAction::Play => self
                .orchestrator_panel
                .send_to_service(MiniOrchestratorInput::Play),
            ControlPanelAction::Stop => self
                .orchestrator_panel
                .send_to_service(MiniOrchestratorInput::Stop),
            ControlPanelAction::Load(path) => self
                .orchestrator_panel
                .send_to_service(MiniOrchestratorInput::Load(path)),
            ControlPanelAction::Save(path) => self
                .orchestrator_panel
                .send_to_service(MiniOrchestratorInput::Save(path)),
        }
    }

    fn register_entities(factory: &mut EntityFactory) {
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

    fn handle_palette_action(&mut self, action: PaletteAction) {
        if let Ok(mut o) = self.mini_orchestrator.lock() {
            match action {
                PaletteAction::NewController(key) => {
                    if let Some(controller) = self.factory.new_controller(&key) {
                        let id = o.add_controller(controller);
                        o.push_to_last_track(id);
                    }
                }
                PaletteAction::NewEffect(key) => {
                    if let Some(effect) = self.factory.new_effect(&key) {
                        let id = o.add_effect(effect);
                        o.push_to_last_track(id);
                    }
                }
                PaletteAction::NewInstrument(key) => {
                    if let Some(instrument) = self.factory.new_instrument(&key) {
                        let id = o.add_instrument(instrument);
                        o.push_to_last_track(id);
                    }
                }
            }
        }
    }

    fn show_top(&mut self, ui: &mut egui::Ui) {
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
}
impl eframe::App for MiniDaw {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Ok(mut dnd) = self.drag_drop_manager.lock() {
            dnd.reset();
        }
        self.handle_message_channels();
        if !self.first_update_done {
            self.first_update_done = true;
            ctx.fonts(|f| self.bold_font_height = f.row_height(&self.bold_font_id));
        }

        let top = egui::TopBottomPanel::top("top-panel")
            .resizable(false)
            .exact_height(64.0);
        let bottom = egui::TopBottomPanel::bottom("bottom-panel")
            .resizable(false)
            .exact_height(self.bold_font_height + 2.0);
        let left = egui::SidePanel::left("left-panel")
            .resizable(true)
            .default_width(150.0)
            .width_range(80.0..=200.0);
        let right = egui::SidePanel::right("right-panel")
            .resizable(true)
            .default_width(150.0)
            .width_range(80.0..=200.0);

        let center = egui::CentralPanel::default();
        top.show(ctx, |ui| {
            self.show_top(ui);
        });
        bottom.show(ctx, |ui| {
            self.show_bottom(ui);
        });
        left.show(ctx, |ui| {
            self.show_left(ui);
        });
        right.show(ctx, |ui| {
            self.show_right(ui);
        });
        center.show(ctx, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                self.show_center(ui);
            });
            self.toasts.show(ctx);
        });
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
impl NewIsEffect for Reverb {}

fn main() -> anyhow::Result<(), eframe::Error> {
    env_logger::init();
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1024.0, 768.0)),
        ..Default::default()
    };

    eframe::run_native(
        "MiniDAW",
        options,
        Box::new(|cc| Box::new(MiniDaw::new(cc))),
    )
}
