// Copyright (c) 2023 Mike Tsao. All rights reserved.

use eframe::egui::{CollapsingHeader, Ui};
use groove_core::traits::Resets;
use groove_orchestration::Orchestrator;
use groove_settings::SongSettings;
use groove_utils::{PathType, Paths};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use strum::IntoEnumIterator;

#[derive(Clone, Copy, PartialEq)]
pub enum Action {
    Keep,
    Delete,
}

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
enum ThingType {
    #[default]
    Top,
    Directory(PathBuf),
    Project(PathBuf),
    Sample,
    Patch,
}

/// [ThingBrowser] shows assets in a tree view.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ThingBrowser {
    depth: usize,
    thing_type: ThingType,
    name: String,
    children: Vec<ThingBrowser>,
}
impl ThingBrowser {
    /// Instantiates a new top-level [ThingBrowser] and scans global/user/dev
    /// directories. TODO: this is synchronous
    pub fn scan_everything() -> Self {
        let mut r = ThingBrowser::default();
        r.thing_type = ThingType::Top;
        for path_type in PathType::iter() {
            r.top_scan(&Paths::assets_path(&path_type), path_type.into());
        }
        r
    }

    pub fn show(&mut self, ui: &mut eframe::egui::Ui, orchestrator: Arc<Mutex<Orchestrator>>) {
        self.ui_impl(ui, orchestrator);
    }

    fn top_scan(&mut self, path: &Path, title: &str) {
        let mut child = self.make_child();
        child.scan(path);
        child.name = title.to_string();
        self.children.push(child);
    }

    fn make_child(&mut self) -> Self {
        let mut child = ThingBrowser::default();
        child.depth = self.depth + 1;
        child
    }

    fn scan(&mut self, path: &Path) {
        if !path.exists() {
            return;
        }
        self.name = path.file_name().unwrap().to_str().unwrap().to_string();
        if path.is_file() {
            if let Some(extension) = path.extension() {
                let extension = extension.to_ascii_lowercase();
                if extension == "yaml" || extension == "yml" || extension == "ens" {
                    self.thing_type = ThingType::Project(path.to_path_buf());
                }
                if extension == "wav" || extension == "aiff" {
                    self.thing_type = ThingType::Sample;
                }
                if extension == "enp" {
                    self.thing_type = ThingType::Patch;
                }
            }
            return;
        }
        if path.is_dir() {
            self.thing_type = ThingType::Directory(path.to_path_buf());
            if let Ok(read_dir) = fs::read_dir(path) {
                for entry in read_dir {
                    if let Ok(entry) = entry {
                        let mut child = self.make_child();
                        child.scan(&entry.path());
                        self.children.push(child);
                    }
                }
            }
        }
    }

    fn ui_impl(&mut self, ui: &mut Ui, orchestrator: Arc<Mutex<Orchestrator>>) -> Action {
        match &self.thing_type {
            ThingType::Top => self.children_ui(ui, orchestrator),
            ThingType::Directory(path) => CollapsingHeader::new(&self.name)
                .id_source(ui.next_auto_id())
                .default_open(self.depth < 1)
                .show(ui, |ui| self.children_ui(ui, orchestrator))
                .body_returned
                .unwrap_or(Action::Keep),
            ThingType::Project(path) => {
                ui.horizontal(|ui| {
                    if ui.button("Load").clicked() {
                        Self::handle_load(orchestrator, &path.clone());
                    }
                    ui.label(format!("Project {}", self.name));
                });
                Action::Keep
            }
            ThingType::Sample => {
                ui.label(format!("Sample {}", self.name));
                Action::Keep
            }
            ThingType::Patch => {
                ui.label(format!("Patch {}", self.name));
                Action::Keep
            }
        }
    }

    fn children_ui(&mut self, ui: &mut Ui, orchestrator: Arc<Mutex<Orchestrator>>) -> Action {
        for child in self.children.iter_mut() {
            child.show(ui, Arc::clone(&orchestrator));
        }

        Action::Keep
    }

    fn handle_load(orchestrator: Arc<Mutex<Orchestrator>>, path: &Path) {
        match SongSettings::new_from_yaml_file(path) {
            Ok(s) => match s.instantiate(&Paths::assets_path(&PathType::Dev), false) {
                Ok(instance) => {
                    if let Ok(mut o) = orchestrator.lock() {
                        let sample_rate = o.sample_rate();
                        *o = instance;
                        o.reset(sample_rate);
                    }
                }
                Err(err) => eprintln!("instantiate: {}", err),
            },
            Err(err) => eprintln!("new_from_yaml: {}", err),
        }
    }
}
