// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::mini::Note;
use eframe::{
    egui::{Response, Ui},
    epaint::{vec2, Color32, RectShape, Rounding, Shape, Stroke},
};
use eframe::{
    emath::{Align2, RectTransform},
    epaint::{pos2, FontId},
};
use groove_core::{
    time::MusicalTime,
    traits::gui::{Displays, DisplaysInTimeline},
};
use std::ops::Range;

/// Wraps a [Legend] as an [eframe::egui::Widget]. Mutates the given view_range.
pub fn legend<'a>(
    view_range: &'a mut std::ops::Range<MusicalTime>,
) -> impl eframe::egui::Widget + 'a {
    move |ui: &mut eframe::egui::Ui| Legend::new(view_range).ui(ui)
}

/// Wraps a [Grid] as an [eframe::egui::Widget].
pub fn grid(
    range: std::ops::Range<MusicalTime>,
    view_range: std::ops::Range<MusicalTime>,
) -> impl eframe::egui::Widget {
    move |ui: &mut eframe::egui::Ui| Grid::default().range(range).view_range(view_range).ui(ui)
}

/// Wraps a [Pattern] as an [eframe::egui::Widget].
pub fn pattern(
    range: std::ops::Range<MusicalTime>,
    view_range: std::ops::Range<MusicalTime>,
) -> impl eframe::egui::Widget {
    move |ui: &mut eframe::egui::Ui| Pattern::new().range(range).view_range(view_range).ui(ui)
}

/// Wraps an [EmptySpace] as an [eframe::egui::Widget].
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
    fn new(view_range: &'a mut std::ops::Range<groove_core::time::MusicalTime>) -> Self {
        Self { view_range }
    }

    fn steps(
        view_range: &std::ops::Range<groove_core::time::MusicalTime>,
    ) -> std::iter::StepBy<Range<usize>> {
        let beat_count = view_range.end.total_beats() - view_range.start.total_beats();
        let step = (beat_count as f32).log10().round() as usize;
        (view_range.start.total_beats()..view_range.end.total_beats()).step_by(step * 2)
    }

    fn set_view_range(
        &mut self,
        view_range: &'a mut std::ops::Range<groove_core::time::MusicalTime>,
    ) {
        self.view_range = view_range;
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
        for (i, beat) in Self::steps(&self.view_range).enumerate() {
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

        let response = response.context_menu(|ui| {
            if ui.button("Start x2").clicked() {
                self.view_range.start = self.view_range.start * 2;
                ui.close_menu();
            }
            if ui.button("Start x0.5").clicked() {
                self.view_range.start = self.view_range.start / 2;
                ui.close_menu();
            }
            if ui.button("Start +4").clicked() {
                self.view_range.start = self.view_range.start + MusicalTime::new_with_beats(4);
                ui.close_menu();
            }
        });

        response
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
    fn range(mut self, range: std::ops::Range<groove_core::time::MusicalTime>) -> Self {
        self.range = range.clone();
        self
    }
    fn view_range(mut self, view_range: std::ops::Range<groove_core::time::MusicalTime>) -> Self {
        self.set_view_range(&view_range);
        self
    }
}
impl DisplaysInTimeline for Grid {
    fn set_view_range(&mut self, view_range: &std::ops::Range<groove_core::time::MusicalTime>) {
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

        let mut shapes = vec![Shape::Rect(RectShape::filled(
            rect,
            Rounding::default(),
            Color32::DARK_GRAY,
        ))];

        ui.painter().rect(
            rect,
            Rounding::default(),
            Color32::DARK_GRAY,
            ui.style().noninteractive().bg_stroke,
        );

        for x in Legend::steps(&self.view_range) {
            shapes.push(Shape::LineSegment {
                points: [
                    to_screen * pos2(x as f32, 0.0),
                    to_screen * pos2(x as f32, 1.0),
                ],
                stroke: Stroke {
                    width: 1.0,
                    color: Color32::YELLOW,
                },
            });
        }
        ui.painter().extend(shapes);

        response
    }
}

/// An egui widget that displays a [Pattern] in the timeline view.
#[derive(Debug, Default)]
pub struct Pattern {
    view_range: Range<MusicalTime>,
    range: Range<MusicalTime>,
    notes: Vec<Note>,
}
impl Pattern {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn range(mut self, range: Range<MusicalTime>) -> Self {
        self.range = range;
        self
    }
    fn view_range(mut self, view_range: std::ops::Range<groove_core::time::MusicalTime>) -> Self {
        self.set_view_range(&view_range);
        self
    }
}
impl DisplaysInTimeline for Pattern {
    fn set_view_range(&mut self, view_range: &std::ops::Range<groove_core::time::MusicalTime>) {
        self.view_range = view_range.clone();
    }
}
impl Displays for Pattern {
    fn ui(&mut self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        let desired_size = ui.available_height() * eframe::egui::vec2(1.0, 1.0);
        let (rect, response) =
            ui.allocate_exact_size(desired_size, eframe::egui::Sense::click_and_drag());
        // skip interaction
        ui.painter().rect(
            rect,
            Rounding::default(),
            Color32::BLACK,
            Stroke {
                width: 1.0,
                color: Color32::YELLOW,
            },
        );
        ui.painter().line_segment(
            [rect.right_top(), rect.left_bottom()],
            Stroke {
                width: 1.0,
                color: Color32::YELLOW,
            },
        );
        ui.painter().line_segment(
            [rect.left_top(), rect.right_bottom()],
            Stroke {
                width: 1.0,
                color: Color32::YELLOW,
            },
        );
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
    fn set_view_range(&mut self, view_range: &std::ops::Range<groove_core::time::MusicalTime>) {
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
        // skip interaction
        ui.painter()
            .rect(rect, Rounding::default(), Color32::BLACK, Stroke::default());
        ui.painter().line_segment(
            [rect.right_top(), rect.left_bottom()],
            Stroke {
                width: 1.0,
                color: Color32::YELLOW,
            },
        );
        ui.painter().line_segment(
            [rect.left_top(), rect.right_bottom()],
            Stroke {
                width: 1.0,
                color: Color32::YELLOW,
            },
        );
        ui.painter().rect(
            rect,
            Rounding::none(),
            Color32::BLACK,
            Stroke {
                width: 1.0,
                color: Color32::YELLOW,
            },
        );
        response
    }
}
