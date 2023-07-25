// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::{selection_set::SelectionSet, UidFactory};
use anyhow::anyhow;
use btreemultimap::BTreeMultiMap;
use derive_builder::Builder;
use eframe::{
    egui::{Frame, Response, Sense, Ui},
    emath::{self, RectTransform},
    epaint::{pos2, vec2, Color32, Pos2, Rect, Rounding, Shape, Stroke, Vec2},
};
use groove_core::{
    midi::{new_note_off, new_note_on, MidiChannel, MidiMessage},
    time::{MusicalTime, TimeSignature},
    traits::{
        gui::Shows, Configurable, ControlEventsFn, Controls, HandlesMidi, Performs, Serializable,
    },
    IsUid, Uid,
};
use groove_proc_macros::{Control, IsController, Params, Uid};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display, ops::Range};

/// Identifies a [Pattern].
#[derive(
    Copy, Clone, Debug, Serialize, Deserialize, Default, Eq, PartialEq, Ord, PartialOrd, Hash,
)]
pub struct PatternUid(pub usize);
impl IsUid for PatternUid {
    fn increment(&mut self) -> &Self {
        self.0 += 1;
        self
    }
}
impl Display for PatternUid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

/// Identifies an arrangement of a [Pattern].
#[derive(
    Copy, Clone, Debug, Serialize, Deserialize, Default, Eq, PartialEq, Ord, PartialOrd, Hash,
)]
pub struct ArrangedPatternUid(pub usize);
impl IsUid for ArrangedPatternUid {
    fn increment(&mut self) -> &Self {
        self.0 += 1;
        self
    }
}
impl Display for ArrangedPatternUid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ArrangedPattern {
    pattern_uid: PatternUid,
    position: MusicalTime,
}
impl ArrangedPattern {
    fn ui_content(&self, ui: &mut Ui, pattern: &Pattern, is_selected: bool) -> Response {
        let steps_horiz = pattern.time_signature.bottom * 4;

        let desired_size = vec2((pattern.duration.total_beats() * 16) as f32, 64.0);
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
        let steps_horiz_f32 = steps_horiz as f32;
        for i in 0..steps_horiz {
            let x = i as f32 / steps_horiz_f32;
            let lines = [to_screen * Pos2::new(x, 0.0), to_screen * Pos2::new(x, 1.0)];
            painter.line_segment(
                lines,
                Stroke {
                    width: 1.0,
                    color: Color32::DARK_GRAY,
                },
            );
        }

        let shapes = pattern.notes.iter().fold(Vec::default(), |mut v, note| {
            v.extend(pattern.make_note_shapes(note, &to_screen, false));
            v
        });

        painter.extend(shapes);

        response
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
enum NoteUiState {
    #[default]
    Normal,
    Hovered,
    Selected,
}

/// A [Note] is a single played note. It knows which key it's playing (which
/// is more or less assumed to be a MIDI key value), and when (start/end) it's
/// supposed to play, relative to time zero.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Note {
    key: u8,
    range: Range<MusicalTime>,

    #[serde(skip)]
    ui_state: NoteUiState,
}

/// A [Pattern] contains a musical sequence that is suitable for
/// pattern-based composition. It is a series of [Note]s and a
/// [TimeSignature]. All the notes should fit into the pattern's duration, and
/// the duration should be a round multiple of the length implied by the time
/// signature.
#[derive(Debug, Serialize, Deserialize, Builder)]
#[builder(build_fn(private, name = "build_from_builder"))]
struct Pattern {
    #[builder(default)]
    time_signature: TimeSignature,

    /// The duration is the amount of time from the start of the pattern to the
    /// point when the next pattern should start. This does not necessarily mean
    /// the time between the first note-on and the first note-off! For example,
    /// an empty 4/4 pattern lasts for 4 beats.
    #[builder(setter(skip))]
    duration: MusicalTime,

    #[builder(default, setter(each(name = "note", into)))]
    notes: Vec<Note>,
}
impl PatternBuilder {
    pub fn build(&self) -> Result<Pattern, PatternBuilderError> {
        match self.build_from_builder() {
            Ok(mut s) => {
                s.post_build();
                Ok(s)
            }
            Err(e) => Err(e),
        }
    }

    /// Given a sequence of MIDI note numbers and an optional grid value that
    /// overrides the one implied by the time signature, adds [Note]s one after
    /// another into the pattern. The value 255 is reserved for rest (no note,
    /// or silence).
    ///
    /// The optional grid_value is similar to the time signature's bottom value
    /// (1 is a whole note, 2 is a half note, etc.). For example, for a 4/4
    /// pattern, None means each note number produces a quarter note, and we
    /// would provide sixteen note numbers to fill the pattern with 4 beats of
    /// four quarter-notes each. For a 4/4 pattern, Some(8) means each note
    /// number should produce an eighth note., and 4 x 8 = 32 note numbers would
    /// fill the pattern.
    ///
    /// If midi_note_numbers contains fewer than the maximum number of note
    /// numbers for the grid value, then the rest of the pattern is silent.
    pub fn note_sequence(
        &mut self,
        midi_note_numbers: Vec<u8>,
        grid_value: Option<usize>,
    ) -> &mut Self {
        let grid_value = grid_value.unwrap_or(self.time_signature.unwrap_or_default().bottom);
        let mut position = MusicalTime::START;
        let position_delta = MusicalTime::new_with_fractional_beats(1.0 / grid_value as f64);
        for note in midi_note_numbers {
            if note != 255 {
                self.note(Note {
                    key: note,
                    range: position..position + position_delta,
                    ui_state: Default::default(),
                });
            }
            position += position_delta;
        }
        self
    }
}
impl Default for Pattern {
    fn default() -> Self {
        let mut r = Self {
            time_signature: TimeSignature::default(),
            duration: Default::default(),
            notes: Default::default(),
        };
        r.post_build();
        r
    }
}
impl Shows for Pattern {
    fn show(&mut self, ui: &mut Ui) {
        Frame::canvas(ui.style()).show(ui, |ui| {
            self.ui_content(ui);
        });
    }
}
impl Pattern {
    fn post_build(&mut self) {
        self.refresh_internals();
    }

