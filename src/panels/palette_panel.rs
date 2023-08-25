use eframe::egui::{Id as EguiId, Ui};
use groove_core::traits::gui::Shows;

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
#[derive(Debug, Default)]
pub struct PalettePanel {}
impl Shows for PalettePanel {
    fn show(&mut self, ui: &mut Ui) {
        for name in EntityFactory::global().keys() {
            ui.label(name.to_string());
        }
    }
}
impl PalettePanel {
    /// Draws the panel.
    pub fn show_with_action(
        &mut self,
        ui: &mut Ui,
        ddm: &mut DragDropManager,
    ) -> Option<PaletteAction> {
        let action = None;
        for key in EntityFactory::global().sorted_keys() {
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
