// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::{
    mini::{
        control_router::ControlRouter, ControlAtlas, ControlTrip, ControlTripBuilder,
        ControlTripPath, DragDropManager, DragDropSource,
    },
    EntityFactory,
};
use eframe::{
    egui::{self, Layout, Sense},
    emath::RectTransform,
    epaint::{pos2, vec2, Color32, Rect, Stroke},
};
use ensnare::prelude::*;
use groove_core::traits::{
    gui::{Displays, DisplaysInTimeline},
    HasUid,
};
use std::ops::Range;

/// Wraps a [ControlAtlas] as a [Widget](eframe::egui::Widget).
pub fn atlas<'a>(
    control_atlas: &'a mut ControlAtlas,
    control_router: &'a mut ControlRouter,
    view_range: Range<MusicalTime>,
) -> impl eframe::egui::Widget + 'a {
    move |ui: &mut eframe::egui::Ui| Atlas::new(control_atlas, control_router, view_range).ui(ui)
}

#[derive(Debug)]
struct Atlas<'a> {
    control_atlas: &'a mut ControlAtlas,
    control_router: &'a mut ControlRouter,
    view_range: Range<MusicalTime>,
}
impl<'a> Atlas<'a> {
    fn new(
        control_atlas: &'a mut ControlAtlas,
        control_router: &'a mut ControlRouter,
        view_range: Range<MusicalTime>,
    ) -> Self {
        Self {
            control_atlas,
            control_router,
            view_range,
        }
    }
}
impl<'a> DisplaysInTimeline for Atlas<'a> {
    fn set_view_range(&mut self, view_range: &std::ops::Range<MusicalTime>) {
        self.view_range = view_range.clone();
    }
}
impl<'a> Displays for Atlas<'a> {
    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        // This push_id() was needed to avoid an ID conflict. I think it is
        // because we're drawing widgets on top of each other, but I'm honestly
        // not sure.
        ui.push_id(ui.next_auto_id(), |ui| {
            let (_id, rect) = ui.allocate_space(vec2(ui.available_width(), 64.0));
            let response = ui
                .allocate_ui_at_rect(rect, |ui| {
                    let mut remove_uid = None;
                    self.control_atlas.trips_mut().iter_mut().for_each(|t| {
                        ui.allocate_ui_at_rect(rect, |ui| {
                            ui.add(trip(t, self.control_router, self.view_range.clone()));

                            // Draw the trip controls.
                            if ui.is_enabled() {
                                // TODO: I don't know why this isn't flush with
                                // the right side of the component.
                                let controls_rect = Rect::from_points(&[
                                    rect.right_top(),
                                    pos2(
                                        rect.right()
                                            - ui.ctx().style().spacing.interact_size.x * 2.0,
                                        rect.top(),
                                    ),
                                ]);
                                ui.allocate_ui_at_rect(controls_rect, |ui| {
                                    ui.allocate_ui_with_layout(
                                        ui.available_size(),
                                        Layout::right_to_left(eframe::emath::Align::Center),
                                        |ui| {
                                            if ui.button("x").clicked() {
                                                remove_uid = Some(t.uid());
                                            }
                                            // TODO: this will be what you drag
                                            // to things you want this trip to
                                            // control
                                            DragDropManager::drag_source(
                                                ui,
                                                ui.next_auto_id(),
                                                DragDropSource::ControlTrip(t.uid()),
                                                |ui| {
                                                    ui.label("S");
                                                },
                                            );
                                        },
                                    );
                                });
                            }
                        });
                    });
                    if let Some(uid) = remove_uid {
                        self.control_atlas.remove_trip(uid);
                    }
                })
                .response;
            if ui.is_enabled() {
                response.context_menu(|ui| {
                    if ui.button("Add trip").clicked() {
                        ui.close_menu();
                        let mut trip = ControlTripBuilder::default()
                            .random(MusicalTime::START)
                            .build()
                            .unwrap();
                        trip.set_uid(EntityFactory::global().mint_uid());
                        self.control_atlas.add_trip(trip);
                    }
                })
            } else {
                response
            }
        })
        .inner
    }
}

/// Wraps a [ControlTrip] as a [Widget](eframe::egui::Widget).
fn trip<'a>(
    trip: &'a mut ControlTrip,
    control_router: &'a mut ControlRouter,
    view_range: Range<MusicalTime>,
) -> impl eframe::egui::Widget + 'a {
    move |ui: &mut eframe::egui::Ui| Trip::new(trip, control_router, view_range).ui(ui)
}

