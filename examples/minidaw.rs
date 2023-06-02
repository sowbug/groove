// Copyright (c) 2023 Mike Tsao. All rights reserved.

use anyhow::{anyhow, Result};
use crossbeam_channel::{Receiver, Select, Sender};
use eframe::{
    egui::{self, Context, Layout, RichText, ScrollArea},
    emath::Align2,
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

#[derive(Copy, Clone, Serialize, Deserialize, Debug, Default, Eq, PartialEq, Hash)]
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
    orchestrator: Arc<Mutex<MiniOrchestrator>>,
    input_channel_pair: ChannelPair<MiniOrchestratorInput>,
    event_channel_pair: ChannelPair<MiniOrchestratorEvent>,
}
impl OrchestratorPanel {
    fn new_with(factory: Arc<EntityFactory>) -> Self {
        let mut r = Self {
            factory,
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

#[derive(Serialize, Deserialize, Debug)]
struct MiniOrchestrator {
    time_signature: TimeSignature,
    tempo: Tempo,

    next_id: Id,
    controllers: HashMap<Id, Box<dyn NewIsController>>,
    instruments: HashMap<Id, Box<dyn NewIsInstrument>>,
    effects: HashMap<Id, Box<dyn NewIsEffect>>,

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
            i.handle_midi_message(&message);
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
        ui.label(format!(
            "I have {} controllers, {} instruments, and {} effects",
            self.controllers.len(),
            self.instruments.len(),
            self.effects.len()
        ));
        for e in self.controllers.values_mut() {
            e.show(ui);
        }
        for e in self.effects.values_mut() {
            e.show(ui);
        }
        for e in self.instruments.values_mut() {
            e.show(ui);
        }
    }
}

type ControllerEntityFactoryFn = fn() -> Box<dyn NewIsController>;
type InstrumentEntityFactoryFn = fn() -> Box<dyn NewIsInstrument>;
type EffectEntityFactoryFn = fn() -> Box<dyn NewIsEffect>;
#[derive(Debug, Default)]
struct EntityFactory {
    controllers: HashMap<String, ControllerEntityFactoryFn>,
    instruments: HashMap<String, InstrumentEntityFactoryFn>,
    effects: HashMap<String, EffectEntityFactoryFn>,
    keys: HashSet<String>,
}
impl EntityFactory {
    pub fn register_controller(&mut self, key: &str, f: ControllerEntityFactoryFn) {
        if self.keys.insert(key.to_string()) {
            self.controllers.insert(key.to_string(), f);
        } else {
            panic!("register_controller({}): duplicate key. Exiting.", key);
        }
    }
    pub fn new_controller(&self, key: &str) -> Option<Box<dyn NewIsController>> {
        if let Some(f) = self.controllers.get(key) {
            Some(f())
        } else {
            None
        }
    }
    pub fn register_instrument(&mut self, key: &str, f: InstrumentEntityFactoryFn) {
        if self.keys.insert(key.to_string()) {
            self.instruments.insert(key.to_string(), f);
        } else {
            panic!("register_instrument({}): duplicate key. Exiting.", key);
        }
    }
    pub fn new_instrument(&self, key: &str) -> Option<Box<dyn NewIsInstrument>> {
        if let Some(f) = self.instruments.get(key) {
            Some(f())
        } else {
            None
        }
    }
    pub fn register_effect(&mut self, key: &str, f: EffectEntityFactoryFn) {
        if self.keys.insert(key.to_string()) {
            self.effects.insert(key.to_string(), f);
        } else {
            panic!("register_effect({}): duplicate key. Exiting.", key);
        }
    }
    pub fn new_effect(&self, key: &str) -> Option<Box<dyn NewIsEffect>> {
        if let Some(f) = self.effects.get(key) {
            Some(f())
        } else {
            None
        }
    }

    pub fn controller_keys(
        &self,
    ) -> std::collections::hash_map::Keys<String, fn() -> Box<dyn NewIsController>> {
        self.controllers.keys()
    }

    pub fn effect_keys(
        &self,
    ) -> std::collections::hash_map::Keys<String, fn() -> Box<dyn NewIsEffect>> {
        self.effects.keys()
    }

    pub fn instrument_keys(
        &self,
    ) -> std::collections::hash_map::Keys<String, fn() -> Box<dyn NewIsInstrument>> {
        self.instruments.keys()
    }
}

#[derive(Debug)]
enum PaletteAction {
    NewController(String),
    NewEffect(String),
    NewInstrument(String),
}
#[derive(Debug)]
struct PalettePanel {
    factory: Arc<EntityFactory>,
}
impl Shows for PalettePanel {
    fn show(&mut self, ui: &mut egui::Ui) {
        for name in &self.factory.keys {
            ui.label(name.to_string());
        }
    }
}
impl PalettePanel {
    pub fn new_with(factory: Arc<EntityFactory>) -> Self {
        Self { factory }
    }

    fn show_with_action(&mut self, ui: &mut egui::Ui) -> Option<PaletteAction> {
        let mut action = None;
        for name in self.factory.controller_keys() {
            if ui.button(name.to_string()).clicked() {
                action = Some(PaletteAction::NewController(name.to_string()));
            };
        }
        for name in self.factory.effect_keys() {
            if ui.button(name.to_string()).clicked() {
                action = Some(PaletteAction::NewEffect(name.to_string()));
            };
        }
        for name in self.factory.instrument_keys() {
            if ui.button(name.to_string()).clicked() {
                action = Some(PaletteAction::NewInstrument(name.to_string()));
            };
        }
        action
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

    toasts: Toasts,
}
impl MiniDaw {
    pub fn new(cc: &CreationContext) -> Self {
        let mut factory = EntityFactory::default();
        Self::register_entities(&mut factory);
        let factory = Arc::new(factory);

        let orchestrator_panel = OrchestratorPanel::new_with(Arc::clone(&factory));
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
            palette_panel: PalettePanel::new_with(factory),

            toasts: Toasts::new()
                .anchor(Align2::RIGHT_BOTTOM, (-10.0, -10.0))
                .direction(egui::Direction::BottomUp),
        };
        r.spawn_channel_watcher(cc.egui_ctx.clone());
        r
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

        factory.register_controller("arpeggiator", || {
            Box::new(Arpeggiator::new_with(
                &ArpeggiatorParams::default(),
                MidiChannel::new(0),
            ))
        });
        factory.register_effect("reverb", || {
            Box::new(Reverb::new_with(&ReverbParams::default()))
        });
        factory.register_instrument("toy-synth", || {
            Box::new(ToySynth::new_with(&ToySynthParams::default()))
        });
        factory.register_instrument("toy-instrument", || {
            Box::new(ToyInstrument::new_with(&ToyInstrumentParams::default()))
        });
        factory.register_instrument("welsh-synth", || {
            Box::new(WelshSynth::new_with(&WelshSynthParams::default()))
        });
    }

    fn handle_palette_action(&mut self, action: PaletteAction) {
        if let Ok(mut o) = self.mini_orchestrator.lock() {
            match action {
                PaletteAction::NewController(key) => {
                    if let Some(controller) = self.factory.new_controller(key.as_str()) {
                        o.add_controller(controller);
                    }
                }
                PaletteAction::NewEffect(key) => {
                    if let Some(effect) = self.factory.new_effect(key.as_str()) {
                        o.add_effect(effect);
                    }
                }
                PaletteAction::NewInstrument(key) => {
                    if let Some(instrument) = self.factory.new_instrument(key.as_str()) {
                        o.add_instrument(instrument);
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
                ui.label(RichText::new(format!("Build: {:?}", app_version())))
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
        if let Ok(mut o) = self.mini_orchestrator.lock() {
            o.show(ui);
        }
    }
}
impl eframe::App for MiniDaw {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_message_channels();
        let top = egui::TopBottomPanel::top("top-panel")
            .resizable(false)
            .exact_height(64.0);
        let bottom = egui::TopBottomPanel::bottom("bottom-panel")
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
