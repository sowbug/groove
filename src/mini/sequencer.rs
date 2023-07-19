// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::UidFactory;
use anyhow::anyhow;
use btreemultimap::BTreeMultiMap;
use derive_builder::Builder;
use eframe::{
    egui::{Frame, Response, Sense, Ui},
    emath::{self, RectTransform},
    epaint::{ahash::HashSet, vec2, Color32, Pos2, Rect, Rounding, Shape, Stroke, Vec2},
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

/// Identifies a [MiniPattern].
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

/// Identifies an arrangement of a [MiniPattern].
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
    fn ui_arrangement(&self, ui: &mut Ui, pattern: &MiniPattern, is_selected: bool) -> Response {
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

/// A [MiniPattern] contains a musical sequence that is suitable for
/// pattern-based composition. It is a series of [MiniNote]s and a
/// [TimeSignature]. All the notes should fit into the pattern's duration, and
/// the duration should be a round multiple of the length implied by the time
/// signature.
#[derive(Debug, Serialize, Deserialize, Builder)]
#[builder(build_fn(private, name = "build_from_builder"))]
struct MiniPattern {
    #[builder(default)]
    time_signature: TimeSignature,

    /// The duration is the amount of time from the start of the pattern to the
    /// point when the next pattern should start. This does not necessarily mean
    /// the time between the first note-on and the first note-off! For example,
    /// an empty 4/4 pattern lasts for 4 beats.
    #[builder(setter(skip))]
    duration: MusicalTime,

    #[builder(default, setter(each(name = "note", into)))]
    notes: Vec<MiniNote>,
}
impl MiniPatternBuilder {
    pub fn build(&self) -> Result<MiniPattern, MiniPatternBuilderError> {
        match self.build_from_builder() {
            Ok(mut s) => {
                s.post_build();
                Ok(s)
            }
            Err(e) => Err(e),
        }
    }
}
impl Default for MiniPattern {
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
impl Shows for MiniPattern {
    fn show(&mut self, ui: &mut Ui) {
        Frame::canvas(ui.style()).show(ui, |ui| {
            self.ui_content(ui);
        });
    }
}
impl MiniPattern {
    fn post_build(&mut self) {
        self.refresh_internals();
    }

    /// Returns the number of notes in the pattern.
    #[allow(dead_code)]
    pub fn note_count(&self) -> usize {
        self.notes.len()
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
        let top = self.time_signature.top as u64;
        let rounded_up_bars = (beats + top) / top;
        self.duration = MusicalTime::new_with_bars(&self.time_signature, rounded_up_bars);
    }

    pub fn add_note(&mut self, note: MiniNote) {
        self.notes.push(note);
        self.refresh_internals();
    }

    pub fn remove_note(&mut self, note: &MiniNote) {
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
    ) -> MiniNote {
        let canvas_pos = from_screen * pointer_pos;
        let key = (canvas_pos.y * notes_vert) as u8;
        let when =
            MusicalTime::new_with_parts(((canvas_pos.x * steps_horiz as f32).floor()) as u64);

        MiniNote {
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
    fn move_note(&mut self, note: &MiniNote, new_start: MusicalTime) {
        self.notes.iter_mut().filter(|n| n == &note).for_each(|n| {
            let n_length = n.range.end - n.range.start;
            n.range = new_start..new_start + n_length;
        });
        self.refresh_internals();
    }

    #[allow(dead_code)]
    fn move_and_resize_note(
        &mut self,
        note: &MiniNote,
        new_start: MusicalTime,
        duration: MusicalTime,
    ) {
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
pub enum MiniSequencerAction {
    CreatePattern,
    ArrangePatternAppend(PatternUid),
    ToggleArrangedPatternSelection(ArrangedPatternUid),
}

#[derive(Debug, Default)]
pub struct MiniSequencerEphemerals {
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

/// [MiniSequencer] converts a chain of [MiniPattern]s into MIDI notes according
/// to a given [Tempo] and [TimeSignature].
#[derive(Debug, Default, Control, IsController, Params, Uid, Serialize, Deserialize, Builder)]
pub struct MiniSequencer {
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
    patterns: HashMap<PatternUid, MiniPattern>,

    #[builder(setter(skip))]
    arranged_patterns: HashMap<ArrangedPatternUid, ArrangedPattern>,

    #[builder(setter(skip))]
    selected_arranged_pattern_uids: HashSet<ArrangedPatternUid>,

    #[serde(skip)]
    #[builder(setter(skip))]
    e: MiniSequencerEphemerals,
}
impl MiniSequencer {
    fn next_arrangement_position(&self) -> MusicalTime {
        self.e.arrangement_cursor
    }

    #[allow(dead_code)]
    fn pattern_by_uid(&self, uid: &PatternUid) -> Option<&MiniPattern> {
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

    fn add_pattern(&mut self, pattern: MiniPattern) -> PatternUid {
        let uid = self.uid_factory.next();
        self.patterns.insert(uid, pattern);
        uid
    }

    fn arrange_pattern_append(&mut self, uid: &PatternUid) -> anyhow::Result<ArrangedPatternUid> {
        self.arrange_pattern(
            uid,
            self.next_arrangement_position().bars(&self.time_signature) as usize,
        )
    }

    fn arrange_pattern(
        &mut self,
        uid: &PatternUid,
        position_in_bars: usize,
    ) -> anyhow::Result<ArrangedPatternUid> {
        let position = MusicalTime::new_with_bars(&self.time_signature, position_in_bars as u64);
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
        let position = MusicalTime::new_with_bars(&self.time_signature, position_in_bars as u64);
        if let Some(pattern) = self.arranged_patterns.get_mut(uid) {
            pattern.position = position;
            self.calculate_events()
        } else {
            Err(anyhow!("Couldn't find arranged pattern {}", uid.0))
        }
    }

    fn ui_content(&mut self, ui: &mut Ui) -> Option<MiniSequencerAction> {
        let mut action = None;
        ui.allocate_ui(vec2(384.0, 128.0), |ui| {
            let patterns = &mut self.patterns;
            if ui.button("Add pattern").clicked() {
                action = Some(MiniSequencerAction::CreatePattern)
            }
            if patterns.is_empty() {
                ui.label("Add a pattern and start editing it");
            } else {
                patterns.iter_mut().for_each(|(uid, p)| {
                    if ui.button("Add to track").clicked() {
                        action = Some(MiniSequencerAction::ArrangePatternAppend(*uid))
                    }
                    p.show(ui);
                });
            }
        });
        action
    }

    /// Renders the arrangement view.
    #[must_use]
    pub fn show_arrangement(&mut self, ui: &mut Ui) -> (Response, Option<MiniSequencerAction>) {
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
                                .ui_arrangement(
                                    ui,
                                    pattern,
                                    self.is_arranged_pattern_selected(arranged_pattern_uid),
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
            .retain(|uid, ap| !self.selected_arranged_pattern_uids.contains(uid));
        self.selected_arranged_pattern_uids.clear();
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
        if self.selected_arranged_pattern_uids.contains(uid) {
            self.selected_arranged_pattern_uids.remove(uid);
        } else {
            self.selected_arranged_pattern_uids.insert(*uid);
        }
    }

    fn select_arranged_pattern(
        &mut self,
        uid: &ArrangedPatternUid,
        selected: bool,
        preserve_selection_set: bool,
    ) {
        if !preserve_selection_set {
            self.selected_arranged_pattern_uids.clear();
        }
        if selected {
            self.selected_arranged_pattern_uids.insert(*uid);
        } else {
            self.selected_arranged_pattern_uids.remove(uid);
        }
    }

    fn is_arranged_pattern_selected(&self, uid: &ArrangedPatternUid) -> bool {
        self.selected_arranged_pattern_uids.contains(uid)
    }

    fn remove_arranged_pattern(&mut self, uid: &ArrangedPatternUid) {
        self.arranged_patterns.remove(uid);
    }
}
impl Shows for MiniSequencer {
    fn show(&mut self, ui: &mut Ui) {
        if let Some(action) = self.ui_content(ui) {
            match action {
                MiniSequencerAction::CreatePattern => {
                    self.add_pattern(MiniPatternBuilder::default().build().unwrap());
                }
                MiniSequencerAction::ArrangePatternAppend(uid) => {
                    if let Err(e) = self.arrange_pattern_append(&uid) {
                        eprintln!("while appending arranged pattern: {e}");
                    }
                }
                MiniSequencerAction::ToggleArrangedPatternSelection(uid) => {
                    self.toggle_arranged_pattern_selection(&uid);
                }
            }
        }
    }
}
impl Performs for MiniSequencer {
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
impl HandlesMidi for MiniSequencer {}
impl Controls for MiniSequencer {
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
impl Configurable for MiniSequencer {}
impl Serializable for MiniSequencer {
    fn after_deser(&mut self) {
        let _ = self.calculate_events();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use groove_core::midi::MidiNote;

    impl MiniNote {
        /// half-note
        const TEST_C4: MiniNote = MiniNote {
            key: MidiNote::C4 as u8,
            range: MusicalTime::START..MusicalTime::DURATION_HALF,
            ui_state: MiniNoteUiState::Normal,
        };
        /// whole note
        const TEST_D4: MiniNote = MiniNote {
            key: MidiNote::D4 as u8,
            range: MusicalTime::START..MusicalTime::DURATION_WHOLE,
            ui_state: MiniNoteUiState::Normal,
        };
        /// two whole notes
        const TEST_E4: MiniNote = MiniNote {
            key: MidiNote::E4 as u8,
            range: MusicalTime::START..MusicalTime::DURATION_BREVE,
            ui_state: MiniNoteUiState::Normal,
        };

        fn new_with(key: MidiNote, start: MusicalTime, duration: MusicalTime) -> Self {
            Self {
                key: key as u8,
                range: start..(start + duration),
                ui_state: Default::default(),
            }
        }
    }

    impl MiniSequencer {
        /// For testing only; adds simple patterns.
        fn populate_pattern(&mut self, pattern_number: usize) -> (PatternUid, usize, MusicalTime) {
            let pattern = match pattern_number {
                0 => MiniPatternBuilder::default()
                    .notes(vec![
                        MiniNote::new_with(
                            MidiNote::C4,
                            MusicalTime::TIME_ZERO,
                            MusicalTime::DURATION_WHOLE,
                        ),
                        MiniNote::new_with(
                            MidiNote::D4,
                            MusicalTime::TIME_END_OF_FIRST_BEAT,
                            MusicalTime::DURATION_WHOLE,
                        ),
                        MiniNote::new_with(
                            MidiNote::E4,
                            MusicalTime::TIME_END_OF_FIRST_BEAT * 2,
                            MusicalTime::DURATION_WHOLE,
                        ),
                    ])
                    .build(),
                1 => MiniPatternBuilder::default()
                    .notes(vec![
                        MiniNote::new_with(
                            MidiNote::C5,
                            MusicalTime::TIME_ZERO,
                            MusicalTime::DURATION_WHOLE,
                        ),
                        MiniNote::new_with(
                            MidiNote::D5,
                            MusicalTime::TIME_END_OF_FIRST_BEAT,
                            MusicalTime::DURATION_WHOLE,
                        ),
                        MiniNote::new_with(
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
        let s = MiniSequencer::default();

        assert!(s.patterns.is_empty(), "default sequencer is empty");
        assert!(
            s.arranged_patterns.is_empty(),
            "default sequencer has no arranged patterns"
        );
        assert!(s.e.events.is_empty(), "default sequencer has no events");
    }

    #[test]
    fn pattern_defaults() {
        let p = MiniPattern::default();
        assert_eq!(p.note_count(), 0, "Default pattern should have zero notes");

        let p = MiniPatternBuilder::default().build().unwrap();
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
        let mut p = MiniPatternBuilder::default().build().unwrap();
        p.add_note(MiniNote::TEST_C4);
        assert_eq!(
            p.duration().total_bars(&p.time_signature()),
            1,
            "Pattern with one half-note should be 1 bar"
        );
    }

    #[test]
    fn pattern_one_breve_is_one_bar() {
        let mut p = MiniPatternBuilder::default().build().unwrap();
        p.add_note(MiniNote::TEST_E4);
        assert_eq!(
            p.duration().total_bars(&p.time_signature()),
            1,
            "Pattern with one note of length breve should be 1 bar"
        );
    }

    #[test]
    fn pattern_one_long_note_is_one_bar() {
        let p = MiniPatternBuilder::default()
            .note(MiniNote::new_with(
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
        let p = MiniPatternBuilder::default()
            .time_signature(TimeSignature::new_with(1, 4).unwrap())
            .note(MiniNote::new_with(
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
        let p = MiniPatternBuilder::default()
            .note(MiniNote::new_with(
                MidiNote::C0,
                MusicalTime::new_with_beats(0),
                MusicalTime::DURATION_HALF,
            ))
            .note(MiniNote::new_with(
                MidiNote::C0,
                MusicalTime::new_with_beats(1),
                MusicalTime::DURATION_HALF,
            ))
            .note(MiniNote::new_with(
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
        let p = MiniPatternBuilder::default()
            .note(MiniNote::new_with(
                MidiNote::C0,
                MusicalTime::new_with_beats(0),
                MusicalTime::DURATION_WHOLE,
            ))
            .note(MiniNote::new_with(
                MidiNote::C0,
                MusicalTime::new_with_beats(1),
                MusicalTime::DURATION_WHOLE,
            ))
            .note(MiniNote::new_with(
                MidiNote::C0,
                MusicalTime::new_with_beats(2),
                MusicalTime::DURATION_WHOLE,
            ))
            .note(MiniNote::new_with(
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
        let p = MiniPatternBuilder::default()
            .note(MiniNote::new_with(
                MidiNote::C0,
                MusicalTime::new_with_beats(0),
                MusicalTime::DURATION_WHOLE,
            ))
            .note(MiniNote::new_with(
                MidiNote::C0,
                MusicalTime::new_with_beats(1),
                MusicalTime::DURATION_WHOLE,
            ))
            .note(MiniNote::new_with(
                MidiNote::C0,
                MusicalTime::new_with_beats(2),
                MusicalTime::DURATION_WHOLE,
            ))
            .note(MiniNote::new_with(
                MidiNote::C0,
                MusicalTime::new_with_beats(3),
                MusicalTime::DURATION_WHOLE,
            ))
            .note(MiniNote::new_with(
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
        let mut s = MiniSequencer::default();
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
        let mut s = MiniSequencer::default();
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
        let p = MiniPatternBuilder::default().build().unwrap();
        assert_eq!(
            p.notes.len(),
            0,
            "Default MiniPatternBuilder yields pattern with zero notes"
        );
        assert_eq!(
            p.duration,
            MusicalTime::new_with_bars(&p.time_signature, 1),
            "Default MiniPatternBuilder yields one-measure pattern"
        );
    }

    #[test]
    fn pattern_api_is_ergonomic() {
        let mut p = MiniPatternBuilder::default()
            .note(MiniNote::TEST_C4.clone())
            .note(MiniNote::TEST_D4.clone())
            .build()
            .unwrap();
        assert_eq!(
            p.notes.len(),
            2,
            "MiniPatternBuilder can add multiple notes"
        );

        p.add_note(MiniNote::TEST_C4.clone());
        assert_eq!(
            p.notes.len(),
            3,
            "MiniPattern can add duplicate notes. This is probably not desirable to allow."
        );

        p.move_note(&MiniNote::TEST_C4, MusicalTime::new_with_beats(4));
        assert_eq!(p.notes.len(), 3, "Moving a note doesn't copy or destroy");
        p.remove_note(&MiniNote::TEST_D4);
        assert_eq!(p.notes.len(), 2, "remove_note() removes notes");
        p.remove_note(&MiniNote::TEST_C4);
        assert_eq!(
            p.notes.len(),
            2,
            "remove_note() must specify the note correctly."
        );
        p.remove_note(&MiniNote::new_with(
            MidiNote::C4,
            MusicalTime::new_with_beats(4),
            MusicalTime::DURATION_HALF,
        ));
        assert!(p.notes.is_empty(), "remove_note() removes duplicate notes.");
    }

    #[test]
    fn move_note_inside_pattern() {
        let mut p = MiniPatternBuilder::default().build().unwrap();

        p.add_note(MiniNote::TEST_C4.clone());
        p.move_note(
            &MiniNote::TEST_C4,
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
        let mut p = MiniPatternBuilder::default().build().unwrap();

        p.add_note(MiniNote::TEST_C4.clone());
        p.move_note(&MiniNote::TEST_C4, MusicalTime::new_with_beats(4));
        assert_eq!(
            p.duration,
            MusicalTime::new_with_beats(4 * 2),
            "Moving a note out of pattern increases duration"
        );
    }

    #[test]
    fn move_and_resize_note() {
        let mut p = MiniPatternBuilder::default().build().unwrap();

        p.add_note(MiniNote::TEST_C4.clone());

        p.move_and_resize_note(
            &MiniNote::TEST_C4,
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
            &MiniNote::new_with(
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
        let mut s = MiniSequencerBuilder::default().build().unwrap();
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
        let mut s = MiniSequencerBuilder::default().build().unwrap();
        let (puid0, _, _) = s.populate_pattern(0);

        let uid0 = s.arrange_pattern(&puid0, 0).unwrap();
        assert_eq!(s.arranged_patterns.len(), 1);

        s.remove_arranged_pattern(&uid0);
        assert!(s.arranged_patterns.is_empty());

        let (puid1, _, _) = s.populate_pattern(1);

        let uid1 = s.arrange_pattern(&puid1, 0).unwrap();
        let uid0 = s.arrange_pattern(&puid0, 1).unwrap();
        assert_eq!(s.arranged_patterns.len(), 2);

        s.select_arranged_pattern(&uid1, true, false);
        s.remove_selected_arranged_patterns();
        assert_eq!(s.arranged_patterns.len(), 1);

        s.select_arranged_pattern(&uid0, true, false);
        s.remove_selected_arranged_patterns();
        assert!(s.arranged_patterns.is_empty());
    }

    #[test]
    fn arranged_pattern_selection_works() {
        let mut s = MiniSequencerBuilder::default().build().unwrap();
        assert!(s.selected_arranged_pattern_uids.is_empty());

        let (puid0, _, _) = s.populate_pattern(0);
        let (puid1, _, _) = s.populate_pattern(1);

        let uid0 = s.arrange_pattern(&puid0, 0).unwrap();
        let uid1 = s.arrange_pattern(&puid1, 1).unwrap();

        assert!(s.selected_arranged_pattern_uids.is_empty());

        s.select_arranged_pattern(&uid0, true, false);
        assert_eq!(s.selected_arranged_pattern_uids.len(), 1);
        assert!(s.is_arranged_pattern_selected(&uid0));
        assert!(!s.is_arranged_pattern_selected(&uid1));

        s.select_arranged_pattern(&uid1, true, true);
        assert_eq!(s.selected_arranged_pattern_uids.len(), 2);
        assert!(s.is_arranged_pattern_selected(&uid0));
        assert!(s.is_arranged_pattern_selected(&uid1));

        s.select_arranged_pattern(&uid1, false, true);
        assert_eq!(s.selected_arranged_pattern_uids.len(), 1);
        assert!(s.is_arranged_pattern_selected(&uid0));
        assert!(!s.is_arranged_pattern_selected(&uid1));

        s.select_arranged_pattern(&uid1, true, false);
        assert_eq!(s.selected_arranged_pattern_uids.len(), 1);
        assert!(!s.is_arranged_pattern_selected(&uid0));
        assert!(s.is_arranged_pattern_selected(&uid1));
    }
}
