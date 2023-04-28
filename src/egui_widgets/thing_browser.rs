// Copyright (c) 2023 Mike Tsao. All rights reserved.

use eframe::egui::{CollapsingHeader, RichText, Ui};
use groove_core::traits::gui::Shows;
use groove_utils::{PathType, Paths};
use std::{fs, path::Path};
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
    Nothing,
    Directory,
    Project,
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
impl Shows for ThingBrowser {
    fn show(&mut self, ui: &mut eframe::egui::Ui) {
        self.ui_impl(ui);
    }
}
impl ThingBrowser {
    /// Instantiates a new top-level [ThingBrowser] and scans global/user/dev
    /// directories. TODO: this is synchronous
    pub fn scan_everything() -> Self {
        let mut r = ThingBrowser::default();
        r.thing_type = ThingType::Directory;
        for path_type in PathType::iter() {
            r.top_scan(&Paths::assets_path(&path_type), path_type.into());
        }
        r
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
            self.thing_type = ThingType::Nothing;
            return;
        }
        self.name = path.file_name().unwrap().to_str().unwrap().to_string();
        if path.is_file() {
            self.thing_type = ThingType::Nothing;
            if let Some(extension) = path.extension() {
                let extension = extension.to_ascii_lowercase();
                if extension == "yaml" || extension == "yml" || extension == "ens" {
                    self.thing_type = ThingType::Project;
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
            self.thing_type = ThingType::Directory;
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

    fn ui_impl(&mut self, ui: &mut Ui) -> Action {
        match self.thing_type {
            ThingType::Nothing => Action::Keep,
            ThingType::Directory => CollapsingHeader::new(&self.name)
                .id_source(ui.next_auto_id())
                .default_open(self.depth < 1)
                .show(ui, |ui| self.children_ui(ui))
                .body_returned
                .unwrap_or(Action::Keep),
            ThingType::Project => {
                ui.label(format!("Project {}", self.name));
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

    fn children_ui(&mut self, ui: &mut Ui) -> Action {
        if self.depth > 0
            && ui
                .button(RichText::new("delete").color(ui.visuals().warn_fg_color))
                .clicked()
        {
            return Action::Delete;
        }

        for child in self.children.iter_mut() {
            child.show(ui);
        }

        Action::Keep
    }
}
