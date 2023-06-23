use super::entity_factory::Key;
use eframe::{
    egui::{CursorIcon, Id as EguiId, InnerResponse, LayerId, Order, Sense, Ui},
    epaint::{self, Rect, Shape, Vec2},
};
use groove_core::Uid;

#[allow(missing_docs)]
#[allow(dead_code)]
#[derive(Debug)]
pub enum DragDropSource {
    ControllerInTrack(usize, Uid),
    EffectInTrack(usize, Uid),
    InstrumentInTrack(usize, Uid),
    NewController(Key),
    NewEffect(Key),
    NewInstrument(Key),
}

// TODO: a way to express rules about what can and can't be dropped
#[allow(missing_docs)]
#[derive(Debug, Default)]
pub struct DragDropManager {
    source: Option<DragDropSource>,
}
#[allow(missing_docs)]
impl DragDropManager {
    pub fn reset(&mut self) {
        self.source = None;
    }

    // These two functions are based on egui_demo_lib/src/demo/drag_and_drop.rs
    #[allow(dead_code)]
    pub fn drag_source(
        &mut self,
        ui: &mut Ui,
        id: EguiId,
        dnd_id: DragDropSource,
        body: impl FnOnce(&mut Ui),
    ) {
        let is_being_dragged = ui.memory(|mem| mem.is_being_dragged(id));

        if is_being_dragged {
            self.source = Some(dnd_id);
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
            let layer_id = LayerId::new(Order::Tooltip, id);
            let response = ui.with_layer_id(layer_id, body).response;
            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                let delta = pointer_pos - response.rect.center();
                ui.ctx().translate_layer(layer_id, delta);
            }
        } else {
            let response = ui.scope(body).response;
            let response = ui.interact(response.rect, id, Sense::drag());
            if response.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::Grab);
            }
        }
    }

    #[allow(dead_code)]
    pub fn drop_target<R>(
        &mut self,
        ui: &mut Ui,
        can_accept_what_is_being_dragged: bool,
        body: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let is_being_dragged = ui.memory(|mem| mem.is_anything_being_dragged());

        let margin = Vec2::splat(2.0);

        let outer_rect_bounds = ui.available_rect_before_wrap();
        let inner_rect = outer_rect_bounds.shrink2(margin);
        let where_to_put_background = ui.painter().add(Shape::Noop);
        let mut content_ui = ui.child_ui(inner_rect, *ui.layout());
        let ret = body(&mut content_ui);
        let outer_rect =
            Rect::from_min_max(outer_rect_bounds.min, content_ui.min_rect().max + margin);
        let (rect, response) = ui.allocate_at_least(outer_rect.size(), Sense::hover());

        let style = if is_being_dragged && can_accept_what_is_being_dragged && response.hovered() {
            ui.visuals().widgets.active
        } else {
            ui.visuals().widgets.inactive
        };

        let mut fill = style.bg_fill;
        let mut stroke = style.bg_stroke;
        if is_being_dragged && !can_accept_what_is_being_dragged {
            fill = ui.visuals().gray_out(fill);
            stroke.color = ui.visuals().gray_out(stroke.color);
        }

        ui.painter().set(
            where_to_put_background,
            epaint::RectShape {
                rounding: style.rounding,
                fill,
                stroke,
                rect,
            },
        );

        InnerResponse::new(ret, response)
    }
}
