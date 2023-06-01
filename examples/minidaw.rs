// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crossbeam_channel::{Receiver, Select, Sender};
use eframe::{
    egui::{self, Context},
    CreationContext,
};
use groove::egui_widgets::{
    AudioPanel2, AudioPanelEvent, ControlBar2, ControlBarAction, MidiPanel, MidiPanelEvent,
    NeedsAudioFn,
};
use groove_core::{
    generators::{EnvelopeParams, Waveform},
    midi::{MidiChannel, MidiMessage},
    time::{SampleRate, Tempo, TimeSignature},
    traits::{gui::Shows, IsController, IsEffect, IsInstrument},
};
use groove_entities::{instruments::WelshSynth, EntityMessage};
use groove_toys::{ToySynth, ToySynthParams};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    hash::Hash,
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

#[derive(Serialize, Deserialize, Debug, Default, Eq, PartialEq, Hash)]
struct Id(usize);

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
    Quit,
}

#[derive(Clone, Debug)]
enum MiniOrchestratorEvent {
    Tempo(Tempo),
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
    orchestrator: MiniOrchestrator,

    input_channel_pair: ChannelPair<MiniOrchestratorInput>,
    event_channel_pair: ChannelPair<MiniOrchestratorEvent>,
}
impl Default for OrchestratorPanel {
    fn default() -> Self {
        let mut r = Self {
            orchestrator: Default::default(),
            input_channel_pair: Default::default(),
            event_channel_pair: Default::default(),
        };
        r.start_thread();
        r
    }
}
impl OrchestratorPanel {
    fn start_thread(&mut self) {
        let receiver = self.input_channel_pair.receiver.clone();
        let sender = self.event_channel_pair.sender.clone();
        std::thread::spawn(move || loop {
            if let Ok(input) = receiver.recv() {
                match input {
                    MiniOrchestratorInput::Midi(channel, message) => todo!(),
                    MiniOrchestratorInput::Play => todo!(),
                    MiniOrchestratorInput::Stop => todo!(),
                    MiniOrchestratorInput::Quit => break,
                }
            } else {
                eprintln!("unexpected failure of Orchestrator channel");
                break;
            }
        });
    }

    fn sender(&self) -> &Sender<MiniOrchestratorInput> {
        &self.input_channel_pair.sender
    }

