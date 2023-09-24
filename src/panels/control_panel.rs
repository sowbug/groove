// Copyright (c) 2023 Mike Tsao. All rights reserved.

use eframe::egui::Ui;
use ensnare_core::{time::Transport, traits::Displays, widgets::core::transport};
use std::path::PathBuf;

/// Actions the user might take via the control panel.
pub enum ControlPanelAction {
    /// Play button pressed.
    Play,

    /// Stop button pressed.
    Stop,

    /// The user asked to create a new project.
    New,

    /// The user asked to load the project having the given filename.
    Open(PathBuf),

    /// The user asked to save the current project to the given filename.
    Save(PathBuf),

    /// The user pressed the settings icon.
    ToggleSettings,
}

/// [ControlPanel] is the UI component at the top of the main window. Transport,
/// MIDI status, etc.
#[derive(Debug, Default)]
pub struct ControlPanel {
    /// A local cached copy of [Transport].
    // TODO: this is awful. I think it goes away when we factor out ui() and
    // make it into a temp reference
    transport_copy: Transport,
}
impl ControlPanel {
    /// Renders the control bar and maybe returns a UI action.
    pub fn show_with_action(&mut self, ui: &mut Ui) -> Option<ControlPanelAction> {
        let mut action = None;
        ui.horizontal_centered(|ui| {
            ui.add(transport(&mut self.transport_copy));
            if ui.button("play").clicked() {
                action = Some(ControlPanelAction::Play);
            }
            if ui.button("stop").clicked() {
                action = Some(ControlPanelAction::Stop);
            }
            ui.separator();
            if ui.button("new").clicked() {
                action = Some(ControlPanelAction::New);
            }
            if ui.button("open").clicked() {
                action = Some(ControlPanelAction::Open(PathBuf::from("minidaw.json")));
            }
            if ui.button("save").clicked() {
                action = Some(ControlPanelAction::Save(PathBuf::from("minidaw.json")));
            }
            ui.separator();
            if ui.button("settings").clicked() {
                action = Some(ControlPanelAction::ToggleSettings);
            }
        });

        action
    }

    /// Updates the copy of [Transport] with a fresh one.
    pub fn set_transport(&mut self, transport: Transport) {
        self.transport_copy = transport;
    }
}
impl Displays for ControlPanel {
    fn ui(&mut self, ui: &mut Ui) -> eframe::egui::Response {
        let _ = self.show_with_action(ui);
        ui.label("TODO")
    }
}

#[cfg(obsolete)]
/// [ControlBar] displays the top bar of controls and global information.
#[derive(Debug, Default)]
pub struct ControlBar {}
#[cfg(obsolete)]
impl ControlBar {
    /// Draws the bar
    pub fn show(&self, ui: &mut Ui, orchestrator: &mut Orchestrator) {
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
                    std::fs::read_to_string(Path::new("/home/miket/orchestrator-serialized.json"));
                if let Ok(contents) = s {
                    if let Ok(new_orchestrator) = serde_json::from_str(&contents) {
                        *orchestrator = new_orchestrator;
                    }
                }
            }

            if ui.button("save").clicked() {
                let s = serde_json::to_string(orchestrator);
                if let Ok(contents) = s {
                    let _ = std::fs::write(
                        Path::new("/home/miket/orchestrator-serialized.json"),
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
                    orchestrator.set_loop(&Range {
                        start: loop_range_start,
                        end: loop_range_end,
                    });
                }
            });
        });
    }
}
