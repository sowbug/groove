// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::mini::{DragDropManager, DragDropSource, Note, Pattern, PatternUid, SelectionSet};
use eframe::{
    egui::{Id as EguiId, Response, Ui},
    emath::RectTransform,
    epaint::{pos2, RectShape, Shape},
};
use ensnare::traits::Displays;
use ensnare::{midi::MidiNote, prelude::*};
use std::collections::HashMap;

use super::MIDI_NOTE_F32_RANGE;

/// Wraps an [Icon] as a [Widget](eframe::egui::Widget).
pub fn icon(
    duration: MusicalTime,
    notes: &[Note],
    is_selected: bool,
) -> impl eframe::egui::Widget + '_ {
    move |ui: &mut eframe::egui::Ui| {
        Icon::new()
            .duration(duration)
            .notes(notes)
            .is_selected(is_selected)
            .ui(ui)
    }
}

/// Wraps a [DraggableIcon] as a [Widget](eframe::egui::Widget).
pub fn draggable_icon() -> impl eframe::egui::Widget {
    move |ui: &mut eframe::egui::Ui| DraggableIcon::new().ui(ui)
}

/// Wraps a [Grid] as a [Widget](eframe::egui::Widget).
pub fn grid(duration: MusicalTime) -> impl eframe::egui::Widget {
    move |ui: &mut eframe::egui::Ui| Grid::default().duration(duration).ui(ui)
}

/// Wraps a [Carousel] as a [Widget](eframe::egui::Widget).
pub fn carousel<'a>(
    pattern_uids: &'a [PatternUid],
    uids_to_patterns: &'a HashMap<PatternUid, Pattern>,
    selection_set: &'a mut SelectionSet<PatternUid>,
) -> impl eframe::egui::Widget + 'a {
    move |ui: &mut eframe::egui::Ui| {
        Carousel::new(pattern_uids, uids_to_patterns, selection_set).ui(ui)
    }
}

/// Displays an iconic representation of a sequence of [Note]s (that might be in
/// a [Pattern](crate::mini::piano_roll::Pattern)). Intended to be a
/// drag-and-drop source.
#[derive(Debug, Default)]
pub struct Icon<'a> {
    duration: MusicalTime,
    notes: &'a [Note],
    is_selected: bool,
}
impl<'a> Icon<'a> {
    /// Creates a new [Icon].
    pub fn new() -> Self {
        Default::default()
    }
    /// Sets the duration of the pattern implied by the notes.
    pub fn duration(mut self, duration: MusicalTime) -> Self {
        self.duration = duration;
        self
    }
    /// Sets the sequence of [Note]s that determine the icon's appearance.
    pub fn notes(mut self, notes: &'a [Note]) -> Self {
        self.notes = notes;
        self
    }
    /// Sets whether this widget is selected in the UI.
    pub fn is_selected(mut self, is_selected: bool) -> Self {
        self.is_selected = is_selected;
        self
    }
}
impl<'a> Displays for Icon<'a> {
    fn ui(&mut self, ui: &mut Ui) -> Response {
        let desired_size = ui.spacing().interact_size.y * eframe::egui::vec2(3.0, 3.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, eframe::egui::Sense::click());

        let visuals = if ui.is_enabled() {
            ui.ctx().style().visuals.widgets.active
        } else {
            ui.ctx().style().visuals.widgets.inactive
        };

        if self.is_selected {
            ui.painter()
                .rect(rect, visuals.rounding, visuals.bg_fill, visuals.fg_stroke);
        } else {
            ui.painter().rect(
                rect,
                visuals.rounding,
                visuals.weak_bg_fill,
                visuals.bg_stroke,
            );
        }
        let to_screen = RectTransform::from_to(
            eframe::epaint::Rect::from_x_y_ranges(
                MusicalTime::START.total_parts() as f32..=self.duration.total_parts() as f32,
                128.0..=0.0,
            ),
            rect,
        );
        for note in self.notes {
            let key = note.key as f32;
            let p1 = to_screen * eframe::epaint::pos2(note.range.start.total_parts() as f32, key);
            let mut p2 = to_screen * eframe::epaint::pos2(note.range.end.total_parts() as f32, key);

            // Even very short notes should be visible.
            if p1.x == p2.x {
                p2.x += 1.0;
            }
            ui.painter().line_segment([p1, p2], visuals.fg_stroke);
        }
        response
    }
}

