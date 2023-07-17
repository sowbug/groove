// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::mini::{Track, TrackAction, TrackUid};
use eframe::{
    egui::{Frame, Ui},
    emath::{self, Align2},
    epaint::{pos2, vec2, Color32, FontId, Rect, Shape, Stroke, Vec2},
};
use groove_core::time::{MusicalTime, TimeSignature};
use std::ops::Range;

/// Renders tracks.
#[derive(Debug, Default)]
pub struct ArrangementView {
    time_signature: TimeSignature,
    viewable_time_range: Range<MusicalTime>,
    size: Vec2,
}
impl ArrangementView {
    /// Main entry point.
    pub fn show<'a>(
        &self,
        ui: &mut Ui,
        tracks: impl Iterator<Item = &'a Track>,
        is_selected_fn: &dyn Fn(TrackUid) -> bool,
        is_control_only_down: bool,
    ) -> Option<TrackAction> {
        let mut action = None;
        Frame::canvas(ui.style())
            .stroke(Stroke {
                width: 2.0,
                color: Color32::RED,
            })
            .show(ui, |ui| {
                let (_id, rect) =
                    ui.allocate_space(vec2(ui.available_width(), ui.available_height()));
                let to_screen = emath::RectTransform::from_to(
                    Rect::from_x_y_ranges(0.0..=1.0, 0.0..=1.0),
                    rect,
                );

                let font_id = FontId::proportional(12.0);
                let beat_count = (self.viewable_time_range.end.total_beats()
                    - self.viewable_time_range.start.total_beats())
                    as usize;
                let skip = if beat_count > 100 {
                    10
                } else if beat_count > 10 {
                    2
                } else {
                    1
                };
                for (i, beat) in (self.viewable_time_range.start.total_beats()
                    ..self.viewable_time_range.end.total_beats())
                    .enumerate()
                {
                    if i != 0 && i != beat_count - 1 && i % skip != 0 {
                        continue;
                    }
                    let percentage = i as f32 / beat_count as f32;
                    let beat_plus_one = beat + 1;
                    ui.painter().text(
                        to_screen * pos2(percentage, 0.1),
                        Align2::CENTER_BOTTOM,
                        format!("{beat_plus_one}"),
                        font_id.clone(),
                        Color32::YELLOW,
                    );
                }
                let mut shapes = vec![];

                let line_points = [to_screen * pos2(0.0, 0.101), to_screen * pos2(1.0, 0.101)];

                shapes.push(Shape::line_segment(
                    line_points,
                    Stroke {
                        color: Color32::YELLOW,
                        width: 1.0,
                    },
                ));
                ui.painter().extend(shapes);

                for track in tracks {
                    let (response, a) = track.show(ui, is_selected_fn(track.uid()));
                    if a.is_some() {
                        action = a;
                    }
                    if response.clicked() {
                        action = Some(TrackAction::Select(track.uid(), is_control_only_down));
                    }
                }
            });
        action
    }

    #[allow(missing_docs)]
    pub fn set_time_signature(&mut self, time_signature: TimeSignature) {
        self.time_signature = time_signature;
    }

    #[allow(missing_docs)]
    pub fn set_viewable_time_range(&mut self, viewable_time_range: Range<MusicalTime>) {
        self.viewable_time_range = viewable_time_range;
    }

    #[allow(missing_docs)]
    pub fn set_size(&mut self, size: Vec2) {
        self.size = size;
    }
}
