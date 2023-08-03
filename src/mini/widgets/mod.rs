// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::piano_roll::Note;
use eframe::{
    egui::{Response, Ui},
    emath::{Align2, RectTransform},
    epaint::{pos2, vec2, Color32, FontId, Rounding, Stroke},
};
use groove_core::time::MusicalTime;
use std::ops::Range;

pub fn pattern_icon(duration: MusicalTime, notes: &[Note]) -> impl eframe::egui::Widget + '_ {
    move |ui: &mut eframe::egui::Ui| {
        PatternIcon::new()
            .duration(duration)
            .notes(notes)
            .ui_content(ui)
    }
}

pub fn arrangement_legend(range: std::ops::Range<MusicalTime>) -> impl eframe::egui::Widget {
    move |ui: &mut eframe::egui::Ui| ArrangementLegend::new().range(range).ui_content(ui)
}

pub fn arrangement_pattern(
    arrangement_range: std::ops::Range<MusicalTime>,
    range: std::ops::Range<MusicalTime>,
) -> impl eframe::egui::Widget {
    move |ui: &mut eframe::egui::Ui| {
        ArrangementPattern::new()
            .arrangement_range(arrangement_range)
            .range(range)
            .ui_content(ui)
    }
}

pub fn arrangement_space(
    arrangement_range: std::ops::Range<MusicalTime>,
    range: std::ops::Range<MusicalTime>,
) -> impl eframe::egui::Widget {
    move |ui: &mut eframe::egui::Ui| {
        ArrangementSpace::new()
            .arrangement_range(arrangement_range)
            .range(range)
            .ui_content(ui)
    }
}

#[derive(Debug, Default)]
pub struct ArrangementPattern {
    arrangement_range: Range<MusicalTime>,
    range: Range<MusicalTime>,
    notes: Vec<Note>,
}
impl ArrangementPattern {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn arrangement_range(mut self, arrangement_range: Range<MusicalTime>) -> Self {
        self.arrangement_range = arrangement_range;
        self
    }
    pub fn range(mut self, range: Range<MusicalTime>) -> Self {
        self.range = range;
        self
    }
    pub fn ui_content(&mut self, ui: &mut Ui) -> Response {
        let desired_size = ui.available_height() * eframe::egui::vec2(1.0, 1.0);
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

#[derive(Debug, Default)]
pub struct ArrangementSpace {
    arrangement_range: Range<MusicalTime>,
    range: Range<MusicalTime>,
    notes: Vec<Note>,
}
impl ArrangementSpace {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn arrangement_range(mut self, arrangement_range: Range<MusicalTime>) -> Self {
        self.arrangement_range = arrangement_range;
        self
    }
    pub fn range(mut self, range: Range<MusicalTime>) -> Self {
        self.range = range;
        self
    }
    pub fn ui_content(&mut self, ui: &mut Ui) -> Response {
        ui.set_min_height(ui.available_height());

        let full_range_beats = (self.arrangement_range.end.total_beats()
            - self.arrangement_range.start.total_beats()
            - 1) as f32;
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

#[derive(Debug, Default)]
pub struct PatternIcon<'a> {
    duration: MusicalTime,
    notes: &'a [Note],
}
impl<'a> PatternIcon<'a> {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn duration(mut self, duration: MusicalTime) -> Self {
        self.duration = duration;
        self
    }
    pub fn notes(mut self, notes: &'a [Note]) -> Self {
        self.notes = notes;
        self
    }
    pub fn ui_content(&mut self, ui: &mut Ui) -> Response {
        let desired_size = ui.spacing().interact_size.y * eframe::egui::vec2(3.0, 3.0);
        let (rect, response) =
            ui.allocate_exact_size(desired_size, eframe::egui::Sense::click_and_drag());
        // skip interaction
        ui.painter().rect(
            rect,
            Rounding::default(),
            Color32::DARK_GRAY,
            Stroke::default(),
        );
        let to_screen = RectTransform::from_to(
            eframe::epaint::Rect::from_x_y_ranges(
                MusicalTime::START.total_beats() as f32..=self.duration.total_beats() as f32,
                0.0..=128.0,
            ),
            rect,
        );
        for note in self.notes {
            let key = note.key as f32;
            let p1 = to_screen * eframe::epaint::pos2(note.range.start.total_beats() as f32, key);
            let p2 =
                to_screen * eframe::epaint::pos2(note.range.end.total_beats() as f32, key + 1.0);
            let p2 = if p1.x != p2.x {
                p2
            } else {
                eframe::epaint::pos2(p2.x + 1.0, p2.y)
            };
            ui.painter().line_segment(
                [p1, p2],
                Stroke {
                    width: 2.0,
                    color: Color32::YELLOW,
                },
            );
        }

        response
    }
}

#[derive(Debug, Default)]
pub struct ArrangementLegend {
    range: std::ops::Range<MusicalTime>,
}
impl ArrangementLegend {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn range(mut self, range: Range<MusicalTime>) -> Self {
        self.range = range;
        self
    }

    pub fn ui_content(&mut self, ui: &mut Ui) -> Response {
        let desired_size = vec2(ui.available_width(), ui.spacing().interact_size.y);
        let (rect, response) =
            ui.allocate_exact_size(desired_size, eframe::egui::Sense::click_and_drag());
        let to_screen = RectTransform::from_to(
            eframe::epaint::Rect::from_x_y_ranges(
                self.range.start.total_beats() as f32..=self.range.end.total_beats() as f32,
                rect.top()..=rect.bottom(),
            ),
            rect,
        );

        let start_beat = self.range.start.total_beats();
        let end_beat = self.range.end.total_beats();

        let font_id = FontId::proportional(12.0);
        let beat_count = (end_beat - start_beat) as usize;
        let skip = if beat_count > 100 {
            10
        } else if beat_count > 10 {
            2
        } else {
            1
        };
        for (i, beat) in (start_beat..end_beat).enumerate() {
            if i != 0 && i != beat_count - 1 && i % skip != 0 {
                continue;
            }
            let beat_plus_one = beat + 1;
            let pos = to_screen * pos2(beat as f32, rect.top());
            ui.painter().text(
                pos,
                Align2::CENTER_TOP,
                format!("{beat_plus_one}"),
                font_id.clone(),
                Color32::YELLOW,
            );
        }
        ui.painter().line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            Stroke {
                width: 1.0,
                color: Color32::YELLOW,
            },
        );

        response
    }
}
