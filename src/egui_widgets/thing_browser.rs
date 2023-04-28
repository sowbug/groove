// Copyright (c) 2023 Mike Tsao. All rights reserved.

use eframe::egui::{CollapsingHeader, RichText, Ui};
use groove_core::traits::Shows;

#[derive(Clone, Copy, PartialEq)]
pub enum Action {
    Keep,
    Delete,
}

/// [ThingBrowser] shows assets in a tree view.
#[derive(Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ThingBrowser(Vec<ThingBrowser>);
impl Shows for ThingBrowser {
    fn show(&mut self, ui: &mut eframe::egui::Ui) {
        self.ui_impl(ui, 0, "samples");
    }
}
impl ThingBrowser {
    /// This will scan the assets and build the tree.
    pub fn demo() -> Self {
        // TODO: scan the assets and project dirs
        Self(vec![
            ThingBrowser(vec![ThingBrowser::default(); 4]),
            ThingBrowser(vec![ThingBrowser(vec![ThingBrowser::default(); 2]); 3]),
        ])
    }

    fn ui_impl(&mut self, ui: &mut Ui, depth: usize, name: &str) -> Action {
        CollapsingHeader::new(name)
            .default_open(depth < 1)
            .show(ui, |ui| self.children_ui(ui, depth))
            .body_returned
            .unwrap_or(Action::Keep)
    }

    fn children_ui(&mut self, ui: &mut Ui, depth: usize) -> Action {
        if depth > 0
            && ui
                .button(RichText::new("delete").color(ui.visuals().warn_fg_color))
                .clicked()
        {
            return Action::Delete;
        }

        self.0 = std::mem::take(self)
            .0
            .into_iter()
            .enumerate()
            .filter_map(|(i, mut tree)| {
                if tree.ui_impl(ui, depth + 1, &format!("child #{}", i)) == Action::Keep {
                    Some(tree)
                } else {
                    None
                }
            })
            .collect();

        if ui.button("+").clicked() {
            self.0.push(ThingBrowser::default());
        }

        Action::Keep
    }
}
