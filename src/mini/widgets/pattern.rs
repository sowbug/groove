// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::mini::Note;
use eframe::{
    egui::{Response, Ui},
    emath::RectTransform,
    epaint::{Color32, Rounding, Stroke},
};
use groove_core::{time::MusicalTime, traits::gui::Displays};

/// Wraps an [Icon] as an [eframe::egui::Widget].
pub fn icon(duration: MusicalTime, notes: &[Note]) -> impl eframe::egui::Widget + '_ {
    move |ui: &mut eframe::egui::Ui| Icon::new().duration(duration).notes(notes).ui(ui)
}

/// Displays an iconic representation of a sequence of [Note]s (that might be in
/// a [crate::mini::piano_roll::Pattern]). Intended to be a drag-and-drop
/// source.
#[derive(Debug, Default)]
pub struct Icon<'a> {
    duration: MusicalTime,
    notes: &'a [Note],
}
impl<'a> Icon<'a> {
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
}
impl<'a> Displays for Icon<'a> {
    fn ui(&mut self, ui: &mut Ui) -> Response {
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
