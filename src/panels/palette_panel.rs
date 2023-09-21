use eframe::egui::{Id as EguiId, Ui};
use ensnare::traits::{
    Configurable, ControlEventsFn, Controllable, Controls, Displays, DisplaysInTimeline, Entity,
    EntityEvent, Generates, GeneratesToInternalBuffer, HandlesMidi, HasSettings, HasUid,
    Serializable, Ticks,
};

use crate::mini::{
    {DragDropManager, DragDropSource}, {EntityFactory, Key},
};

/// Actions that [PalettePanel] can generate.
#[derive(Debug)]
pub enum PaletteAction {
    /// Requests a new entity of type [Key].
    NewEntity(Key),
}

/// A tree view of devices that can be placed in tracks.
#[derive(Debug, Default)]
pub struct PalettePanel {}
impl Displays for PalettePanel {
    fn ui(&mut self, ui: &mut Ui) -> eframe::egui::Response {
        ui.vertical(|ui| {
            for name in EntityFactory::global().keys() {
                ui.label(name.to_string());
            }
        })
        .response
    }
}
impl PalettePanel {
    /// Draws the panel.
    pub fn show_with_action(&mut self, ui: &mut Ui) -> Option<PaletteAction> {
        let action = None;
        for key in EntityFactory::global().sorted_keys() {
            DragDropManager::drag_source(
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
