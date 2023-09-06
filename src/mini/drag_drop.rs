// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::{entity_factory::Key, piano_roll::PatternUid, TrackUid};
use eframe::{
    egui::{CursorIcon, Id as EguiId, InnerResponse, LayerId, Order, Sense, Ui},
    epaint::{self, Color32, Rect, Shape, Stroke, Vec2},
};
use groove_core::{time::MusicalTime, Uid};
use once_cell::sync::OnceCell;
use std::sync::Mutex;
use strum_macros::Display;

/// The one and only DragDropManager. Access it with `DragDropManager::global()`.
static DD_MANAGER: OnceCell<Mutex<DragDropManager>> = OnceCell::new();

#[allow(missing_docs)]
#[derive(Clone, Debug, Display, PartialEq, Eq)]
pub enum DragDropSource {
    NewDevice(Key),
    Pattern(PatternUid),
    ControlTrip(Uid),
}

#[allow(missing_docs)]
#[derive(Clone, Debug, Display)]
pub enum DragDropEvent {
    AddDeviceToTrack(Key, TrackUid),
    AddPatternToTrack(PatternUid, TrackUid, MusicalTime),
}

// TODO: a way to express rules about what can and can't be dropped
#[allow(missing_docs)]
#[derive(Debug, Default)]
pub struct DragDropManager {
    source: Option<DragDropSource>,
    events: Vec<DragDropEvent>,
}
#[allow(missing_docs)]
impl DragDropManager {
    /// Provides the one and only [DragDropManager].
    pub fn global() -> &'static Mutex<Self> {
        DD_MANAGER
            .get()
            .expect("DragDropManager has not been initialized")
    }

    pub fn reset() {
        Self::global().lock().unwrap().source = None;
    }

    pub fn enqueue_event(event: DragDropEvent) {
        Self::global().lock().unwrap().events.push(event);
    }

    pub fn take_and_clear_events() -> Vec<DragDropEvent> {
        let mut drag_drop_manager = Self::global().lock().unwrap();
        let events = drag_drop_manager.events.clone();
        drag_drop_manager.events.clear();
        events.into_iter().rev().collect()
    }

    // These two functions are based on egui_demo_lib/src/demo/drag_and_drop.rs
    pub fn drag_source(
        ui: &mut Ui,
        id: EguiId,
        source: DragDropSource,
        body: impl FnOnce(&mut Ui),
    ) {
        // This allows the app to avoid having to call reset() on every event
        // loop iteration, and fixes the bug that a drop target could see only
        // the drag sources that were instantiated earlier in the main event
        // loop.
        if !Self::is_anything_being_dragged(ui) {
            Self::reset();
        }

        if ui.memory(|mem| mem.is_being_dragged(id)) {
            // It is. So let's mark that it's the one.
            Self::global().lock().unwrap().source = Some(source);

            // Indicate in UI that we're dragging.
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);

            // Plan to draw above everything else except debug.
            let layer_id = LayerId::new(Order::Tooltip, id);

            // Draw the body and grab the response.
            let response = ui.with_layer_id(layer_id, body).response;

            // Shift the entire tooltip layer to keep up with mouse movement.
            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                let delta = pointer_pos - response.rect.center();
                ui.ctx().translate_layer(layer_id, delta);
            }
        } else {
            // Let the body draw itself, but scope to undo any style changes.
            let response = ui.scope(body).response;

            // If the mouse is still over the item, change cursor to indicate
            // that user could drag.
            let response = ui.interact(response.rect, id, Sense::drag());
            if response.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::Grab);
            }
        }
    }

    pub fn drop_target<R>(
        ui: &mut Ui,
        can_accept_what_is_being_dragged: bool,
        body: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        // Is there any drag source at all?
        let is_anything_dragged = Self::is_anything_being_dragged(ui);

        // Carve out a UI-sized area but leave a bit of margin to draw DnD
        // highlight.
        let margin = Vec2::splat(2.0);
        let outer_rect_bounds = ui.available_rect_before_wrap();
        let inner_rect = outer_rect_bounds.shrink2(margin);

        // We want this to draw behind the body, but we're not sure what it is
        // yet.
        let where_to_put_background = ui.painter().add(Shape::Noop);

        // Draw the potential target.
        let mut content_ui = ui.child_ui(inner_rect, *ui.layout());
        let ret = body(&mut content_ui);

        // I think but am not sure that this calculates the actual boundaries of
        // what the body drew.
        let outer_rect =
            Rect::from_min_max(outer_rect_bounds.min, content_ui.min_rect().max + margin);

        // Figure out what's going on in that rect.
        let (rect, response) = ui.allocate_at_least(outer_rect.size(), Sense::hover());

        // Adjust styling depending on whether this is still a potential target.
        let style = if is_anything_dragged && can_accept_what_is_being_dragged && response.hovered()
        {
            ui.visuals().widgets.active
        } else {
            ui.visuals().widgets.inactive
        };
        let mut fill = style.bg_fill;
        let mut stroke = style.bg_stroke;
        if is_anything_dragged {
            if !can_accept_what_is_being_dragged {
                fill = ui.visuals().gray_out(fill);
                stroke.color = ui.visuals().gray_out(stroke.color);
            }
        } else {
            fill = Color32::TRANSPARENT;
            stroke = Stroke::NONE;
        };

        // Update the background border based on target state.
        ui.painter().set(
            where_to_put_background,
            epaint::RectShape {
                rounding: style.rounding,
                fill,
                stroke,
                rect,
            },
        );

        if is_anything_dragged && !can_accept_what_is_being_dragged {
            ui.ctx().set_cursor_icon(CursorIcon::NotAllowed);
        }

        InnerResponse::new(ret, response)
    }

    fn is_anything_being_dragged(ui: &mut Ui) -> bool {
        ui.memory(|mem| mem.is_anything_being_dragged())
    }

    fn is_source_set() -> bool {
        Self::global().lock().unwrap().source.is_some()
    }

    pub fn is_dropped(ui: &mut Ui, response: &eframe::egui::Response) -> bool {
        Self::is_anything_being_dragged(ui)
            && response.hovered()
            && ui.input(|i| i.pointer.any_released())
            && Self::is_source_set()
    }

    pub fn source() -> Option<DragDropSource> {
        Self::global().lock().unwrap().source.clone()
    }

    pub fn initialize(drag_drop_manager: Self) -> Result<(), Mutex<DragDropManager>> {
        DD_MANAGER.set(Mutex::new(drag_drop_manager))
    }
}
