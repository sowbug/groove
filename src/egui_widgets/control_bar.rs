// Copyright (c) 2023 Mike Tsao. All rights reserved.

use std::{ops::Range, path::Path};

use eframe::egui::{self, DragValue};
use groove_core::{time::PerfectTimeUnit, traits::Performs};
use groove_orchestration::Orchestrator;

/// [ControlBar] displays the top bar of controls and global information.
#[derive(Debug, Default)]
pub struct ControlBar {}
impl ControlBar {
    /// Draws the bar
    pub fn show(&self, ui: &mut egui::Ui, orchestrator: &mut Orchestrator) {
        ui.horizontal(|ui| {
            let mut bpm = orchestrator.bpm();
            let mut is_loop_enabled = orchestrator.is_loop_enabled();
            let (mut loop_range_start, mut loop_range_end) =
                if let Some(range) = orchestrator.loop_range() {
                    (range.start, range.end)
                } else {
                    (PerfectTimeUnit(0.0), PerfectTimeUnit(0.0))
                };
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
            if ui.button("load (BROKEN)").clicked() {
                let s =
                    std::fs::read_to_string(Path::new("/home/miket/orchestrator-serialized.yaml"));
                if let Ok(contents) = s {
                    if let Ok(new_orchestrator) = serde_yaml::from_str(&contents) {
                        *orchestrator = new_orchestrator;
                    }
                }
            }

            if ui.button("save").clicked() {
                let s = serde_yaml::to_string(orchestrator);
                if let Ok(contents) = s {
                    let _ = std::fs::write(
                        Path::new("/home/miket/orchestrator-serialized.yaml"),
                        contents,
                    );
                }
            }

            let clock = orchestrator.clock();
            let minutes: u8 = (clock.seconds() / 60.0).floor() as u8;
            let seconds = clock.seconds() as usize % 60;
            let thousandths = (clock.seconds().fract() * 1000.0) as u16;
            ui.label(format!("{minutes:03}:{seconds:02}:{thousandths:03}"));

            if ui.checkbox(&mut is_loop_enabled, "Loop").changed() {
                orchestrator.set_loop_enabled(is_loop_enabled);
            }
            ui.add_enabled_ui(is_loop_enabled, |ui| {
                let mut changed = false;
                let (mut loop_start_text, mut loop_end_text) = (
                    format!("{}", loop_range_start),
                    format!("{}", loop_range_end),
                );
                if ui.text_edit_singleline(&mut loop_start_text).changed() {
                    if let Ok(v) = loop_start_text.parse::<f64>() {
                        changed = true;
                        loop_range_start = PerfectTimeUnit(v);
                    }
                };
                if ui.text_edit_singleline(&mut loop_end_text).changed() {
                    if let Ok(v) = loop_end_text.parse::<f64>() {
                        changed = true;
                        loop_range_end = PerfectTimeUnit(v);
                    }
                };
                if changed {
                    eprintln!("changing {} {}", loop_range_start, loop_range_end);
                    orchestrator.set_loop(&Range {
                        start: loop_range_start,
                        end: loop_range_end,
                    });
                }
            });
        });
    }
}