/// Displays a simple representation of a [Pattern]. Intended to be a
/// drag-and-drop source. This is needed in the short term because egui doesn't
/// have an easy way to make a widget both clickable and a drag source.
#[derive(Debug, Default)]
pub struct DraggableIcon {}
impl DraggableIcon {
    /// Creates a new [DraggableIcon].
    pub fn new() -> Self {
        Default::default()
    }
}
impl Displays for DraggableIcon {
    fn ui(&mut self, ui: &mut Ui) -> Response {
        let desired_size = ui.spacing().interact_size.y * eframe::egui::vec2(3.0, 1.0);
        let (rect, response) =
            ui.allocate_exact_size(desired_size, eframe::egui::Sense::click_and_drag());

        let visuals = if ui.is_enabled() {
            ui.ctx().style().visuals.widgets.active
        } else {
            ui.ctx().style().visuals.widgets.inactive
        };

        ui.painter().rect(
            rect,
            visuals.rounding,
            visuals.weak_bg_fill,
            visuals.bg_stroke,
        );

        response
    }
}

/// Displays a row of selectable icons, each with a drag source.
#[derive(Debug)]
pub struct Carousel<'a> {
    pattern_uids: &'a [PatternUid],
    uids_to_patterns: &'a HashMap<PatternUid, Pattern>,
    selection_set: &'a mut SelectionSet<PatternUid>,
}
impl<'a> Carousel<'a> {
    /// Creates a new [Carousel].
    pub fn new(
        pattern_uids: &'a [PatternUid],
        uids_to_patterns: &'a HashMap<PatternUid, Pattern>,
        selection_set: &'a mut SelectionSet<PatternUid>,
    ) -> Self {
        Self {
            pattern_uids,
            uids_to_patterns,
            selection_set,
        }
    }
}
impl<'a> Displays for Carousel<'a> {
    fn ui(&mut self, ui: &mut Ui) -> Response {
        ui.horizontal_top(|ui| {
            let icon_width = ui.available_width() / self.pattern_uids.len() as f32;
            ui.set_max_width(ui.available_width());
            ui.set_height(64.0);
            self.pattern_uids.iter().for_each(|pattern_uid| {
                ui.vertical(|ui| {
                    ui.set_max_width(icon_width);
                    if let Some(pattern) = self.uids_to_patterns.get(pattern_uid) {
                        if ui
                            .add(icon(
                                pattern.duration(),
                                pattern.notes(),
                                self.selection_set.contains(pattern_uid),
                            ))
                            .clicked()
                        {
                            self.selection_set.click(pattern_uid, false);
                        };
                    }
                    let dd_id = EguiId::new("piano roll").with(pattern_uid);
                    DragDropManager::drag_source(
                        ui,
                        dd_id,
                        DragDropSource::Pattern(*pattern_uid),
                        |ui| {
                            ui.add(draggable_icon());
                        },
                    );
                });
            });
        })
        .response
    }
}

/// An egui widget that draws a grid in
/// [PianoRoll](crate::mini::piano_roll::PianoRoll)'s pattern-editing view.
#[derive(Debug, Default)]
pub struct Grid {
    /// The extent of the [Pattern](crate::mini::piano_roll::Pattern) to be
    /// edited.
    duration: MusicalTime,
}
impl Grid {
    fn duration(mut self, duration: MusicalTime) -> Self {
        self.duration = duration;
        self
    }
}
impl Displays for Grid {
    fn ui(&mut self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        let desired_size = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(desired_size, eframe::egui::Sense::hover());
        let to_screen = RectTransform::from_to(
            eframe::epaint::Rect::from_x_y_ranges(
                MusicalTime::START.total_parts() as f32..=self.duration.total_parts() as f32,
                MIDI_NOTE_F32_RANGE,
            ),
            rect,
        );
        let visuals = ui.ctx().style().visuals.widgets.noninteractive;

        let mut shapes = vec![Shape::Rect(RectShape::filled(
            rect,
            visuals.rounding,
            visuals.bg_fill,
        ))];

        for part in 0..self.duration.total_parts() {
            let x = part as f32;
            let stroke = if part % 16 == 0 {
                visuals.fg_stroke
            } else {
                visuals.bg_stroke
            };
            shapes.push(Shape::LineSegment {
                points: [to_screen * pos2(x, 0.0), to_screen * pos2(x, 127.0)],
                stroke,
            });
        }
        for key in MidiNote::MIN as u8..MidiNote::MAX as u8 {
            let left = to_screen * pos2(MusicalTime::START.total_parts() as f32, key as f32);
            let right = to_screen * pos2(self.duration.total_parts() as f32, key as f32);
            let stroke = if (key - MidiNote::C0 as u8) % 12 == 0 {
                visuals.fg_stroke
            } else {
                visuals.bg_stroke
            };
            shapes.push(Shape::LineSegment {
                points: [left, right],
                stroke,
            })
        }
        ui.painter().extend(shapes);

        response
    }
}
