// Copyright (c) 2023 Mike Tsao. All rights reserved.

use eframe::{
    egui::{Frame, Response, Sense, Ui},
    emath::{self, RectTransform},
    epaint::{vec2, Color32, Pos2, Rect, Rounding, Shape, Stroke, Vec2},
};
use groove_core::{
    midi::MidiChannel,
    time::{MusicalTime, TimeSignature},
    traits::{
        gui::Shows, Configurable, ControlMessagesFn, Controls, HandlesMidi, IsController, Performs,
    },
    Uid,
};
use groove_entities::EntityMessage;
use groove_proc_macros::{Control, Params, Uid};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ops::Range};

use super::UidFactory;

#[derive(Debug, Serialize, Deserialize)]
struct ArrangedPattern {
    pattern_uid: Uid,
    start: MusicalTime,
    is_selected: bool,
}
impl Shows for ArrangedPattern {
    fn show(&mut self, ui: &mut eframe::egui::Ui) {
        Frame::canvas(ui.style()).show(ui, |ui| {
            self.ui_content(ui);
        });
    }
}
impl ArrangedPattern {
    fn ui_content(&mut self, ui: &mut Ui) {
        Frame::default()
            .stroke(Stroke::new(
                1.0,
                if self.is_selected {
                    Color32::YELLOW
                } else {
                    Color32::BLUE
                },
            ))
            .show(ui, |ui| ui.label(format!("{}", self.pattern_uid)));
    }

    fn show_in_arrangement(
        &mut self,
        ui: &mut eframe::egui::Ui,
        pattern: &MiniPattern,
    ) -> Response {
        pattern.show_in_arrangement(ui, self.is_selected)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
enum MiniNoteUiState {
    #[default]
    Normal,
    Hovered,
    Selected,
}

/// A [MiniNote] is a single played note. It knows which key it's playing (which
/// is more or less assumed to be a MIDI key value), and when (start/end) it's
/// supposed to play, relative to time zero.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct MiniNote {
    key: u8,
    range: Range<MusicalTime>,

    #[serde(skip)]
    ui_state: MiniNoteUiState,
}

/// A [MiniPattern] contains a musical sequence. It is a series of [MiniNote]s
/// and a [TimeSignature]. All the notes should fit into the pattern's duration.
#[derive(Debug, Serialize, Deserialize)]
struct MiniPattern {
    time_signature: TimeSignature,
    duration: MusicalTime,
    notes: Vec<MiniNote>,
}
impl Default for MiniPattern {
    fn default() -> Self {
        let time_signature = TimeSignature::default();
        let duration = time_signature.duration();
        Self {
            time_signature,
            duration,
            notes: Default::default(),
        }
    }
}
impl Shows for MiniPattern {
    fn show(&mut self, ui: &mut eframe::egui::Ui) {
        Frame::canvas(ui.style()).show(ui, |ui| {
            self.ui_content(ui);
        });
    }
}
impl MiniPattern {
    pub fn add(&mut self, note: MiniNote) {
        self.notes.push(note);
    }

    pub fn remove(&mut self, note: &MiniNote) {
        self.notes.retain(|v| v != note);
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.notes.clear();
    }

    fn make_note_shapes(
        &self,
        note: &MiniNote,
        to_screen: &RectTransform,
        is_highlighted: bool,
    ) -> Vec<Shape> {
        let rect = to_screen
            .transform_rect(self.rect_for_note(note))
            .shrink(1.0);
        let color = if note.ui_state == MiniNoteUiState::Selected {
            Color32::LIGHT_GRAY
        } else if is_highlighted {
            Color32::WHITE
        } else {
            Color32::DARK_BLUE
        };
        vec![
            Shape::rect_stroke(rect, Rounding::default(), Stroke { width: 2.0, color }),
            Shape::rect_filled(rect.shrink(2.0), Rounding::default(), Color32::LIGHT_BLUE),
        ]
    }

    fn rect_for_note(&self, note: &MiniNote) -> Rect {
        let notes_vert = 24.0;
        const FIGURE_THIS_OUT: f32 = 16.0;
        let ul = Pos2 {
            x: note.range.start.total_parts() as f32 / FIGURE_THIS_OUT,
            y: (note.key as f32) / notes_vert,
        };
        let br = Pos2 {
            x: note.range.end.total_parts() as f32 / FIGURE_THIS_OUT,
            y: (1.0 + note.key as f32) / notes_vert,
        };
        Rect::from_two_pos(ul, br)
    }

