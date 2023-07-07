use eframe::egui::{Id as EguiId, Ui};
use groove_core::traits::gui::Shows;
use std::sync::{Arc, Mutex};

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
    drag_drop_manager: Arc<Mutex<DragDropManager>>,
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
    pub fn new_with(
        factory: Arc<EntityFactory>,
        drag_drop_manager: Arc<Mutex<DragDropManager>>,
    ) -> Self {
        Self {
            factory,
            drag_drop_manager,
        }
    }

    /// Draws the panel.
    pub fn show_with_action(&mut self, ui: &mut Ui) -> Option<PaletteAction> {
        let mut action = None;
        if let Ok(mut dnd) = self.drag_drop_manager.lock() {
            for key in self.factory.keys().iter() {
                dnd.drag_source(
                    ui,
                    EguiId::new(key),
                    DragDropSource::NewController(key.clone()),
                    |ui| {
                        if ui.button(key.to_string()).clicked() {
                            action = Some(PaletteAction::NewThing(key.clone()));
                        }
                    },
                );
            }
        }
        action
    }
}