#[derive(Debug)]
struct Trip<'a> {
    control_trip: &'a mut ControlTrip,
    control_router: &'a mut ControlRouter,
    view_range: Range<MusicalTime>,
}
impl<'a> Trip<'a> {
    fn new(
        control_trip: &'a mut ControlTrip,
        control_router: &'a mut ControlRouter,
        view_range: Range<MusicalTime>,
    ) -> Self {
        Self {
            control_trip,
            control_router,
            view_range,
        }
    }
}
impl<'a> Displays for Trip<'a> {
    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click());
        let to_screen = RectTransform::from_to(
            Rect::from_x_y_ranges(
                self.view_range.start.total_units() as f32
                    ..=self.view_range.end.total_units() as f32,
                ControlValue::MAX.0 as f32..=ControlValue::MIN.0 as f32,
            ),
            response.rect,
        );

        // The first step always starts at the left of the view range.
        let mut pos = to_screen
            * pos2(
                MusicalTime::START.total_units() as f32,
                if let Some(step) = self.control_trip.steps_mut().first() {
                    step.value.0 as f32
                } else {
                    0.0
                },
            );
        let stroke = if ui.is_enabled() {
            ui.ctx().style().visuals.widgets.active.bg_stroke
        } else {
            ui.ctx().style().visuals.widgets.inactive.bg_stroke
        };
        let steps_len = self.control_trip.steps().len();
        self.control_trip
            .steps_mut()
            .iter_mut()
            .enumerate()
            .for_each(|(index, step)| {
                // Get the next step position, adjusting if it's the last one.
                let second_pos = if index + 1 == steps_len {
                    let value = pos.y;
                    // Last step. Extend to end of view range.
                    let mut tmp_pos =
                        to_screen * pos2(self.view_range.end.total_units() as f32, 0.0);
                    tmp_pos.y = value;
                    tmp_pos
                } else {
                    // Not last step. Get the actual value.
                    to_screen * pos2(step.time.total_units() as f32, step.value.0 as f32)
                };

                // If we're hovering over this step, highlight it.
                let stroke = if response.hovered() {
                    if let Some(hover_pos) = ui.ctx().pointer_interact_pos() {
                        if hover_pos.x >= pos.x && hover_pos.x < second_pos.x {
                            if response.clicked() {
                                let from_screen = to_screen.inverse();
                                let hover_pos_local = from_screen * hover_pos;
                                step.value = ControlValue::from(hover_pos_local.y);
                            } else if response.secondary_clicked() {
                                step.path = step.path.next();
                            }

                            Stroke {
                                width: stroke.width * 2.0,
                                color: Color32::YELLOW,
                            }
                        } else {
                            stroke
                        }
                    } else {
                        stroke
                    }
                } else {
                    stroke
                };

                // Draw according to the step type.
                match step.path {
                    ControlTripPath::None => {}
                    ControlTripPath::Flat => {
                        painter.line_segment([pos, pos2(pos.x, second_pos.y)], stroke);
                        painter.line_segment([pos2(pos.x, second_pos.y), second_pos], stroke);
                    }
                    ControlTripPath::Linear => {
                        painter.line_segment([pos, second_pos], stroke);
                    }
                    ControlTripPath::Logarithmic => todo!(),
                    ControlTripPath::Exponential => todo!(),
                }
                pos = second_pos;
            });

        if ui.is_enabled() {
            let label =
                if let Some(links) = self.control_router.control_links(self.control_trip.uid()) {
                    let link_texts = links.iter().fold(Vec::default(), |mut v, (uid, index)| {
                        // TODO: this can be a descriptive list of controlled things
                        v.push(format!("{uid}-{index:?} "));
                        v
                    });
                    link_texts.join("/")
                } else {
                    String::from("none")
                };
            if ui
                .allocate_ui_at_rect(response.rect, |ui| ui.button(&label))
                .inner
                .clicked()
            {
                // TODO: this is incomplete. It's a placeholder while I figure
                // out the best way to present this information (it might
                // actually be DnD rather than menu-driven).
                self.control_router.link_control(
                    self.control_trip.uid(),
                    Uid(234),
                    ControlIndex(456),
                );
            }
        }

        response
    }
}
