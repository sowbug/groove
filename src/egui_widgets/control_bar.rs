// Copyright (c) 2023 Mike Tsao. All rights reserved.

use eframe::egui::{self, DragValue};
use groove_core::traits::Performs;
use groove_orchestration::Orchestrator;

/// [ControlBar] displays the top bar of controls and global information.
#[derive(Debug, Default)]
pub struct ControlBar {}
impl ControlBar {
    /// Draws the bar
    pub fn show(&self, ui: &mut egui::Ui, orchestrator: &mut Orchestrator) {
        ui.horizontal(|ui| {
            let mut bpm = orchestrator.bpm();
            if ui
                .add(
                    DragValue::new(&mut bpm)
                        .speed(0.1)
                        .suffix(" BPM")
                        .fixed_decimals(2),
                )
                .changed()
            {
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
