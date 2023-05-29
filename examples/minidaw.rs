// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crossbeam_channel::Select;
use eframe::{
    egui::{self, Context},
    CreationContext,
};
use groove::egui_widgets::{AudioPanel, ControlBar, MidiPanel, MidiPanelEvent};
use groove_core::{
    generators::{EnvelopeParams, Waveform},
    time::ClockParams,
    traits::gui::Shows,
};
use groove_orchestration::{Entity, Orchestrator};
use groove_toys::{ToySynth, ToySynthParams};
use std::sync::{Arc, Mutex};

struct MiniDaw {
    orchestrator: Arc<Mutex<Orchestrator>>,
    control_bar: ControlBar,
    audio_panel: AudioPanel,
    midi_panel: MidiPanel,
}
impl MiniDaw {
    pub fn new(cc: &CreationContext) -> Self {
        let clock_params = ClockParams {
            bpm: 128.0,
            midi_ticks_per_second: 960,
            time_signature: groove_core::time::TimeSignatureParams { top: 4, bottom: 4 },
        };
        let mut orchestrator = Orchestrator::new_with(&clock_params);
        let synth = ToySynth::new_with(&ToySynthParams {
            voice_count: 1,
            waveform: Waveform::Sine,
            envelope: EnvelopeParams::safe_default(),
        });
        let uid = orchestrator.add(Entity::ToySynth(Box::new(synth)));
        let _ = orchestrator.patch_chain_to_main_mixer(&[uid]);
        orchestrator.connect_midi_downstream(uid, 0);

        let orchestrator = Arc::new(Mutex::new(orchestrator));
        let mut r = Self {
            orchestrator: Arc::clone(&orchestrator),
            control_bar: ControlBar::default(),
            audio_panel: AudioPanel::new_with(Arc::clone(&orchestrator)),
            midi_panel: MidiPanel::default(),
        };
        r.spawn_channel_watcher(cc.egui_ctx.clone());
        r
    }

    fn handle_message_channels(&mut self) {
        loop {
            let mut received_any = false;
            if let Ok(m) = self.midi_panel.receiver().try_recv() {
                received_any = true;
                match m {
                    MidiPanelEvent::Midi(channel, message) => {
                        if let Ok(mut o) = self.orchestrator.lock() {
                            let _ = o.update(
                                groove_orchestration::messages::GrooveInput::MidiFromExternal(
                                    channel, message,
                                ),
                            );
                        }
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
            }
            if !received_any {
                break;
            }
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
        let _ = std::thread::spawn(move || {
            let mut sel = Select::new();
            let _ = sel.recv(&r1);
            loop {
                let _ = sel.ready();
                ctx.request_repaint();
            }
        });
    }
}
impl eframe::App for MiniDaw {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_message_channels();
        let top = egui::TopBottomPanel::top("top");
        let center = egui::CentralPanel::default();
        if let Ok(mut o) = self.orchestrator.lock() {
            top.show(ctx, |ui| self.control_bar.show(ui, &mut o));
            center.show(ctx, |ui| {
                self.audio_panel.show(ui);
                self.midi_panel.show(ui);
                o.show(ui);
                ui.label(format!("BPM is {}", o.clock().bpm()));
            });
        }
    }
}

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
