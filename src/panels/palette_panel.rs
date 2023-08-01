use eframe::egui::{Id as EguiId, Ui};
use groove_core::traits::gui::Shows;
use std::sync::Arc;

use crate::mini::{
    {DragDropManager, DragDropSource}, {EntityFactory, Key},
};

/// Actions that [PalettePanel] can generate.
#[derive(Debug)]
pub enum PaletteAction {
    /// Requests a new entity of type [Key].
    NewThing(Key),
}

/// A tree view of devices that can be placed in tracks.
#[derive(Debug)]
pub struct PalettePanel {
    factory: Arc<EntityFactory>,
}
impl Shows for PalettePanel {
    fn show(&mut self, ui: &mut Ui) {
        for name in self.factory.keys() {
            ui.label(name.to_string());
        }
    }
}
impl PalettePanel {
    /// Creates a new [PalettePanel].
    pub fn new_with(factory: Arc<EntityFactory>) -> Self {
        Self { factory }
    }

    /// Draws the panel.
    pub fn show_with_action(
        &mut self,
        ui: &mut Ui,
        ddm: &mut DragDropManager,
    ) -> Option<PaletteAction> {
        let action = None;
        for key in self.factory.sorted_keys() {
            ddm.drag_source(
                ui,
                EguiId::new(key),
                DragDropSource::NewDevice(key.clone()),
                |ui| {
                    ui.label(key.to_string());
                },
            );
        }
        action
    }
}