    /// Returns the number of notes in the pattern.
    #[allow(dead_code)]
    pub fn note_count(&self) -> usize {
        self.notes.len()
    }

    /// Returns the pattern grid's number of subdivisions, which is calculated
    /// from the time signature. The number is simply the time signature's top x
    /// bottom. For example, a 3/4 pattern will have 12 subdivisions (three
    /// beats per measure, each beat divided into four quarter notes).
    ///
    /// This is just a UI default and doesn't affect the actual granularity of a
    /// note position.
    pub fn default_grid_value(&self) -> usize {
        self.time_signature.top * self.time_signature.bottom
    }

    fn refresh_internals(&mut self) {
        let final_event_time = self
            .notes
            .iter()
            .map(|n| n.range.end)
            .max()
            .unwrap_or_default();

        // This is how we deal with Range<> being inclusive start, exclusive
        // end. It matters because we want the calculated duration to be rounded
        // up to the next measure, but we don't want a note-off event right on
        // the edge to extend that calculation to include another bar.
        let final_event_time = if final_event_time == MusicalTime::START {
            final_event_time
        } else {
            final_event_time - MusicalTime::new_with_units(1)
        };
        let beats = final_event_time.total_beats();
        let top = self.time_signature.top;
        let rounded_up_bars = (beats + top) / top;
        self.duration = MusicalTime::new_with_bars(&self.time_signature, rounded_up_bars);
    }

    pub fn add_note(&mut self, note: Note) {
        self.notes.push(note);
        self.refresh_internals();
    }

    pub fn remove_note(&mut self, note: &Note) {
        self.notes.retain(|v| v != note);
        self.refresh_internals();
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.notes.clear();
        self.refresh_internals();
    }

    fn make_note_shapes(
        &self,
        note: &Note,
        to_screen: &RectTransform,
        is_highlighted: bool,
    ) -> Vec<Shape> {
        let rect = to_screen
            .transform_rect(self.rect_for_note(note))
            .shrink(1.0);
        let color = if note.ui_state == NoteUiState::Selected {
            Color32::LIGHT_GRAY
        } else if is_highlighted {
            Color32::WHITE
        } else {
            Color32::DARK_BLUE
        };
        let rect = if (rect.right() - rect.left()).abs() < 1.0 {
            Rect::from_two_pos(rect.left_top(), pos2(rect.left() + 1.0, rect.bottom()))
        } else {
            rect
        };
        let rect = if (rect.bottom() - rect.top()).abs() < 1.0 {
            Rect::from_two_pos(rect.left_top(), pos2(rect.right(), rect.top() + 1.0))
        } else {
            rect
        };
        debug_assert!(rect.area() > 0.0);
        vec![
            Shape::rect_stroke(rect, Rounding::default(), Stroke { width: 2.0, color }),
            Shape::rect_filled(rect.shrink(2.0), Rounding::default(), Color32::LIGHT_BLUE),
        ]
    }

