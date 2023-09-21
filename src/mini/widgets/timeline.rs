// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::{control::atlas, controllers::es_sequencer};
use crate::mini::{
    control_router::ControlRouter, ControlAtlas, DragDropEvent, DragDropManager, DragDropSource,
    ESSequencer, TrackUid,
};
use eframe::{
    egui::{self, vec2, Response, Ui},
    emath::{Align2, RectTransform},
    epaint::{pos2, FontId, Rect, RectShape, Shape},
};
use ensnare::prelude::*;
use groove_core::traits::gui::{Displays, DisplaysInTimeline};
use std::ops::Range;
use strum::EnumCount;
use strum_macros::{EnumCount as EnumCountMacro, FromRepr};

/// Wraps a [Timeline] as a [Widget](eframe::egui::Widget). Mutates the given view_range.
pub fn timeline<'a>(
    track_uid: TrackUid,
    sequencer: &'a mut ESSequencer,
    control_atlas: &'a mut ControlAtlas,
    control_router: &'a mut ControlRouter,
    range: Range<MusicalTime>,
    view_range: Range<MusicalTime>,
    focused: FocusedComponent,
) -> impl eframe::egui::Widget + 'a {
    move |ui: &mut eframe::egui::Ui| {
        Timeline::new(track_uid, sequencer, control_atlas, control_router)
            .range(range)
            .view_range(view_range)
            .focused(focused)
            .ui(ui)
    }
}

/// Wraps a [Legend] as a [Widget](eframe::egui::Widget). Mutates the given view_range.
pub fn legend(view_range: &mut std::ops::Range<MusicalTime>) -> impl eframe::egui::Widget + '_ {
    move |ui: &mut eframe::egui::Ui| Legend::new(view_range).ui(ui)
}

/// Wraps a [Grid] as a [Widget](eframe::egui::Widget).
pub fn grid(
    range: std::ops::Range<MusicalTime>,
    view_range: std::ops::Range<MusicalTime>,
) -> impl eframe::egui::Widget {
    move |ui: &mut eframe::egui::Ui| Grid::default().range(range).view_range(view_range).ui(ui)
}

/// Wraps an [EmptySpace] as a [Widget](eframe::egui::Widget).
pub fn empty_space(
    range: std::ops::Range<MusicalTime>,
    view_range: std::ops::Range<MusicalTime>,
) -> impl eframe::egui::Widget {
    move |ui: &mut eframe::egui::Ui| EmptySpace::new().range(range).view_range(view_range).ui(ui)
}

/// An egui widget that draws a legend on the horizontal axis of the timeline
/// view.
#[derive(Debug)]
pub struct Legend<'a> {
    /// The GUI view's time range.
    view_range: &'a mut Range<MusicalTime>,
}
impl<'a> Legend<'a> {
    fn new(view_range: &'a mut std::ops::Range<MusicalTime>) -> Self {
        Self { view_range }
    }

    fn steps(
        view_range: &std::ops::Range<MusicalTime>,
    ) -> std::iter::StepBy<Range<usize>> {
        let beat_count = view_range.end.total_beats() - view_range.start.total_beats();
        let step = (beat_count as f32).log10().round() as usize;
        (view_range.start.total_beats()..view_range.end.total_beats()).step_by(step * 2)
    }
}
impl<'a> Displays for Legend<'a> {
    fn ui(&mut self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        let desired_size = vec2(ui.available_width(), ui.spacing().interact_size.y);
        let (rect, response) = ui.allocate_exact_size(desired_size, eframe::egui::Sense::click());
        let to_screen = RectTransform::from_to(
            eframe::epaint::Rect::from_x_y_ranges(
                self.view_range.start.total_beats() as f32
                    ..=self.view_range.end.total_beats() as f32,
                rect.top()..=rect.bottom(),
            ),
            rect,
        );

        let font_id = FontId::proportional(12.0);
        for beat in Self::steps(self.view_range) {
            let beat_plus_one = beat + 1;
            let pos = to_screen * pos2(beat as f32, rect.top());
            ui.painter().text(
                pos,
                Align2::CENTER_TOP,
                format!("{beat_plus_one}"),
                font_id.clone(),
                ui.style().noninteractive().text_color(),
            );
        }
        ui.painter().line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            ui.style().noninteractive().fg_stroke,
        );

        response.context_menu(|ui| {
            if ui.button("Start x2").clicked() {
                self.view_range.start = self.view_range.start * 2;
                ui.close_menu();
            }
            if ui.button("Start x0.5").clicked() {
                self.view_range.start = self.view_range.start / 2;
                ui.close_menu();
            }
            if ui.button("Start +4").clicked() {
                self.view_range.start += MusicalTime::new_with_beats(4);
                ui.close_menu();
            }
        })
    }
}

