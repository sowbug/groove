// Copyright (c) 2023 Mike Tsao. All rights reserved.

use eframe::egui::{CollapsingHeader, Ui};
use groove_orchestration::Orchestrator;
use groove_settings::SongSettings;
use groove_utils::Paths;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

/// User-specific preferences for the whole app
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Preferences {
    /// The last-selected MIDI input
    selected_midi_input: Option<String>,
    /// The last-selected MIDI output
    selected_midi_output: Option<String>,

    /// Whether we should reload the last-loaded project on startup
    should_reload_last_project: bool,

    /// The last-loaded project filename.
    last_project_filename: Option<PathBuf>,

    #[serde(skip)]
    is_saved: bool,
}
impl Preferences {
    /// Loads preferences from a well-known location, and creates a Preferences
    /// struct
    pub async fn load() -> anyhow::Result<Self, anyhow::Error> {
        use async_std::prelude::*;

        let mut contents = String::new();
        let mut file = async_std::fs::File::open(Paths::prefs_file())
            .await
            .map_err(|e| anyhow::format_err!("Couldn't open prefs file: {}", e))?;
        file.read_to_string(&mut contents)
            .await
            .map_err(|e| anyhow::format_err!("Couldn't read prefs file: {}", e))?;
        serde_json::from_str(&contents)
            .map_err(|e| anyhow::format_err!("Couldn't parse prefs file: {}", e))
    }

    /// Saves the current in-memory preferences
    async fn save(&mut self) -> anyhow::Result<(), anyhow::Error> {
        use async_std::prelude::*;

        let json = serde_json::to_string_pretty(&self)
            .map_err(|_| anyhow::format_err!("Unable to serialize prefs JSON"))?;
        let path = Paths::prefs_file();
        if let Some(dir) = path.parent() {
            async_std::fs::create_dir_all(dir).await.map_err(|e| {
                anyhow::format_err!("Unable to create prefs parent directories: {}", e)
            })?;
        }

        let mut file = async_std::fs::File::create(path)
            .await
            .map_err(|e| anyhow::format_err!("Unable to create prefs file: {}", e))?;

        file.write_all(json.as_bytes())
            .await
            .map_err(|e| anyhow::format_err!("Unable to write prefs file: {}", e))?;

        self.is_saved = true;
        Ok(())
    }

    // TODO: this might make more sense in Orchestrator, or maybe utils
    /// Loads the specified project file.
    pub fn handle_load(
        paths: &Paths,
        path: &Path,
        orchestrator: Arc<Mutex<Orchestrator>>,
    ) -> anyhow::Result<PathBuf, anyhow::Error> {
        match SongSettings::new_from_project_file(path) {
            Ok(s) => match s.instantiate(paths, false) {
                Ok(instance) => {
                    if let Ok(mut o) = orchestrator.lock() {
                        let sample_rate = o.sample_rate();
                        *o = instance;
                        o.update_sample_rate(sample_rate);
                    }
                    Ok(path.to_path_buf())
                }
                Err(err) => Err(anyhow::format_err!(
                    "Error while processing project file {}: {}",
                    path.display(),
                    err
                )),
            },
            Err(err) => Err(anyhow::format_err!(
                "Error while reading project file {}: {}",
                path.display(),
                err
            )),
        }
    }

    fn mark_dirty(&mut self) {
        if !self.is_saved && futures::executor::block_on(self.save()).is_ok() {
            self.is_saved = true;
        }
    }

    /// currently selected MIDI input
    pub fn selected_midi_input(&self) -> Option<&String> {
        self.selected_midi_input.as_ref()
    }

    /// currently selected MIDI output
    pub fn selected_midi_output(&self) -> Option<&String> {
        self.selected_midi_output.as_ref()
    }

    /// Set current MIDI input
    pub fn set_selected_midi_input(&mut self, selected_midi_input: &str) {
        self.selected_midi_input = Some(selected_midi_input.to_string());
        self.mark_dirty();
    }

    /// Set current MIDI output
    pub fn set_selected_midi_output(&mut self, selected_midi_output: &str) {
        self.selected_midi_output = Some(selected_midi_output.to_string());
        self.mark_dirty();
    }

    /// filename of most recently loaded project
    pub fn project_filename(&self) -> Option<&PathBuf> {
        self.last_project_filename.as_ref()
    }

    /// update most recently loaded project filename
    pub fn set_project_filename(&mut self, project_filename: &Path) {
        let should_update = if let Some(filename) = &self.last_project_filename {
            // We had one; is it different?
            filename.as_path() != project_filename
        } else {
            // We didn't have one, but we do now
            true
        };
        if should_update {
            self.last_project_filename = Some(project_filename.to_path_buf());
            self.mark_dirty();
        }
    }

    /// Whether to reload the last-loaded project on app start
    pub fn should_reload_last_project(&self) -> bool {
        self.should_reload_last_project
    }

    /// Set whether to reload the last-loaded project on app start
    pub fn set_should_reload_last_project(&mut self, should_reload_last_project: bool) {
        if self.should_reload_last_project != should_reload_last_project {
            self.should_reload_last_project = should_reload_last_project;
            self.mark_dirty();
        }
    }
}
impl Displays for Preferences {
    fn ui(&mut self, ui: &mut Ui) -> eframe::egui::Response {
        CollapsingHeader::new("General")
            .default_open(true)
            .show(ui, |ui| {
                if ui
                    .checkbox(
                        &mut self.should_reload_last_project,
                        "Load last project on startup",
                    )
                    .changed()
                {
                    self.mark_dirty();
                }
            })
            .header_response
    }
}
