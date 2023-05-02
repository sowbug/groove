// Copyright (c) 2023 Mike Tsao. All rights reserved.

use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use eframe::egui::{self};
use groove_core::traits::{gui::Shows, Resets};
use groove_orchestration::Orchestrator;
use groove_settings::SongSettings;
use groove_utils::Paths;
use serde::{Deserialize, Serialize};

/// User-specific preferences for the whole app
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Preferences {
    /// The last-selected MIDI input
    selected_midi_input: Option<String>,
    /// The last-selected MIDI output
    selected_midi_output: Option<String>,

    /// The last-loaded project filename. Its presence in the Option indicates
    /// whether we should reload it on startup.
    last_project_filename: Option<String>,

    #[serde(skip)]
    current_project_filename: Arc<Mutex<String>>,

    #[serde(skip)]
    is_dirty: bool,
}
impl Preferences {
    /// Loads preferences from a well-known location, and creates a Preferences
    /// struct
    pub async fn load() -> anyhow::Result<Self, anyhow::Error> {
        use async_std::prelude::*;

        let mut contents = String::new();
        let mut file = async_std::fs::File::open(Paths::prefs_file())
            .await
            .map_err(|_| anyhow::format_err!("Couldn't open prefs file"))?;
        file.read_to_string(&mut contents)
            .await
            .map_err(|_| anyhow::format_err!("Couldn't read prefs file"))?;
        serde_json::from_str(&contents)
            .map_err(|_| anyhow::format_err!("Couldn't parse prefs file"))
    }

    // TODO: this might make more sense in Orchestrator, or maybe utils
    /// Loads the specified project file.
    pub fn handle_load(
        paths: &Paths,
        path: &Path,
        orchestrator: Arc<Mutex<Orchestrator>>,
    ) -> anyhow::Result<(), anyhow::Error> {
        match SongSettings::new_from_yaml_file(path) {
            Ok(s) => match s.instantiate(paths, false) {
                Ok(instance) => {
                    if let Ok(mut o) = orchestrator.lock() {
                        let sample_rate = o.sample_rate();
                        *o = instance;
                        o.reset(sample_rate);
                    }
                    Ok(())
                }
                Err(err) => Err(anyhow::format_err!(
                    "Error while processing project file {}: {}",
                    path.display(),
                    err
                )),
            },
            Err(err) => Err(anyhow::format_err!(
                "Error while reading YAML file {}: {}",
                path.display(),
                err
            )),
        }
    }

    #[doc(hidden)]
    pub fn selected_midi_input(&self) -> Option<&String> {
        self.selected_midi_input.as_ref()
    }

    #[doc(hidden)]
    pub fn selected_midi_output(&self) -> Option<&String> {
        self.selected_midi_output.as_ref()
    }

    #[doc(hidden)]
    pub fn last_project_filename(&self) -> Option<&String> {
        self.last_project_filename.as_ref()
    }

    #[doc(hidden)]
    pub fn set_selected_midi_input(&mut self, selected_midi_input: &str) {
        self.selected_midi_input = Some(selected_midi_input.to_string());
        self.is_dirty = true;
    }

    #[doc(hidden)]
    pub fn set_selected_midi_output(&mut self, selected_midi_output: &str) {
        self.selected_midi_output = Some(selected_midi_output.to_string());
        self.is_dirty = true;
    }

    #[doc(hidden)]
    pub fn set_current_project_filename(&mut self, current_project_filename: &str) {
        if let Ok(mut filename) = self.current_project_filename.lock() {
            *filename = current_project_filename.to_string();
            self.is_dirty = true;
        }
    }
}
impl Shows for Preferences {
    fn show(&mut self, ui: &mut egui::Ui) {
        let mut should_reload = self.last_project_filename.is_some();
        if ui
            .checkbox(&mut should_reload, "Load last project on startup")
            .changed()
        {
            self.last_project_filename = if should_reload {
                if let Ok(filename) = self.current_project_filename.lock() {
                    Some(filename.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        }
    }
}