/// An egui widget that draws a grid in the timeline view.
#[derive(Debug, Default)]
pub struct Grid {
    /// The timeline's full time range.
    range: Range<MusicalTime>,

    /// The GUI view's time range.
    view_range: Range<MusicalTime>,
}
impl Grid {
    fn range(mut self, range: std::ops::Range<MusicalTime>) -> Self {
        self.range = range.clone();
        self
    }
    fn view_range(mut self, view_range: std::ops::Range<MusicalTime>) -> Self {
        self.set_view_range(&view_range);
        self
    }
}
impl DisplaysInTimeline for Grid {
    fn set_view_range(&mut self, view_range: &std::ops::Range<MusicalTime>) {
        self.view_range = view_range.clone();
    }
}
impl Displays for Grid {
    fn ui(&mut self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        let desired_size = vec2(ui.available_width(), 64.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, eframe::egui::Sense::hover());
        let to_screen = RectTransform::from_to(
            eframe::epaint::Rect::from_x_y_ranges(
                self.view_range.start.total_beats() as f32
                    ..=self.view_range.end.total_beats() as f32,
                0.0..=1.0,
            ),
            rect,
        );
        let visuals = ui.ctx().style().visuals.widgets.noninteractive;

        let mut shapes = vec![Shape::Rect(RectShape::filled(
            rect,
            visuals.rounding,
            visuals.bg_fill,
        ))];

        for x in Legend::steps(&self.view_range) {
            shapes.push(Shape::LineSegment {
                points: [
                    to_screen * pos2(x as f32, 0.0),
                    to_screen * pos2(x as f32, 1.0),
                ],
                stroke: visuals.fg_stroke,
            });
        }
        ui.painter().extend(shapes);

        response
    }
}

/// An egui widget that displays nothing in the timeline view. This is useful as
/// a DnD target.
#[derive(Debug, Default)]
pub struct EmptySpace {
    view_range: Range<MusicalTime>,
    range: Range<MusicalTime>,
}
#[allow(missing_docs)]
impl EmptySpace {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn view_range(mut self, view_range: Range<MusicalTime>) -> Self {
        self.set_view_range(&view_range);
        self
    }
    pub fn range(mut self, range: Range<MusicalTime>) -> Self {
        self.range = range;
        self
    }
}
impl DisplaysInTimeline for EmptySpace {
    fn set_view_range(&mut self, view_range: &std::ops::Range<MusicalTime>) {
        self.view_range = view_range.clone();
    }
}
impl Displays for EmptySpace {
    fn ui(&mut self, ui: &mut Ui) -> Response {
        ui.set_min_height(ui.available_height());

        let full_range_beats =
            (self.view_range.end.total_beats() - self.view_range.start.total_beats() - 1) as f32;
        let range_beats =
            (self.range.end.total_beats() - self.range.start.total_beats() - 1) as f32;
        let range_as_pct = range_beats / full_range_beats;
        let desired_size = vec2(ui.available_width() * range_as_pct, ui.available_height());
        let (rect, response) =
            ui.allocate_exact_size(desired_size, eframe::egui::Sense::click_and_drag());

        let visuals = if ui.is_enabled() {
            ui.ctx().style().visuals.widgets.active
        } else {
            ui.ctx().style().visuals.widgets.noninteractive
        };

        // skip interaction
        ui.painter()
            .rect(rect, visuals.rounding, visuals.bg_fill, visuals.bg_stroke);
        ui.painter()
            .line_segment([rect.right_top(), rect.left_bottom()], visuals.fg_stroke);
        ui.painter()
            .line_segment([rect.left_top(), rect.right_bottom()], visuals.fg_stroke);
        response
    }
}

#[derive(Debug, Default, Copy, Clone, EnumCountMacro, FromRepr)]
#[allow(missing_docs)]
pub enum FocusedComponent {
    #[default]
    Sequencer,
    ControlAtlas,
}
impl FocusedComponent {
    /// Returns the next enum, wrapping to the start if necessary.
    pub fn next(&self) -> Self {
        FocusedComponent::from_repr((*self as usize + 1) % FocusedComponent::COUNT).unwrap()
    }
}

