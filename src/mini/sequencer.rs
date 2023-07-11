// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::UidFactory;
use anyhow::{anyhow, Ok, Result};
use btreemultimap::BTreeMultiMap;
use eframe::{
    egui::{Frame, Response, Sense, Ui},
    emath::{self, RectTransform},
    epaint::{vec2, Color32, Pos2, Rect, Rounding, Shape, Stroke, Vec2},
};
use groove_core::{
    midi::{new_note_off, new_note_on, MidiChannel, MidiMessage},
    time::{MusicalTime, TimeSignature},
    traits::{
        gui::Shows, Configurable, ControlEventsFn, Controls, HandlesMidi, Performs, Serializable,
    },
    Uid,
};
use groove_proc_macros::{Control, IsController, Params, Uid};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ops::Range};

/// Identifies an arrangement of a [MiniPattern].
#[derive(
    Copy, Clone, Debug, Serialize, Deserialize, Default, Eq, PartialEq, Ord, PartialOrd, Hash,
)]
pub struct ArrangedPatternUid(pub Uid);

#[derive(Debug, Serialize, Deserialize)]
struct ArrangedPattern {
    pattern_uid: Uid,
    position: MusicalTime,
    is_selected: bool,
}
impl Shows for ArrangedPattern {
    fn show(&mut self, ui: &mut Ui) {
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

    fn show_in_arrangement(&self, ui: &mut Ui, pattern: &MiniPattern) -> Response {
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
    fn show(&mut self, ui: &mut Ui) {
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

    fn ui_content(&mut self, ui: &mut Ui) -> eframe::egui::Response {
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

    /// This pattern's duration in [MusicalTime].
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
pub enum MiniSequencerAction {
    CreatePattern,
    ArrangePatternAppend(Uid),
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
    // Whether we're performing, in the [Performs] sense.
    is_performing: bool,
}

/// [MiniSequencer] converts a chain of [MiniPattern]s into MIDI notes according
/// to a given [Tempo] and [TimeSignature].
#[derive(Debug, Default, Control, IsController, Params, Uid, Serialize, Deserialize)]
pub struct MiniSequencer {
    uid: Uid,
    midi_channel_out: MidiChannel,

    uid_factory: UidFactory,
    arranged_pattern_uid_factory: UidFactory,

    // All the patterns the sequencer knows about. These are not arranged.
    patterns: HashMap<Uid, MiniPattern>,

    arranged_patterns: HashMap<ArrangedPatternUid, ArrangedPattern>,

    #[serde(skip)]
    e: MiniSequencerEphemerals,
}
impl MiniSequencer {
    /// Creates a new [MiniSequencer].
    #[allow(unused_variables)]
    pub fn new_with(params: &MiniSequencerParams, midi_channel_out: MidiChannel) -> Self {
        Self {
            midi_channel_out,
            ..Default::default()
        }
    }

    fn next_arrangement_position(&self) -> MusicalTime {
        self.e.final_event_time
    }

    fn add_pattern(&mut self, pattern: MiniPattern) -> Uid {
        let uid = self.uid_factory.next();
        self.patterns.insert(uid, pattern);
        uid
    }

    fn arrange_pattern_append(&mut self, uid: &Uid) -> Result<ArrangedPatternUid> {
        self.arrange_pattern(uid, self.next_arrangement_position())
    }

    fn arrange_pattern(&mut self, uid: &Uid, position: MusicalTime) -> Result<ArrangedPatternUid> {
        if self.patterns.get(uid).is_some() {
            let arranged_pattern_uid = ArrangedPatternUid(self.arranged_pattern_uid_factory.next());
            self.arranged_patterns.insert(
                arranged_pattern_uid,
                ArrangedPattern {
                    pattern_uid: *uid,
                    position,
                    is_selected: false,
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

    fn move_pattern(&mut self, uid: &ArrangedPatternUid, position: MusicalTime) -> Result<()> {
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
    pub fn show_arrangement(&self, ui: &mut Ui) -> (Response, Option<MiniSequencerAction>) {
        let mut action = None;
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
                    for (arranged_pattern_uid, arranged_pattern) in self.arranged_patterns.iter() {
                        if let Some(pattern) = self.patterns.get(&arranged_pattern.pattern_uid) {
                            if arranged_pattern.show_in_arrangement(ui, pattern).clicked() {
                                // TODO: handle shift/control
                                action = Some(MiniSequencerAction::ToggleArrangedPatternSelection(
                                    *arranged_pattern_uid,
                                ));
                            }
                        }
                    }
                })
                .response
            })
            .inner,
            action,
        )
    }

    /// Removes all selected arranged patterns.
    pub fn remove_selected_patterns(&mut self) {
        self.arranged_patterns.retain(|_, p| !p.is_selected);
    }

    fn calculate_events(&mut self) -> Result<()> {
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
}
impl Shows for MiniSequencer {
    fn show(&mut self, ui: &mut Ui) {
        if let Some(action) = self.ui_content(ui) {
            match action {
                MiniSequencerAction::CreatePattern => {
                    self.add_pattern(MiniPattern::default());
                }
                MiniSequencerAction::ArrangePatternAppend(uid) => {
                    if let Err(e) = self.arrange_pattern_append(&uid) {
                        eprintln!("while appending arranged pattern: {e}");
                    }
                }
                MiniSequencerAction::ToggleArrangedPatternSelection(uid) => {
                    if let Some(mut pattern) = self.arranged_patterns.get_mut(&uid) {
                        pattern.is_selected = !pattern.is_selected;
                    }
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
        fn new_with(key: MidiNote, start: MusicalTime, duration: MusicalTime) -> Self {
            Self {
                key: key as u8,
                range: start..(start + duration),
                ui_state: Default::default(),
            }
        }
    }

    impl MiniPattern {
        fn new_with(time_signature: TimeSignature, notes: Vec<MiniNote>) -> Self {
            let min_time = notes.iter().map(|n| n.range.start).min();
            let max_time = notes.iter().map(|n| n.range.end).max();
            let duration = if min_time.is_some() && max_time.is_some() {
                max_time.unwrap() - min_time.unwrap()
            } else {
                MusicalTime::TIME_ZERO
            };
            Self {
                time_signature,
                duration,
                notes,
            }
        }
    }

    impl MiniSequencer {
        /// For testing only; adds simple patterns.
        fn populate_pattern(&mut self, pattern_number: usize) -> (Uid, usize, MusicalTime) {
            let pattern = match pattern_number {
                0 => MiniPattern::new_with(
                    Default::default(),
                    vec![
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
                    ],
                ),
                1 => MiniPattern::new_with(
                    Default::default(),
                    vec![
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
                    ],
                ),
                _ => panic!(),
            };
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
    fn test_patterns() {
        let mut s = MiniSequencer::default();
        let (pid0, p0_note_count, p0_duration) = s.populate_pattern(0);
        let (pid1, p1_note_count, p1_duration) = s.populate_pattern(1);
        assert_eq!(s.patterns.len(), 2);

        assert!(s.arrange_pattern_append(&pid0).is_ok());
        assert_eq!(s.arranged_patterns.len(), 1, "arranging pattern works");
        assert_eq!(
            s.e.final_event_time, p0_duration,
            "arranging pattern updates final event time"
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
        // We're playing a little fast and loose here, but at this moment in
        // time it's true that arrange_pattern() adds the next pattern exactly
        // at the end of the previous one.
        assert_eq!(
            s.e.final_event_time,
            p0_duration + p1_duration,
            "arranging second pattern updates final event time"
        );
        assert_eq!(
            s.e.events.len(),
            p0_note_count * 2 + p1_note_count * 2,
            "multiple arranged patterns produces expected number of events"
        );
    }

    #[test]
    fn precise_arrangement() {
        let mut s = MiniSequencer::default();
        let (pid0, _, p0_duration) = s.populate_pattern(0);

        assert!(s
            .arrange_pattern(&pid0, MusicalTime::new_with_units(1234))
            .is_ok());
        assert_eq!(
            s.e.final_event_time,
            p0_duration + 1234,
            "Absolute arrangement properly updates final event time"
        );
    }

    #[test]
    fn rearrangement() {
        let mut s = MiniSequencer::default();
        let (pid0, _, p0_duration) = s.populate_pattern(0);
        let ap_uid0 = s.arrange_pattern_append(&pid0).unwrap();
        assert_eq!(s.e.final_event_time, p0_duration);
        assert!(s
            .move_pattern(&ap_uid0, MusicalTime::new_with_units(10000))
            .is_ok());
        assert_eq!(s.e.final_event_time, p0_duration + 10000);
    }
}