    fn receiver(&self) -> &Receiver<MiniOrchestratorEvent> {
        &self.event_channel_pair.receiver
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct MiniOrchestrator {
    time_signature: TimeSignature,
    tempo: Tempo,

    #[serde(skip)]
    sample_rate: SampleRate,

    controllers: HashMap<Id, Box<dyn NewIsController>>,
    instruments: HashMap<Id, Box<dyn NewIsInstrument>>,
    effects: HashMap<Id, Box<dyn NewIsEffect>>,
}
impl Default for MiniOrchestrator {
    fn default() -> Self {
        let r = Self {
            time_signature: Default::default(),
            tempo: Default::default(),
            sample_rate: Default::default(),
            controllers: Default::default(),
            instruments: Default::default(),
            effects: Default::default(),
        };
        r.broadcast_initial_state();
        r
    }
}
impl MiniOrchestrator {
    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    fn set_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
    }

    fn tempo(&self) -> Tempo {
        self.tempo
    }

    fn set_tempo(&mut self, tempo: Tempo) {
        self.tempo = tempo;
    }

    fn do_something(&mut self) {
        eprintln!("I'm here!")
    }

    fn broadcast(&self, event: MiniOrchestratorEvent) {
        let _ = self.event_channel_pair.sender.send(event);
    }

    fn broadcast_initial_state(&self) {
        self.broadcast_tempo();
    }

    fn broadcast_tempo(&self) {
        self.broadcast(MiniOrchestratorEvent::Tempo(self.tempo));
    }
}
impl Shows for MiniOrchestrator {
    fn show(&mut self, ui: &mut egui::Ui) {
        ui.label(format!(
            "I have {} controllers, {} instruments, and {} effects",
            self.controllers.len(),
            self.instruments.len(),
            self.effects.len()
        ));
    }
}

struct MiniDaw {
    mini_orchestrator: Arc<Mutex<MiniOrchestrator>>,
    mini_orchestrator_sender: Sender<MiniOrchestratorInput>,
    mini_orchestrator_receiver: Receiver<MiniOrchestratorEvent>,
    control_bar: ControlBar2,
    audio_panel: AudioPanel2,
    midi_panel: MidiPanel,
}
impl MiniDaw {
    pub fn new(cc: &CreationContext) -> Self {
        let filename = "minidaw.json";
        let mut mini_orchestrator = if let Ok(s) = std::fs::read_to_string(filename) {
            if let Ok(mo) = serde_json::from_str(&s) {
                mo
            } else {
                MiniOrchestrator::default()
            }
        } else {
            MiniOrchestrator::default()
        };
        if mini_orchestrator.instruments.is_empty() {
            mini_orchestrator.instruments.insert(
                Id(3),
                Box::new(ToySynth::new_with(&&ToySynthParams {
                    voice_count: 1,
                    waveform: Waveform::Sine,
                    envelope: EnvelopeParams::safe_default(),
                })),
            );
        }
        if let Ok(s) = serde_json::to_string(&mini_orchestrator) {
            let _ = std::fs::write(filename, s);
        }

        let mini_orchestrator_sender = mini_orchestrator.sender().clone();
        let mini_orchestrator_receiver = mini_orchestrator.receiver().clone();
        let mini_orchestrator = Arc::new(Mutex::new(mini_orchestrator));

        let mo2 = Arc::clone(&mini_orchestrator);
        let needs_audio: NeedsAudioFn = Box::new(move || {
            if let Ok(mut o) = mo2.lock() {
                o.do_something();
            }
        });

        let mut r = Self {
            mini_orchestrator: Arc::clone(&mini_orchestrator),
            mini_orchestrator_sender,
            mini_orchestrator_receiver,
            control_bar: ControlBar2::default(),
            audio_panel: AudioPanel2::new_with(Box::new(needs_audio)),
            midi_panel: MidiPanel::default(),
        };
        r.spawn_channel_watcher(cc.egui_ctx.clone());
        r
    }

    fn tell_orchestrator(&self, message: MiniOrchestratorInput) {
        let _ = self.mini_orchestrator_sender.send(message);
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
                    self.tell_orchestrator(MiniOrchestratorInput::Midi(channel, message));
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
        if let Ok(m) = self.mini_orchestrator_receiver.try_recv() {
            match m {
                MiniOrchestratorEvent::Tempo(tempo) => {
                    self.control_bar.set_tempo(tempo);
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
        let r3 = self.mini_orchestrator_receiver.clone();
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

    fn handle_control_bar_action(&mut self, action: ControlBarAction) {
        match action {
            ControlBarAction::Play => self.tell_orchestrator(MiniOrchestratorInput::Play),
            ControlBarAction::Stop => self.tell_orchestrator(MiniOrchestratorInput::Stop),
        }
    }
}
impl eframe::App for MiniDaw {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_message_channels();
        let top = egui::TopBottomPanel::top("top");
        let center = egui::CentralPanel::default();
        top.show(ctx, |ui| {
            if let Some(action) = self.control_bar.show(ui) {
                self.handle_control_bar_action(action);
            }
        });
        center.show(ctx, |ui| {
            self.audio_panel.show(ui);
            self.midi_panel.show(ui);
            if let Ok(mut o) = self.mini_orchestrator.lock() {
                o.show(ui);
            }
        });
    }
}

#[typetag::serde]
impl NewIsInstrument for WelshSynth {}
#[typetag::serde]
impl NewIsInstrument for ToySynth {}

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