/// Draws the content area of a Timeline, which is the view of a [Track].
#[derive(Debug)]
struct Timeline<'a> {
    track_uid: TrackUid,

    /// The full timespan of the project.
    range: Range<MusicalTime>,

    /// The part of the timeline that is viewable.
    view_range: Range<MusicalTime>,

    /// Which component is currently enabled,
    focused: FocusedComponent,

    control_atlas: &'a mut ControlAtlas,
    control_router: &'a mut ControlRouter,
    sequencer: &'a mut ESSequencer,
}
impl<'a> DisplaysInTimeline for Timeline<'a> {
    fn set_view_range(&mut self, view_range: &std::ops::Range<MusicalTime>) {
        self.view_range = view_range.clone();
    }
}
impl<'a> Displays for Timeline<'a> {
    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let mut from_screen = RectTransform::identity(Rect::NOTHING);
        let can_accept = self.check_drag_source();
        let response = DragDropManager::drop_target(ui, can_accept, |ui| {
            let desired_size = vec2(ui.available_width(), 64.0);
            let (_id, rect) = ui.allocate_space(desired_size);
            from_screen = RectTransform::from_to(
                rect,
                Rect::from_x_y_ranges(
                    self.view_range.start.total_units() as f32
                        ..=self.view_range.end.total_units() as f32,
                    rect.top()..=rect.bottom(),
                ),
            );

            // What's going on here? To correctly capture Sense events, we
            // need to draw only the UI that we consider active, or enabled,
            // or focused, because egui does not seem to like widgets being
            // drawn on top of each other. So we first draw the non-focused
            // UI components, but wrapped in add_enabled_ui(false) so that
            // egui won't try to sense them. Then we draw the one focused
            // component normally.
            ui.add_enabled_ui(false, |ui| {
                self.ui_not_focused(ui, rect, self.focused);
            });
            self.ui_focused(ui, rect, self.focused)
        })
        .response;
        if DragDropManager::is_dropped(ui, &response) {
            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                let time_pos = from_screen * pointer_pos;
                let time = MusicalTime::new_with_units(time_pos.x as usize);
                if let Some(source) = DragDropManager::source() {
                    let event = match source {
                        DragDropSource::NewDevice(key) => {
                            Some(DragDropEvent::AddDeviceToTrack(key, self.track_uid))
                        }
                        DragDropSource::Pattern(pattern_uid) => Some(
                            DragDropEvent::AddPatternToTrack(pattern_uid, self.track_uid, time),
                        ),
                        DragDropSource::ControlTrip(_uid) => None,
                    };
                    if let Some(event) = event {
                        DragDropManager::enqueue_event(event);
                    }
                }
            } else {
                eprintln!("Dropped on timeline at unknown position");
            }
        }
        response
    }
}
impl<'a> Timeline<'a> {
    pub fn new(
        track_uid: TrackUid,
        sequencer: &'a mut ESSequencer,
        control_atlas: &'a mut ControlAtlas,
        control_router: &'a mut ControlRouter,
    ) -> Self {
        Self {
            track_uid,
            range: Default::default(),
            view_range: Default::default(),
            focused: Default::default(),
            sequencer,
            control_atlas,
            control_router,
        }
    }
    fn range(mut self, range: Range<MusicalTime>) -> Self {
        self.range = range;
        self
    }

    fn view_range(mut self, view_range: Range<MusicalTime>) -> Self {
        self.set_view_range(&view_range);
        self
    }

    fn focused(mut self, component: FocusedComponent) -> Self {
        self.focused = component;
        self
    }

    // Draws the Timeline component that is currently focused.
    fn ui_focused(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        component: FocusedComponent,
    ) -> egui::Response {
        match component {
            FocusedComponent::ControlAtlas => {
                ui.allocate_ui_at_rect(rect, |ui| {
                    ui.add(atlas(
                        self.control_atlas,
                        self.control_router,
                        self.view_range.clone(),
                    ))
                })
                .inner
            }
            FocusedComponent::Sequencer => {
                ui.allocate_ui_at_rect(rect, |ui| {
                    ui.add(es_sequencer(self.sequencer, self.view_range.clone()))
                })
                .inner
            }
        }
    }

    // Draws the Timeline components that are not currently focused. It's up to
    // the caller to wrap in ui.add_enabled_ui().
    fn ui_not_focused(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        which: FocusedComponent,
    ) -> egui::Response {
        // The Grid is always disabled and drawn first.
        let mut response = ui
            .allocate_ui_at_rect(rect, |ui| {
                ui.add(grid(self.range.clone(), self.view_range.clone()))
            })
            .inner;

        // Now go through and draw the components that are *not* enabled.
        if !matches!(which, FocusedComponent::ControlAtlas) {
            response |= ui
                .allocate_ui_at_rect(rect, |ui| {
                    ui.add(atlas(
                        self.control_atlas,
                        self.control_router,
                        self.view_range.clone(),
                    ))
                })
                .inner;
        }
        if !matches!(which, FocusedComponent::Sequencer) {
            response |= ui
                .allocate_ui_at_rect(rect, |ui| {
                    ui.add(es_sequencer(self.sequencer, self.view_range.clone()))
                })
                .inner;
        }
        response
    }

    // Looks at what's being dragged, if anything, and updates any state needed
    // to handle it. Returns whether we are interested in this drag source.
    fn check_drag_source(&mut self) -> bool {
        if let Some(source) = DragDropManager::source() {
            if matches!(source, DragDropSource::Pattern(_)) {
                self.focused = FocusedComponent::Sequencer;
                return true;
            }
        }
        false
    }
}