    fn ui_content(&mut self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        let notes_vert = 24.0;
        let steps_horiz = 16.0;

        let desired_size = ui.available_size_before_wrap();
        let desired_size = Vec2::new(desired_size.x, 256.0);
        let (mut response, painter) = ui.allocate_painter(desired_size, Sense::click());

        let to_screen = emath::RectTransform::from_to(
            Rect::from_min_size(Pos2::ZERO, Vec2::splat(1.0)),
            response.rect,
        );
        let from_screen = to_screen.inverse();

        painter.rect_filled(response.rect, Rounding::default(), Color32::GRAY);
        for i in 0..16 {
            let x = i as f32 / steps_horiz;
            let lines = [to_screen * Pos2::new(x, 0.0), to_screen * Pos2::new(x, 1.0)];
            painter.line_segment(
                lines,
                Stroke {
                    width: 1.0,
                    color: Color32::DARK_GRAY,
                },
            );
        }

        // Are we over any existing note?
        let mut hovered_note = None;
        if let Some(hover_pos) = response.hover_pos() {
            for note in &self.notes {
                let note_rect = to_screen.transform_rect(self.rect_for_note(&note));
                if note_rect.contains(hover_pos) {
                    hovered_note = Some(note.clone());
                    break;
                }
            }
        }

        // Clicking means we add a new note in an empty space, or remove an existing one.
        if response.clicked() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                let note =
                    self.note_for_position(&from_screen, steps_horiz, notes_vert, pointer_pos);
                if let Some(hovered) = hovered_note {
                    let _ = self.remove(&hovered);
                    hovered_note = None;
                } else {
                    let _ = self.add(note);
                }
                response.mark_changed();
            }
        }

        let shapes = self.notes.iter().fold(Vec::default(), |mut v, note| {
            let is_highlighted = if let Some(n) = &hovered_note {
                n == note
            } else {
                false
            };
            v.extend(self.make_note_shapes(note, &to_screen, is_highlighted));
            v
        });

        painter.extend(shapes);

        response
    }

    fn note_for_position(
        &self,
        from_screen: &RectTransform,
        steps_horiz: f32,
        notes_vert: f32,
        pointer_pos: Pos2,
    ) -> MiniNote {
        let canvas_pos = from_screen * pointer_pos;
        let key = (canvas_pos.y * notes_vert) as u8;
        let when = MusicalTime::new_with_parts(((canvas_pos.x * steps_horiz).floor()) as u64);

        MiniNote {
            key,
            range: Range {
                start: when,
                end: when + MusicalTime::new_with_parts(1),
            },
            ui_state: Default::default(),
        }
    }

    pub fn duration(&self) -> MusicalTime {
        self.duration
    }

    fn show_in_arrangement(&self, ui: &mut Ui, is_selected: bool) -> Response {
        let steps_horiz = 16.0;

        let desired_size = vec2((self.duration.total_beats() * 16) as f32, 64.0);
        let (response, painter) = ui.allocate_painter(desired_size, Sense::click());

        let to_screen = emath::RectTransform::from_to(
            Rect::from_min_size(Pos2::ZERO, Vec2::splat(1.0)),
            response.rect,
        );

        painter.rect_filled(response.rect, Rounding::default(), Color32::DARK_GRAY);
        painter.rect_stroke(
            response.rect,
            Rounding::none(),
            Stroke::new(if is_selected { 2.0 } else { 0.0 }, Color32::WHITE),
        );
        for i in 0..16 {
            let x = i as f32 / steps_horiz;
            let lines = [to_screen * Pos2::new(x, 0.0), to_screen * Pos2::new(x, 1.0)];
            painter.line_segment(
                lines,
                Stroke {
                    width: 1.0,
                    color: Color32::DARK_GRAY,
                },
            );
        }

        let shapes = self.notes.iter().fold(Vec::default(), |mut v, note| {
            v.extend(self.make_note_shapes(note, &to_screen, false));
            v
        });

        painter.extend(shapes);

        response
    }
}

#[derive(Debug)]
enum MiniSequencerAction {
    CreatePattern,
    ArrangePattern(Uid),
}

/// [MiniSequencer] converts a chain of [MiniPattern]s into MIDI notes according
/// to a given [Tempo] and [TimeSignature].
#[derive(Debug, Default, Control, Params, Uid, Serialize, Deserialize)]
pub struct MiniSequencer {
    uid: groove_core::Uid,
    midi_channel_out: MidiChannel,

    uid_factory: UidFactory,