    fn rect_for_note(&self, note: &Note) -> Rect {
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

    fn ui_content(&mut self, ui: &mut Ui) -> eframe::egui::Response {
        let notes_vert = 24.0;
        let steps_horiz = self.time_signature.bottom * 4;

        let desired_size = ui.available_size_before_wrap();
        let desired_size = Vec2::new(desired_size.x, 256.0);
        let (mut response, painter) = ui.allocate_painter(desired_size, Sense::click());

        let to_screen = emath::RectTransform::from_to(
            Rect::from_min_size(Pos2::ZERO, Vec2::splat(1.0)),
            response.rect,
        );
        let from_screen = to_screen.inverse();

        painter.rect_filled(response.rect, Rounding::default(), Color32::GRAY);
        for i in 0..steps_horiz {
            let x = i as f32 / steps_horiz as f32;
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
                    let _ = self.remove_note(&hovered);
                    hovered_note = None;
                } else {
                    let _ = self.add_note(note);
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
        steps_horiz: usize,
        notes_vert: f32,
        pointer_pos: Pos2,
    ) -> Note {
        let canvas_pos = from_screen * pointer_pos;
        let key = (canvas_pos.y * notes_vert) as u8;
        let when =
            MusicalTime::new_with_parts(((canvas_pos.x * steps_horiz as f32).floor()) as usize);

        Note {
            key,
            range: Range {
                start: when,
                end: when + MusicalTime::new_with_parts(1),
            },
            ui_state: Default::default(),
        }
    }

    #[allow(dead_code)]
    /// This pattern's duration in [MusicalTime].
    pub fn duration(&self) -> MusicalTime {
        self.duration
    }

    #[allow(dead_code)]
    fn move_note(&mut self, note: &Note, new_start: MusicalTime) {
        self.notes.iter_mut().filter(|n| n == &note).for_each(|n| {
            let n_length = n.range.end - n.range.start;
            n.range = new_start..new_start + n_length;
        });
        self.refresh_internals();
    }

    #[allow(dead_code)]
    fn move_and_resize_note(&mut self, note: &Note, new_start: MusicalTime, duration: MusicalTime) {
        self.notes.iter_mut().filter(|n| n == &note).for_each(|n| {
            n.range = new_start..new_start + duration;
        });
        self.refresh_internals();
    }

    #[allow(dead_code)]
    fn time_signature(&self) -> TimeSignature {
        self.time_signature
    }
}

#[derive(Debug)]
pub enum SequencerAction {
    CreatePattern,
    ArrangePatternAppend(PatternUid),
    ToggleArrangedPatternSelection(ArrangedPatternUid),
}

#[derive(Debug, Default)]
pub struct SequencerEphemerals {
    // The sequencer should be performing work for this time slice.
    range: Range<MusicalTime>,
    // The actual events that the sequencer emits. These are composed of arranged patterns.
    events: BTreeMultiMap<MusicalTime, MidiMessage>,
    // The latest end time (exclusive) of all the events.
    final_event_time: MusicalTime,
    // The next place to insert a pattern.
    arrangement_cursor: MusicalTime,
    // Whether we're performing, in the [Performs] sense.
    is_performing: bool,
}

/// [Sequencer] converts a chain of [Pattern]s into MIDI notes according
/// to a given [Tempo] and [TimeSignature].
#[derive(Debug, Default, Control, IsController, Params, Uid, Serialize, Deserialize, Builder)]
pub struct Sequencer {
    #[builder(default)]
    uid: Uid,
    #[builder(default)]
    midi_channel_out: MidiChannel,

    #[builder(default)]
    time_signature: TimeSignature,

    #[builder(setter(skip))]
    uid_factory: UidFactory<PatternUid>,
    #[builder(setter(skip))]
    arranged_pattern_uid_factory: UidFactory<ArrangedPatternUid>,

    // All the patterns the sequencer knows about. These are not arranged.
    #[builder(setter(skip))]
    patterns: HashMap<PatternUid, Pattern>,

    #[builder(setter(skip))]
    arranged_patterns: HashMap<ArrangedPatternUid, ArrangedPattern>,

    #[builder(setter(skip))]
    arranged_pattern_selection_set: SelectionSet<ArrangedPatternUid>,

    #[serde(skip)]
    #[builder(setter(skip))]
    e: SequencerEphemerals,
}
impl Sequencer {
    fn next_arrangement_position(&self) -> MusicalTime {
        self.e.arrangement_cursor
    }

    #[allow(dead_code)]
    fn pattern_by_uid(&self, uid: &PatternUid) -> Option<&Pattern> {
        self.patterns.get(uid)
    }

    #[allow(dead_code)]
    fn arranged_pattern_by_uid(&self, uid: &ArrangedPatternUid) -> Option<&ArrangedPattern> {
        self.arranged_patterns.get(uid)
    }

    #[allow(dead_code)]
    fn shift_arranged_pattern_left(&mut self, uid: &ArrangedPatternUid) -> anyhow::Result<()> {
        if let Some(ap) = self.arranged_patterns.get_mut(uid) {
            if ap.position >= MusicalTime::DURATION_WHOLE {
                ap.position -= MusicalTime::DURATION_WHOLE;
            }
            Ok(())
        } else {
            Err(anyhow!("Couldn't find pattern {uid}"))
        }
    }

    #[allow(dead_code)]
    fn shift_arranged_pattern_right(&mut self, uid: &ArrangedPatternUid) -> anyhow::Result<()> {
        if let Some(ap) = self.arranged_patterns.get_mut(uid) {
            ap.position += MusicalTime::DURATION_WHOLE;
            Ok(())
        } else {
            Err(anyhow!("Couldn't find pattern {uid}"))
        }
    }

    fn add_pattern(&mut self, pattern: Pattern) -> PatternUid {
        let uid = self.uid_factory.next();
        self.patterns.insert(uid, pattern);
        uid
    }

    fn arrange_pattern_append(&mut self, uid: &PatternUid) -> anyhow::Result<ArrangedPatternUid> {
        if let Ok(apuid) = self.arrange_pattern(
            uid,
            self.next_arrangement_position().bars(&self.time_signature) as usize,
        ) {
            if let Some(pattern) = self.patterns.get(uid) {
                self.e.arrangement_cursor += pattern.duration;
            }
            Ok(apuid)
        } else {
            Err(anyhow!("something went wrong"))
        }
    }

    fn arrange_pattern(
        &mut self,
        uid: &PatternUid,
        position_in_bars: usize,
    ) -> anyhow::Result<ArrangedPatternUid> {
        let position = MusicalTime::new_with_bars(&self.time_signature, position_in_bars);
        if self.patterns.get(uid).is_some() {
            let arranged_pattern_uid = self.arranged_pattern_uid_factory.next();
            self.arranged_patterns.insert(
                arranged_pattern_uid,
                ArrangedPattern {
                    pattern_uid: *uid,
                    position,
                },
            );
            if let Err(r) = self.calculate_events() {
                Err(r)
            } else {
                Ok(arranged_pattern_uid)
            }
        } else {
            Err(anyhow!("Pattern {uid} not found during arrangement"))
        }
    }

    #[allow(dead_code)]
    fn move_pattern(
        &mut self,
        uid: &ArrangedPatternUid,
        position_in_bars: usize,
    ) -> anyhow::Result<()> {
        let position = MusicalTime::new_with_bars(&self.time_signature, position_in_bars);
        if let Some(pattern) = self.arranged_patterns.get_mut(uid) {
            pattern.position = position;
            self.calculate_events()
        } else {
            Err(anyhow!("Couldn't find arranged pattern {}", uid.0))
        }
    }

    fn ui_content(&mut self, ui: &mut Ui) -> Option<SequencerAction> {
        let mut action = None;
        ui.allocate_ui(vec2(384.0, 128.0), |ui| {
            let patterns = &mut self.patterns;
            if ui.button("Add pattern").clicked() {
                action = Some(SequencerAction::CreatePattern)
            }
            if patterns.is_empty() {
                ui.label("Add a pattern and start editing it");
            } else {
                patterns.iter_mut().for_each(|(uid, p)| {
                    if ui.button("Add to track").clicked() {
                        action = Some(SequencerAction::ArrangePatternAppend(*uid))
                    }
                    p.show(ui);
                });
            }
        });
        action
    }

    /// Renders the track's arrangement view.
    #[must_use]
    pub fn ui_arrangement(
        &mut self,
        ui: &mut Ui,
        viewable_time_range: &Range<MusicalTime>,
    ) -> (Response, Option<SequencerAction>) {
        let desired_size = vec2(ui.available_width(), 64.0);
        let (_id, rect) = ui.allocate_space(desired_size);
        let painter = ui.painter_at(rect);

        let start_beat = viewable_time_range.start.total_beats();
        let end_beat = viewable_time_range.end.total_beats();
        let to_screen = emath::RectTransform::from_to(
            Rect::from_x_y_ranges(start_beat as f32..=end_beat as f32, 0.0..=1.0),
            rect,
        );

        painter.rect_filled(rect, Rounding::default(), Color32::GRAY);

        // This is a near copy of the label code in
        // Orchestrator::ui_arrangement_labels(). TODO refactor
        let start_beat = viewable_time_range.start.total_beats();
        let end_beat = viewable_time_range.end.total_beats();
        let beat_count = (end_beat - start_beat) as usize;
        let to_screen_beats = emath::RectTransform::from_to(
            Rect::from_x_y_ranges(
                viewable_time_range.start.total_beats() as f32
                    ..=viewable_time_range.end.total_beats() as f32,
                0.0..=1.0,
            ),
            rect,
        );

        let skip = self.time_signature.top;
        let mut shapes = Vec::default();
        for (i, beat) in (start_beat..end_beat).enumerate() {
            if i != 0 && i != beat_count - 1 && i % skip != 0 {
                continue;
            }
            shapes.push(Shape::LineSegment {
                points: [
                    to_screen_beats * pos2(beat as f32, 0.0),
                    to_screen_beats * pos2(beat as f32, 1.0),
                ],
                stroke: Stroke {
                    width: 1.0,
                    color: Color32::DARK_GRAY,
                },
            });
        }
        painter.extend(shapes);

        for (arranged_pattern_uid, arranged_pattern) in self.arranged_patterns.iter() {
            if let Some(pattern) = self.patterns.get(&arranged_pattern.pattern_uid) {
                let start = arranged_pattern.position;
                let end = start + pattern.duration;
                let start_beats = start.total_beats();
                let end_beats = end.total_beats();

                let ap_rect = Rect::from_two_pos(
                    to_screen * pos2(start_beats as f32, 0.0),
                    to_screen * pos2(end_beats as f32, 1.0),
                );
                let to_screen_ap = emath::RectTransform::from_to(
                    Rect::from_x_y_ranges(0.0..=1.0, 0.0..=1.0),
                    ap_rect,
                );
                painter.rect_filled(ap_rect, Rounding::default(), Color32::LIGHT_BLUE);

                let shapes = pattern.notes.iter().fold(Vec::default(), |mut v, note| {
                    v.extend(pattern.make_note_shapes(note, &to_screen_ap, false));
                    v
                });

                painter.extend(shapes);

                // if arranged_pattern
                //     .ui_content(
                //         ui,
                //         pattern,
                //         self.arranged_pattern_selection_set
                //             .contains(arranged_pattern_uid),
                //     )
                //     .clicked()
                // {
                //     // TODO: handle shift/control
                //     uid_to_toggle = Some(*arranged_pattern_uid);
                // }
            }
        }

        (ui.allocate_ui_at_rect(rect, |ui| {}).response, None)
    }

    /// Renders the arrangement view.
    #[must_use]
    pub fn show_arrangement(&mut self, ui: &mut Ui) -> (Response, Option<SequencerAction>) {
        let action = None;
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

        (
            ui.allocate_ui_at_rect(rect, |ui| {
                ui.style_mut().spacing.item_spacing = Vec2::ZERO;
                ui.horizontal_top(|ui| {
                    let mut uid_to_toggle = None;
                    for (arranged_pattern_uid, arranged_pattern) in self.arranged_patterns.iter() {
                        if let Some(pattern) = self.patterns.get(&arranged_pattern.pattern_uid) {
                            if arranged_pattern
                                .ui_content(
                                    ui,
                                    pattern,
                                    self.arranged_pattern_selection_set
                                        .contains(arranged_pattern_uid),
                                )
                                .clicked()
                            {
                                // TODO: handle shift/control
                                uid_to_toggle = Some(*arranged_pattern_uid);
                            }
                        }
                    }
                    if let Some(uid) = uid_to_toggle {
                        self.toggle_arranged_pattern_selection(&uid);
                    }
                })
                .response
            })
            .inner,
            action,
        )
    }

    /// Removes all selected arranged patterns.
    pub fn remove_selected_arranged_patterns(&mut self) {
        self.arranged_patterns
            .retain(|uid, _ap| !self.arranged_pattern_selection_set.contains(uid));
        self.arranged_pattern_selection_set.clear();
    }

    fn calculate_events(&mut self) -> anyhow::Result<()> {
        self.e.events.clear();
        self.e.final_event_time = MusicalTime::default();
        for ap in self.arranged_patterns.values() {
            let uid = ap.pattern_uid;
            if let Some(pattern) = self.patterns.get(&uid) {
                for note in &pattern.notes {
                    self.e
                        .events
                        .insert(ap.position + note.range.start, new_note_on(note.key, 127));
                    let end_time = ap.position + note.range.end;
                    if end_time > self.e.final_event_time {
                        self.e.final_event_time = end_time;
                    }
                    self.e.events.insert(end_time, new_note_off(note.key, 0));
                }
            } else {
                return Err(anyhow!(
                    "Pattern {uid} not found during event recalculation"
                ));
            }
        }
        Ok(())
    }

    fn toggle_arranged_pattern_selection(&mut self, uid: &ArrangedPatternUid) {
        if self.arranged_pattern_selection_set.contains(uid) {
            self.arranged_pattern_selection_set.remove(uid);
        } else {
            self.arranged_pattern_selection_set.insert(*uid);
        }
    }

    #[allow(dead_code)]
    fn remove_arranged_pattern(&mut self, uid: &ArrangedPatternUid) {
        self.arranged_patterns.remove(uid);
    }
}
impl Shows for Sequencer {
    fn show(&mut self, ui: &mut Ui) {
        if let Some(action) = self.ui_content(ui) {
            match action {
                SequencerAction::CreatePattern => {
                    self.add_pattern(PatternBuilder::default().build().unwrap());
                }
                SequencerAction::ArrangePatternAppend(uid) => {
                    if let Err(e) = self.arrange_pattern_append(&uid) {
                        eprintln!("while appending arranged pattern: {e}");
                    }
                }
                SequencerAction::ToggleArrangedPatternSelection(uid) => {
                    self.toggle_arranged_pattern_selection(&uid);
                }
            }
        }
    }
}
impl Performs for Sequencer {
    fn play(&mut self) {
        self.e.is_performing = true;
    }

    fn stop(&mut self) {
        self.e.is_performing = false;
    }

    fn skip_to_start(&mut self) {}

    fn is_performing(&self) -> bool {
        self.e.is_performing
    }
}
impl HandlesMidi for Sequencer {}
impl Controls for Sequencer {
    fn update_time(&mut self, range: &std::ops::Range<MusicalTime>) {
        self.e.range = range.clone();
    }

    fn work(&mut self, control_events_fn: &mut ControlEventsFn) {
        let events = self.e.events.range(self.e.range.start..self.e.range.end);
        for event in events {
            control_events_fn(
                self.uid,
                groove_core::traits::ThingEvent::Midi(MidiChannel(0), *event.1),
            );
        }
    }

    fn is_finished(&self) -> bool {
        // both these are exclusive range bounds
        self.e.range.end >= self.e.final_event_time
    }
}
impl Configurable for Sequencer {}
impl Serializable for Sequencer {
    fn after_deser(&mut self) {
        let _ = self.calculate_events();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use groove_core::midi::MidiNote;

    impl Note {
        /// half-note
        const TEST_C4: Note = Note {
            key: MidiNote::C4 as u8,
            range: MusicalTime::START..MusicalTime::DURATION_HALF,
            ui_state: NoteUiState::Normal,
        };
        /// whole note
        const TEST_D4: Note = Note {
            key: MidiNote::D4 as u8,
            range: MusicalTime::START..MusicalTime::DURATION_WHOLE,
            ui_state: NoteUiState::Normal,
        };
        /// two whole notes
        const TEST_E4: Note = Note {
            key: MidiNote::E4 as u8,
            range: MusicalTime::START..MusicalTime::DURATION_BREVE,
            ui_state: NoteUiState::Normal,
        };

        fn new_with(key: MidiNote, start: MusicalTime, duration: MusicalTime) -> Self {
            Self {
                key: key as u8,
                range: start..(start + duration),
                ui_state: Default::default(),
            }
        }
    }

    impl Sequencer {
        /// For testing only; adds simple patterns.
        fn populate_pattern(&mut self, pattern_number: usize) -> (PatternUid, usize, MusicalTime) {
            let pattern = match pattern_number {
                0 => PatternBuilder::default()
                    .notes(vec![
                        Note::new_with(
                            MidiNote::C4,
                            MusicalTime::TIME_ZERO,
                            MusicalTime::DURATION_WHOLE,
                        ),
                        Note::new_with(
                            MidiNote::D4,
                            MusicalTime::TIME_END_OF_FIRST_BEAT,
                            MusicalTime::DURATION_WHOLE,
                        ),
                        Note::new_with(
                            MidiNote::E4,
                            MusicalTime::TIME_END_OF_FIRST_BEAT * 2,
                            MusicalTime::DURATION_WHOLE,
                        ),
                    ])
                    .build(),
                1 => PatternBuilder::default()
                    .notes(vec![
                        Note::new_with(
                            MidiNote::C5,
                            MusicalTime::TIME_ZERO,
                            MusicalTime::DURATION_WHOLE,
                        ),
                        Note::new_with(
                            MidiNote::D5,
                            MusicalTime::TIME_END_OF_FIRST_BEAT,
                            MusicalTime::DURATION_WHOLE,
                        ),
                        Note::new_with(
                            MidiNote::E5,
                            MusicalTime::TIME_END_OF_FIRST_BEAT * 2,
                            MusicalTime::DURATION_WHOLE,
                        ),
                    ])
                    .build(),
                _ => panic!(),
            }
            .unwrap();
            let note_count = pattern.notes.len();
            let duration = pattern.duration;
            (self.add_pattern(pattern), note_count, duration)
        }
    }

    #[test]
    fn basic() {
        let s = Sequencer::default();

        assert!(s.patterns.is_empty(), "default sequencer is empty");
        assert!(
            s.arranged_patterns.is_empty(),
            "default sequencer has no arranged patterns"
        );
        assert!(s.e.events.is_empty(), "default sequencer has no events");
    }

    #[test]
    fn pattern_defaults() {
        let p = Pattern::default();
        assert_eq!(p.note_count(), 0, "Default pattern should have zero notes");

        let p = PatternBuilder::default().build().unwrap();
        assert_eq!(
            p.note_count(),
            0,
            "Default built pattern should have zero notes"
        );

        assert_eq!(
            p.time_signature(),
            TimeSignature::COMMON_TIME,
            "Default built pattern should have 4/4 time signature"
        );

        assert_eq!(
            p.duration(),
            MusicalTime::new_with_bars(&TimeSignature::COMMON_TIME, 1),
            "Default built pattern's duration should be one measure"
        );
    }

    #[test]
    fn pattern_one_half_note_is_one_bar() {
        let mut p = PatternBuilder::default().build().unwrap();
        p.add_note(Note::TEST_C4);
        assert_eq!(
            p.duration().total_bars(&p.time_signature()),
            1,
            "Pattern with one half-note should be 1 bar"
        );
    }

    #[test]
    fn pattern_one_breve_is_one_bar() {
        let mut p = PatternBuilder::default().build().unwrap();
        p.add_note(Note::TEST_E4);
        assert_eq!(
            p.duration().total_bars(&p.time_signature()),
            1,
            "Pattern with one note of length breve should be 1 bar"
        );
    }

    #[test]
    fn pattern_one_long_note_is_one_bar() {
        let p = PatternBuilder::default()
            .note(Note::new_with(
                MidiNote::C0,
                MusicalTime::new_with_beats(0),
                MusicalTime::new_with_beats(4),
            ))
            .build()
            .unwrap();
        assert_eq!(
            p.duration().total_bars(&p.time_signature()),
            1,
            "Pattern with a single bar-long note is one bar"
        );
    }

    #[test]
    fn pattern_one_beat_with_1_4_time_signature_is_one_bar() {
        let p = PatternBuilder::default()
            .time_signature(TimeSignature::new_with(1, 4).unwrap())
            .note(Note::new_with(
                MidiNote::C0,
                MusicalTime::new_with_beats(0),
                MusicalTime::new_with_beats(1),
            ))
            .build()
            .unwrap();
        assert_eq!(
            p.duration().total_bars(&p.time_signature()),
            1,
            "Pattern with a single whole note in 1/4 time is one bar"
        );
    }

    #[test]
    fn pattern_three_half_notes_is_one_bar() {
        let p = PatternBuilder::default()
            .note(Note::new_with(
                MidiNote::C0,
                MusicalTime::new_with_beats(0),
                MusicalTime::DURATION_HALF,
            ))
            .note(Note::new_with(
                MidiNote::C0,
                MusicalTime::new_with_beats(1),
                MusicalTime::DURATION_HALF,
            ))
            .note(Note::new_with(
                MidiNote::C0,
                MusicalTime::new_with_beats(2),
                MusicalTime::DURATION_HALF,
            ))
            .build()
            .unwrap();
        assert_eq!(
            p.duration().total_bars(&p.time_signature()),
            1,
            "Pattern with three half-notes on beat should be 1 bar"
        );
    }

    #[test]
    fn pattern_four_whole_notes_is_one_bar() {
        let p = PatternBuilder::default()
            .note(Note::new_with(
                MidiNote::C0,
                MusicalTime::new_with_beats(0),
                MusicalTime::DURATION_WHOLE,
            ))
            .note(Note::new_with(
                MidiNote::C0,
                MusicalTime::new_with_beats(1),
                MusicalTime::DURATION_WHOLE,
            ))
            .note(Note::new_with(
                MidiNote::C0,
                MusicalTime::new_with_beats(2),
                MusicalTime::DURATION_WHOLE,
            ))
            .note(Note::new_with(
                MidiNote::C0,
                MusicalTime::new_with_beats(3),
                MusicalTime::DURATION_WHOLE,
            ))
            .build()
            .unwrap();
        assert_eq!(
            p.duration().total_bars(&p.time_signature()),
            1,
            "Pattern with four whole notes on beat should be 1 bar"
        );
    }

    #[test]
    fn pattern_five_notes_is_two_bars() {
        let p = PatternBuilder::default()
            .note(Note::new_with(
                MidiNote::C0,
                MusicalTime::new_with_beats(0),
                MusicalTime::DURATION_WHOLE,
            ))
            .note(Note::new_with(
                MidiNote::C0,
                MusicalTime::new_with_beats(1),
                MusicalTime::DURATION_WHOLE,
            ))
            .note(Note::new_with(
                MidiNote::C0,
                MusicalTime::new_with_beats(2),
                MusicalTime::DURATION_WHOLE,
            ))
            .note(Note::new_with(
                MidiNote::C0,
                MusicalTime::new_with_beats(3),
                MusicalTime::DURATION_WHOLE,
            ))
            .note(Note::new_with(
                MidiNote::C0,
                MusicalTime::new_with_beats(4),
                MusicalTime::DURATION_SIXTEENTH,
            ))
            .build()
            .unwrap();
        assert_eq!(
            p.duration().total_bars(&p.time_signature()),
            2,
            "Pattern with four whole notes and then a sixteenth should be 2 bars"
        );
    }

    #[test]
    fn test_patterns() {
        let mut s = Sequencer::default();
        let (pid0, p0_note_count, p0_duration) = s.populate_pattern(0);
        let (pid1, p1_note_count, p1_duration) = s.populate_pattern(1);
        assert_eq!(s.patterns.len(), 2);

        assert!(s.arrange_pattern_append(&pid0).is_ok());
        assert_eq!(s.arranged_patterns.len(), 1, "arranging pattern works");
        assert_eq!(
            p0_duration,
            MusicalTime::new_with_bars(&TimeSignature::default(), 1),
            "arranging pattern leads to correct pattern duration"
        );

        // One event for note-on, one for note-off = two events per note.
        assert_eq!(
            s.e.events.len(),
            p0_note_count * 2,
            "sequencer can schedule multiple simultaneous events"
        );

        assert!(s.arrange_pattern_append(&pid1).is_ok());
        assert_eq!(
            s.arranged_patterns.len(),
            2,
            "arranging multiple patterns works"
        );

        assert_eq!(
            p0_duration + p1_duration,
            MusicalTime::new_with_bars(&TimeSignature::default(), 2),
            "arranging second pattern leads to correct pattern duration"
        );
        assert_eq!(
            s.e.events.len(),
            p0_note_count * 2 + p1_note_count * 2,
            "multiple arranged patterns produces expected number of events"
        );
    }

    #[test]
    fn rearrangement() {
        // Start with empty sequencer
        let mut s = Sequencer::default();
        assert_eq!(s.e.final_event_time, MusicalTime::START);

        // Add a pattern to the palette.
        let (pid0, _, p0_duration) = s.populate_pattern(0);
        assert_eq!(p0_duration, MusicalTime::new_with_beats(4));

        // Arrange that pattern at the cursor location.
        let ap_uid0 = s.arrange_pattern_append(&pid0).unwrap();
        assert_eq!(
            s.e.final_event_time,
            MusicalTime::TIME_END_OF_FIRST_BEAT * 2 + MusicalTime::DURATION_WHOLE,
            "Arranging a pattern properly sets the final event time"
        );

        // Move it to the second bar.
        assert!(s.move_pattern(&ap_uid0, 1).is_ok());
        assert_eq!(
            s.e.final_event_time,
            MusicalTime::new_with_bars(&s.time_signature, 1)
                + MusicalTime::TIME_END_OF_FIRST_BEAT * 2
                + MusicalTime::DURATION_WHOLE,
        );
    }

    #[test]
    fn default_pattern_builder() {
        let p = PatternBuilder::default().build().unwrap();
        assert_eq!(
            p.notes.len(),
            0,
            "Default PatternBuilder yields pattern with zero notes"
        );
        assert_eq!(
            p.duration,
            MusicalTime::new_with_bars(&p.time_signature, 1),
            "Default PatternBuilder yields one-measure pattern"
        );
    }

    #[test]
    fn pattern_api_is_ergonomic() {
        let mut p = PatternBuilder::default()
            .note(Note::TEST_C4.clone())
            .note(Note::TEST_D4.clone())
            .build()
            .unwrap();
        assert_eq!(p.notes.len(), 2, "PatternBuilder can add multiple notes");

        p.add_note(Note::TEST_C4.clone());
        assert_eq!(
            p.notes.len(),
            3,
            "Pattern can add duplicate notes. This is probably not desirable to allow."
        );

        p.move_note(&Note::TEST_C4, MusicalTime::new_with_beats(4));
        assert_eq!(p.notes.len(), 3, "Moving a note doesn't copy or destroy");
        p.remove_note(&Note::TEST_D4);
        assert_eq!(p.notes.len(), 2, "remove_note() removes notes");
        p.remove_note(&Note::TEST_C4);
        assert_eq!(
            p.notes.len(),
            2,
            "remove_note() must specify the note correctly."
        );
        p.remove_note(&Note::new_with(
            MidiNote::C4,
            MusicalTime::new_with_beats(4),
            MusicalTime::DURATION_HALF,
        ));
        assert!(p.notes.is_empty(), "remove_note() removes duplicate notes.");
    }

    #[test]
    fn move_note_inside_pattern() {
        let mut p = PatternBuilder::default().build().unwrap();

        p.add_note(Note::TEST_C4.clone());
        p.move_note(
            &Note::TEST_C4,
            MusicalTime::START + MusicalTime::DURATION_SIXTEENTH,
        );
        assert_eq!(
            p.notes[0].range.start,
            MusicalTime::START + MusicalTime::DURATION_SIXTEENTH,
            "moving a note works"
        );
        assert_eq!(
            p.duration,
            MusicalTime::new_with_beats(4),
            "Moving a note in pattern doesn't change duration"
        );
    }

    #[test]
    fn move_note_outside_pattern() {
        let mut p = PatternBuilder::default().build().unwrap();

        p.add_note(Note::TEST_C4.clone());
        p.move_note(&Note::TEST_C4, MusicalTime::new_with_beats(4));
        assert_eq!(
            p.duration,
            MusicalTime::new_with_beats(4 * 2),
            "Moving a note out of pattern increases duration"
        );
    }

    #[test]
    fn move_and_resize_note() {
        let mut p = PatternBuilder::default().build().unwrap();

        p.add_note(Note::TEST_C4.clone());

        p.move_and_resize_note(
            &Note::TEST_C4,
            MusicalTime::START + MusicalTime::DURATION_EIGHTH,
            MusicalTime::DURATION_WHOLE,
        );
        let expected_range = (MusicalTime::START + MusicalTime::DURATION_EIGHTH)
            ..(MusicalTime::START + MusicalTime::DURATION_EIGHTH + MusicalTime::DURATION_WHOLE);
        assert_eq!(
            p.notes[0].range, expected_range,
            "moving/resizing a note works"
        );
        assert_eq!(
            p.duration,
            MusicalTime::new_with_beats(4),
            "moving/resizing within pattern doesn't change duration"
        );

        p.move_and_resize_note(
            &Note::new_with(
                MidiNote::C4,
                expected_range.start,
                expected_range.end - expected_range.start,
            ),
            MusicalTime::new_with_beats(4),
            MusicalTime::DURATION_WHOLE,
        );
        assert_eq!(
            p.duration,
            MusicalTime::new_with_beats(8),
            "moving/resizing outside current pattern makes the pattern longer"
        );
    }

    #[test]
    fn shift_pattern() {
        let mut s = SequencerBuilder::default().build().unwrap();
        let (puid, _, _) = s.populate_pattern(0);
        let apuid = s.arrange_pattern(&puid, 0).unwrap();
        assert_eq!(
            s.arranged_pattern_by_uid(&apuid).unwrap().position,
            MusicalTime::START
        );

        assert!(s.shift_arranged_pattern_right(&apuid).is_ok());
        assert_eq!(
            s.arranged_pattern_by_uid(&apuid).unwrap().position,
            MusicalTime::DURATION_WHOLE,
            "shift right works"
        );

        assert!(s.shift_arranged_pattern_left(&apuid).is_ok());
        assert_eq!(
            s.arranged_pattern_by_uid(&apuid).unwrap().position,
            MusicalTime::START,
            "nondegenerate shift left works"
        );

        assert!(s.shift_arranged_pattern_left(&apuid).is_ok());
        assert_eq!(
            s.arranged_pattern_by_uid(&apuid).unwrap().position,
            MusicalTime::START,
            "degenerate shift left is a no-op"
        );
    }

    #[test]
    fn removing_arranged_pattern_works() {
        let mut s = SequencerBuilder::default().build().unwrap();
        let (puid0, _, _) = s.populate_pattern(0);

        let uid0 = s.arrange_pattern(&puid0, 0).unwrap();
        assert_eq!(s.arranged_patterns.len(), 1);

        s.remove_arranged_pattern(&uid0);
        assert!(s.arranged_patterns.is_empty());

        let (puid1, _, _) = s.populate_pattern(1);

        let uid1 = s.arrange_pattern(&puid1, 0).unwrap();
        let uid0 = s.arrange_pattern(&puid0, 1).unwrap();
        assert_eq!(s.arranged_patterns.len(), 2);

        s.arranged_pattern_selection_set.click(uid1, false);
        s.remove_selected_arranged_patterns();
        assert_eq!(s.arranged_patterns.len(), 1);

        s.arranged_pattern_selection_set.click(uid0, false);
        s.remove_selected_arranged_patterns();
        assert!(s.arranged_patterns.is_empty());
    }

    #[test]
    fn arranged_pattern_selection_works() {
        let mut s = SequencerBuilder::default().build().unwrap();
        assert!(s.arranged_pattern_selection_set.is_empty());

        let (puid0, _, _) = s.populate_pattern(0);
        let (puid1, _, _) = s.populate_pattern(1);

        let uid0 = s.arrange_pattern(&puid0, 0).unwrap();
        let uid1 = s.arrange_pattern(&puid1, 1).unwrap();

        assert!(s.arranged_pattern_selection_set.is_empty());

        s.arranged_pattern_selection_set.click(uid0, false);
        assert_eq!(s.arranged_pattern_selection_set.len(), 1);
        assert!(s.arranged_pattern_selection_set.contains(&uid0));
        assert!(!s.arranged_pattern_selection_set.contains(&uid1));

        s.arranged_pattern_selection_set.click(uid1, true);
        assert_eq!(s.arranged_pattern_selection_set.len(), 2);
        assert!(s.arranged_pattern_selection_set.contains(&uid0));
        assert!(s.arranged_pattern_selection_set.contains(&uid1));

        s.arranged_pattern_selection_set.click(uid1, true);
        assert_eq!(s.arranged_pattern_selection_set.len(), 1);
        assert!(s.arranged_pattern_selection_set.contains(&uid0));
        assert!(!s.arranged_pattern_selection_set.contains(&uid1));

        s.arranged_pattern_selection_set.click(uid1, false);
        assert_eq!(s.arranged_pattern_selection_set.len(), 1);
        assert!(!s.arranged_pattern_selection_set.contains(&uid0));
        assert!(s.arranged_pattern_selection_set.contains(&uid1));
    }

    #[test]
    fn pattern_dimensions_are_valid() {
        let p = Pattern::default();
        assert_eq!(
            p.time_signature,
            TimeSignature::COMMON_TIME,
            "default pattern should have sensible time signature"
        );

        for ts in vec![
            TimeSignature::COMMON_TIME,
            TimeSignature::CUT_TIME,
            TimeSignature::new_with(7, 64).unwrap(),
        ] {
            let p = PatternBuilder::default()
                .time_signature(ts)
                .build()
                .unwrap();
            assert_eq!(
                p.duration,
                MusicalTime::new_with_beats(ts.top),
                "Pattern's beat count matches its time signature"
            );

            // A typical 4/4 pattern has 16 subdivisions, which is a common
            // pattern resolution in other pattern-based sequencers and piano
            // rolls.
            assert_eq!(p.default_grid_value(), ts.bottom * ts.top,
                "Pattern's default grid value should be the time signature's beat count times its note value");
        }
    }

    #[test]
    fn pattern_note_insertion_is_easy() {
        let sixteen_notes = vec![
            60, 61, 62, 63, 64, 65, 66, 67, 60, 61, 62, 63, 64, 65, 66, 67,
        ];
        let len_16 = sixteen_notes.len();
        let p = PatternBuilder::default()
            .note_sequence(sixteen_notes, None)
            .build()
            .unwrap();
        assert_eq!(p.note_count(), len_16, "sixteen quarter notes");
        assert_eq!(p.notes[15].key, 67);
        assert_eq!(
            p.notes[15].range,
            MusicalTime::DURATION_QUARTER * 15..MusicalTime::DURATION_WHOLE * p.time_signature.top
        );
        assert_eq!(
            p.duration,
            MusicalTime::DURATION_WHOLE * p.time_signature.top
        );

        let seventeen_notes = vec![
            60, 61, 62, 63, 64, 65, 66, 67, 60, 61, 62, 63, 64, 65, 66, 67, 68,
        ];
        let p = PatternBuilder::default()
            .note_sequence(seventeen_notes, None)
            .build()
            .unwrap();
        assert_eq!(
            p.duration,
            MusicalTime::DURATION_WHOLE * p.time_signature.top * 2,
            "17 notes in 4/4 pattern produces two bars"
        );

        let four_notes = vec![60, 61, 62, 63];
        let len_4 = four_notes.len();
        let p = PatternBuilder::default()
            .note_sequence(four_notes, Some(4))
            .build()
            .unwrap();
        assert_eq!(p.note_count(), len_4, "four quarter notes");
        assert_eq!(
            p.duration,
            MusicalTime::DURATION_WHOLE * p.time_signature.top
        );

        let three_notes_and_silence = vec![60, 0, 62, 63];
        let len_3_1 = three_notes_and_silence.len();
        let p = PatternBuilder::default()
            .note_sequence(three_notes_and_silence, Some(4))
            .build()
            .unwrap();
        assert_eq!(p.note_count(), len_3_1, "three quarter notes with one rest");
        assert_eq!(
            p.duration,
            MusicalTime::DURATION_WHOLE * p.time_signature.top
        );

        let eight_notes = vec![60, 61, 62, 63, 64, 65, 66, 67];
        let len_8 = eight_notes.len();
        let p = PatternBuilder::default()
            .time_signature(TimeSignature::CUT_TIME)
            .note_sequence(eight_notes, None)
            .build()
            .unwrap();
        assert_eq!(
            p.note_count(),
            len_8,
            "eight eighth notes in 2/2 time is two bars long"
        );
        assert_eq!(
            p.duration,
            MusicalTime::DURATION_WHOLE * p.time_signature.top * 2
        );

        let one_note = vec![60];
        let len_1 = one_note.len();
        let p = PatternBuilder::default()
            .note_sequence(one_note, None)
            .build()
            .unwrap();
        assert_eq!(
            p.note_count(),
            len_1,
            "one quarter note, and the rest is silence"
        );
        assert_eq!(p.notes[0].key, 60);
        assert_eq!(
            p.notes[0].range,
            MusicalTime::START..MusicalTime::DURATION_QUARTER
        );
        assert_eq!(
            p.duration,
            MusicalTime::DURATION_WHOLE * p.time_signature.top
        );
    }

    #[test]
    fn cut_time_duration() {
        let p = PatternBuilder::default()
            .time_signature(TimeSignature::CUT_TIME)
            .build()
            .unwrap();
        assert_eq!(p.duration, MusicalTime::new_with_beats(2));
    }
}
