// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::panels::Preferences;
use crossbeam_channel::{Receiver, Sender};
use eframe::egui::{CollapsingHeader, Ui};
use groove_orchestration::Orchestrator;
use groove_utils::Paths;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

/// The browser provides updates to the app through [EntityBrowserEvent] messages.
#[derive(Debug)]
pub enum EntityBrowserEvent {
    /// A new project was loaded. Filename provided.
    ProjectLoaded(Result<PathBuf, anyhow::Error>),
}

#[derive(Clone, Copy, PartialEq)]
pub enum Action {
    Keep,
}

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
enum EntityType {
    #[default]
    Top,
    Directory(PathBuf),
    Project(PathBuf),
    Sample,
    Patch,
}

/// [EntityBrowser] shows assets in a tree view.
#[derive(Clone, Debug)]
pub struct EntityBrowser {
    app_receiver: Receiver<EntityBrowserEvent>, // to give to the app to receive what we sent
    app_sender: Sender<EntityBrowserEvent>,     // for us to send to the app
    root: EntityBrowserNode,
}
impl EntityBrowser {
    /// Instantiates a new top-level [EntityBrowser] and scans global/user/dev
    /// directories. TODO: this is synchronous
    pub fn scan_everything(paths: &Paths, extra_paths: Vec<PathBuf>) -> Self {
        let mut root = EntityBrowserNode {
            thing_type: EntityType::Top,
            ..Default::default()
        };
        for path in paths.hives() {
            eprintln!("Scanning hive {}", path.display());
            root.top_scan(path, path.display().to_string().as_str());
        }
        for path in extra_paths {
            eprintln!("Scanning extra path {}", path.display());
            root.top_scan(&path, path.display().to_string().as_str());
        }
        let (app_sender, app_receiver) = crossbeam_channel::unbounded();
        Self {
            app_receiver,
            app_sender,
            root,
        }
    }

    /// Renders the entity browser.
    pub fn show(&mut self, ui: &mut Ui, paths: &Paths, orchestrator: Arc<Mutex<Orchestrator>>) {
        self.root
            .ui_impl(ui, paths, self.app_sender.clone(), orchestrator);
    }

    /// The receive side of the [EntityBrowserEvent] channel.
    pub fn receiver(&self) -> &Receiver<EntityBrowserEvent> {
        &self.app_receiver
    }
}

/// [EntityBrowser] shows assets in a tree view.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct EntityBrowserNode {
    depth: usize,
    thing_type: EntityType,
    name: String,
    children: Vec<EntityBrowserNode>,
}
impl EntityBrowserNode {
    fn top_scan(&mut self, path: &Path, title: &str) {
        let mut child = self.make_child();
        child.scan(path);
        child.name = title.to_string();
        self.children.push(child);
    }

    fn make_child(&mut self) -> Self {
        EntityBrowserNode {
            depth: self.depth + 1,
            ..Default::default()
        }
    }

    fn scan(&mut self, path: &Path) {
        if !path.exists() {
            return;
        }
        self.name = path.file_name().unwrap().to_str().unwrap().to_string();
        if path.is_file() {
            if let Some(extension) = path.extension() {
                let extension = extension.to_ascii_lowercase();
                if extension == "ens" || extension == "json" || extension == "json5" {
                    self.thing_type = EntityType::Project(path.to_path_buf());
                }
                if extension == "wav" || extension == "aiff" {
                    self.thing_type = EntityType::Sample;
                }
                if extension == "enp" {
                    self.thing_type = EntityType::Patch;
                }
            }
            return;
        }
        if path.is_dir() {
            self.thing_type = EntityType::Directory(path.to_path_buf());
            if let Ok(read_dir) = fs::read_dir(path) {
                for entry in read_dir.flatten() {
                    let mut child = self.make_child();
                    child.scan(&entry.path());
                    self.children.push(child);
                }
            }
        }
    }

    fn show(
        &mut self,
        ui: &mut Ui,
        paths: &Paths,
        sender: Sender<EntityBrowserEvent>,
        orchestrator: Arc<Mutex<Orchestrator>>,
    ) {
        self.ui_impl(ui, paths, sender, orchestrator);
    }

    fn ui_impl(
        &mut self,
        ui: &mut Ui,
        paths: &Paths,
        sender: Sender<EntityBrowserEvent>,
        orchestrator: Arc<Mutex<Orchestrator>>,
    ) -> Action {
        match &self.thing_type {
            EntityType::Top => self.children_ui(ui, paths, sender, orchestrator),
            EntityType::Directory(_path) => CollapsingHeader::new(&self.name)
                .id_source(ui.next_auto_id())
                .default_open(self.depth < 2)
                .show(ui, |ui| self.children_ui(ui, paths, sender, orchestrator))
                .body_returned
                .unwrap_or(Action::Keep),
            EntityType::Project(path) => {
                ui.horizontal(|ui| {
                    if ui.button("Load").clicked() {
                        let _ = sender.send(EntityBrowserEvent::ProjectLoaded(
                            Preferences::handle_load(paths, &path.clone(), orchestrator),
                        ));
                    }
                    ui.label(format!("Project {}", self.name));
                });
                Action::Keep
            }
            EntityType::Sample => {
                ui.label(format!("Sample {}", self.name));
                Action::Keep
            }
            EntityType::Patch => {
                ui.label(format!("Patch {}", self.name));
                Action::Keep
            }
        }
    }

    fn children_ui(
        &mut self,
        ui: &mut Ui,
        paths: &Paths,
        sender: Sender<EntityBrowserEvent>,
        orchestrator: Arc<Mutex<Orchestrator>>,
    ) -> Action {
        for child in self.children.iter_mut() {
            child.show(ui, paths, sender.clone(), Arc::clone(&orchestrator));
        }

        Action::Keep
    }
}
