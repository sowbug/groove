// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! The egui app is an [egui](https://github.com/emilk/egui)-based DAW.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::{self};
use groove::egui_widgets::{AudioPanel, ControlBar, MidiPanel, ThingBrowser};
use groove_core::{
    time::ClockNano,
    traits::gui::{Shows, ShowsTopLevel},
};
use groove_orchestration::Orchestrator;
use std::sync::{Arc, Mutex};

fn main() -> Result<(), eframe::Error> {
    env_logger::init();
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1920.0, 1080.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Groove (egui)",
        options,
        Box::new(|_cc| Box::<GrooveApp>::default()),
    )
}

struct GrooveApp {
    orchestrator: Arc<Mutex<Orchestrator>>,

    control_bar: ControlBar,
    audio_panel: AudioPanel,
    midi_panel: MidiPanel,

    thing_browser: ThingBrowser,
}
impl Default for GrooveApp {
    fn default() -> Self {
        let clock_settings = ClockNano::default();
        let orchestrator = Arc::new(Mutex::new(Orchestrator::new_with(clock_settings)));

        Self {
            orchestrator: Arc::clone(&orchestrator),

            control_bar: ControlBar::default(),
            audio_panel: AudioPanel::new_with(Arc::clone(&orchestrator)),
            midi_panel: MidiPanel::new_with(),
            thing_browser: ThingBrowser::scan_everything(),
        }
    }
}

impl eframe::App for GrooveApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
        });
        left.show(ctx, |ui| {
            self.thing_browser.show(ui, Arc::clone(&self.orchestrator));
        });
        center.show(ctx, |ui| {
            if let Ok(mut o) = self.orchestrator.lock() {
                o.show(ui);
            }
            self.midi_panel.show(ctx);
            self.audio_panel.show(ctx);
        });

        // TODO: this is how to keep redrawing when the system doesn't otherwise
        // know that a repaint is needed. This is fine for now, but it's
        // expensive, and we should be smarter about it.
        ctx.request_repaint();
    }
}