    // All the patterns the sequencer knows about. These are not arranged.
    patterns: HashMap<Uid, MiniPattern>,

    arrangement_cursor: MusicalTime,
    arranged_patterns: Vec<ArrangedPattern>,

    // The sequencer should be performing work for this time slice.
    #[serde(skip)]
    range: Range<MusicalTime>,
}
impl MiniSequencer {
    /// Creates a new [MiniSequencer]
    #[allow(unused_variables)]
    pub fn new_with(params: &MiniSequencerParams, midi_channel_out: MidiChannel) -> Self {
        Self {
            midi_channel_out,
            ..Default::default()
        }
    }

    fn append_pattern(&mut self, uid: &Uid) {
        self.arranged_patterns.push(ArrangedPattern {
            pattern_uid: *uid,
            start: self.arrangement_cursor,
            is_selected: false,
        });
        if let Some(pattern) = self.patterns.get(uid) {
            self.arrangement_cursor += pattern.duration();
        }
    }

    fn ui_content(&mut self, ui: &mut Ui) -> Option<MiniSequencerAction> {
        let mut action = None;
        ui.allocate_ui(vec2(ui.available_width(), 128.0), |ui| {
            let patterns = &mut self.patterns;
            if ui.button("Add pattern").clicked() {
                action = Some(MiniSequencerAction::CreatePattern)
            }
            if patterns.is_empty() {
                ui.label("Add a pattern and start editing it");
            } else {
                patterns.iter_mut().for_each(|(uid, p)| {
                    if ui.button("Add to track").clicked() {
                        action = Some(MiniSequencerAction::ArrangePattern(*uid))
                    }
                    p.show(ui);
                });
            }
        });
        action
    }

    /// Renders the arrangement view
    pub fn show_arrangement(&mut self, ui: &mut Ui) -> Response {
        let desired_size = vec2(ui.available_width(), 64.0);
        let (_id, rect) = ui.allocate_space(desired_size);
        let painter = ui.painter_at(rect);

        let to_screen =
            emath::RectTransform::from_to(Rect::from_min_size(Pos2::ZERO, Vec2::splat(1.0)), rect);

        painter.rect_filled(rect, Rounding::default(), Color32::GRAY);
        for i in 0..16 {
            let x = i as f32 / 16.0;
            let lines = [to_screen * Pos2::new(x, 0.0), to_screen * Pos2::new(x, 1.0)];
            painter.line_segment(
                lines,
                Stroke {
                    width: 1.0,
                    color: Color32::DARK_GRAY,
                },
            );
        }

        ui.allocate_ui_at_rect(rect, |ui| {
            ui.style_mut().spacing.item_spacing = Vec2::ZERO;
            ui.horizontal_top(|ui| {
                for arranged_pattern in self.arranged_patterns.iter_mut() {
                    if let Some(pattern) = self.patterns.get(&arranged_pattern.pattern_uid) {
                        if arranged_pattern.show_in_arrangement(ui, pattern).clicked() {
                            // TODO: handle shift/control
                            arranged_pattern.is_selected = !arranged_pattern.is_selected;
                        }
                    }
                }
            })
            .response
        })
        .inner
    }

    /// Removes all selected arranged patterns.
    pub fn remove_selected_patterns(&mut self) {
        self.arranged_patterns.retain(|p| !p.is_selected);
    }
}
impl IsController for MiniSequencer {}
impl Shows for MiniSequencer {
    fn show(&mut self, ui: &mut Ui) {
        if let Some(action) = self.ui_content(ui) {
            match action {
                MiniSequencerAction::CreatePattern => {
                    self.patterns
                        .insert(self.uid_factory.next(), MiniPattern::default());
                }
                MiniSequencerAction::ArrangePattern(uid) => self.append_pattern(&uid),
            }
        }
    }
}
impl Performs for MiniSequencer {
    fn play(&mut self) {
        todo!()
    }

    fn stop(&mut self) {
        todo!()
    }

    fn skip_to_start(&mut self) {
        todo!()
    }

    fn is_performing(&self) -> bool {
        todo!()
    }
}
impl HandlesMidi for MiniSequencer {}
impl Controls for MiniSequencer {
    type Message = EntityMessage;

    fn update_time(&mut self, range: &std::ops::Range<MusicalTime>) {
        self.range = range.clone();
    }

    fn work(&mut self, _: &mut ControlMessagesFn<Self::Message>) {
        // TODO
    }

    fn is_finished(&self) -> bool {
        true
    }
}
impl Configurable for MiniSequencer {}
