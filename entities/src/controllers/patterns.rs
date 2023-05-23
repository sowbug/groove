// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::Sequencer;
use crate::messages::EntityMessage;
//use btreemultimap::BTreeMultiMap;
use groove_core::{
    midi::{HandlesMidi, MidiChannel, MidiMessage},
    time::{BeatValue, PerfectTimeUnit, TimeSignature, TimeSignatureParams},
    traits::{IsController, Performs, Resets, TicksWithMessages},
};
use groove_proc_macros::{Control, Params, Uid};
use std::{cmp, fmt::Debug, ops::Range};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

/// [PatternMessage] specifies interactions that can happen between
/// [PatternManager] and other components such as an application GUI.
#[derive(Clone, Debug)]
pub enum PatternMessage {
    SomethingHappened,
    ButtonPressed,
}

/// A [Note] represents a key-down and key-up event pair that lasts for a
/// specified duration.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Note {
    pub key: u8,
    pub velocity: u8,
    pub duration: PerfectTimeUnit, // expressed as multiple of the containing Pattern's note value.
}

/// A [Pattern] is a series of [Note] rows that play simultaneously.
/// [PatternManager] uses [Patterns](Pattern) to program a [Sequencer].
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Pattern<T: Default> {
    pub note_value: Option<BeatValue>,
    pub notes: Vec<Vec<T>>,
}

impl<T: Default> Pattern<T> {
    #[allow(dead_code)]
    pub(crate) fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn note_to_value(note: &str) -> u8 {
        // TODO https://en.wikipedia.org/wiki/Scientific_pitch_notation labels,
        // e.g., for General MIDI percussion
        note.parse().unwrap_or_default()
    }
}

// There is so much paperwork for a vector because this will eventually become a
// substantial part of the GUI experience.
/// [PatternManager] stores all the [Patterns] that make up a song.
#[derive(Clone, Debug, Default, Control, Params, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct PatternManager {
    uid: usize,
    patterns: Vec<Pattern<Note>>,
    selected_pattern: usize,
}
impl IsController for PatternManager {}
impl HandlesMidi for PatternManager {}
impl Resets for PatternManager {}
impl TicksWithMessages for PatternManager {
    type Message = EntityMessage;

    #[allow(unused_variables)]
    fn tick(&mut self, tick_count: usize) -> (Option<Vec<Self::Message>>, usize) {
        (None, 0)
    }
}
impl Performs for PatternManager {
    fn play(&mut self) {}
    fn stop(&mut self) {}
    fn skip_to_start(&mut self) {}
    fn set_loop(&mut self, _range: &Range<PerfectTimeUnit>) {}
    fn clear_loop(&mut self) {}
    fn set_loop_enabled(&mut self, _is_enabled: bool) {}
    fn is_performing(&self) -> bool {
        false
    }
}
impl PatternManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, pattern: Pattern<Note>) {
        self.patterns.push(pattern);
    }

    pub fn patterns(&self) -> &[Pattern<Note>] {
        &self.patterns
    }

    #[cfg(feature = "iced-framework")]
    #[allow(unreachable_patterns)]
    pub fn update(&mut self, message: PatternManagerMessage) {
        match message {
            PatternManagerMessage::PatternManager(_s) => *self = Self::new(),
            _ => self.derived_update(message),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct NewNote {
    key: u8,
    velocity: u8,
    //    duration: PerfectTimeUnit,
    range: Range<f32>,
}

//pub type NewPatternEventsMap = BTreeMultiMap<PerfectTimeUnit, NewNote>;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct NewPattern {
    //    notes: NewPatternEventsMap,
    notes: Vec<NewNote>,

    #[cfg(feature = "egui-framework")]
    #[cfg_attr(feature = "serialization", serde(skip))]
    dragged_note: Option<NewNote>,
}
impl NewPattern {
    pub fn add(&mut self, note: NewNote, _when: PerfectTimeUnit) {
        //        self.notes.insert(when, note);
        self.notes.push(note);
    }

    pub fn remove(&mut self, note: NewNote, _when: PerfectTimeUnit) {
        // if let Some(v) = self.notes.get_vec_mut(when) {
        //     if v.contains(&note) {
        //         v.retain(|x| *x != note);
        //     }
        // }
        self.notes.retain(|v| *v != note);
    }

    pub fn clear(&mut self) {
        self.notes.clear();
    }
}
impl Default for NewPattern {
    fn default() -> Self {
        Self {
            notes: vec![
                NewNote {
                    key: 1,
                    velocity: 126,
                    range: Range {
                        start: 0.0,
                        end: 1.0,
                    },
                },
                NewNote {
                    key: 80,
                    velocity: 127,
                    range: Range {
                        start: 3.0,
                        end: 4.0,
                    },
                },
            ],
            dragged_note: Default::default(),
        }
    }
}

#[cfg(feature = "egui-framework")]
mod gui {
    use super::{NewPattern, Note, Pattern, PatternManager};
    use crate::controllers::patterns::NewNote;
    use eframe::{
        egui::{Frame, Grid, ScrollArea, Sense},
        emath::{self, RectTransform},
        epaint::{Color32, Pos2, Rect, Rounding, Shape, Stroke, Vec2},
    };
    use groove_core::{
        time::{BeatValue, PerfectTimeUnit},
        traits::gui::Shows,
    };
    use std::ops::Range;

    impl Pattern<Note> {
        pub const CELL_WIDTH: f32 = 32.0;
        pub const CELL_HEIGHT: f32 = 24.0;
    }

    impl Shows for Pattern<Note> {
        fn show(&mut self, ui: &mut eframe::egui::Ui) {
            if let Some(v) = self.note_value.as_mut() {
                v.show(ui);
            } else {
                // We want to inherit the beat value from orchestrator, but we
                // don't have it! TODO
                //
                // TODO again: actually, what does it mean for a pattern to
                // inherit a beat value? The pattern isn't going to change
                // automatically if the time signature changes. I don't think
                // this makes sense to be optional.
                BeatValue::show_inherited(ui);
            }
            Grid::new(ui.next_auto_id()).show(ui, |ui| {
                for notes in self.notes.iter_mut() {
                    for note in notes.iter_mut() {
                        Frame::none()
                            .stroke(Stroke::new(2.0, Color32::GRAY))
                            .fill(Color32::DARK_GRAY)
                            .show(ui, |ui| {
                                let mut text = format!("{}", note.key);
                                if ui.text_edit_singleline(&mut text).changed() {
                                    if let Ok(key) = text.parse() {
                                        note.key = key;
                                    }
                                };
                            });
                    }
                }
            });
        }
    }

    impl Shows for PatternManager {
        fn show(&mut self, ui: &mut eframe::egui::Ui) {
            ui.set_min_width(16.0 * Pattern::CELL_WIDTH + 8.0); //  8 pixels margin
            ScrollArea::vertical().show(ui, |ui| {
                let mut is_first = true;
                for pattern in self.patterns.iter_mut() {
                    if is_first {
                        is_first = false;
                    } else {
                        ui.separator();
                    }
                    pattern.show(ui);
                }
            });
        }
    }

    impl NewPattern {
        pub fn show(&mut self, ui: &mut eframe::egui::Ui) {
            Frame::canvas(ui.style()).show(ui, |ui| {
                self.ui_content(ui);
            });
        }

        fn make_note_shapes(
            &self,
            note: &NewNote,
            to_screen: &RectTransform,
            is_highlighted: bool,
        ) -> Vec<Shape> {
            let rect = to_screen
                .transform_rect(self.rect_for_note(note))
                .shrink(1.0);
            let color = if is_highlighted {
                Color32::WHITE
            } else {
                Color32::DARK_BLUE
            };
            vec![
                Shape::rect_stroke(rect, Rounding::default(), Stroke { width: 2.0, color }),
                Shape::rect_filled(rect.shrink(2.0), Rounding::default(), Color32::LIGHT_BLUE),
            ]
        }

        fn rect_for_note(&self, note: &NewNote) -> Rect {
            let notes_vert = 24.0;
            let steps_horiz = 16.0;
            let note_rect_size = Vec2 {
                x: 1.0 / steps_horiz,
                y: 1.0 / notes_vert,
            };
            let ul = Pos2 {
                x: note.range.start / 4.0,
                y: (note.key as f32) / notes_vert,
            };
            Rect::from_min_size(ul, note_rect_size)
        }

        fn ui_content(&mut self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
            let notes_vert = 24.0;
            let steps_horiz = 16.0;

            let desired_size = ui.available_size_before_wrap();
            let desired_size = Vec2::new(desired_size.x, 256.0);
            let (mut response, painter) =
                ui.allocate_painter(desired_size, Sense::click_and_drag());

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
                    let note_rect = to_screen.transform_rect(self.rect_for_note(note));
                    if note_rect.contains(hover_pos) {
                        hovered_note = Some(note.clone());
                        break;
                    }
                }
            }

            if response.clicked() {
                if let Some(pointer_pos) = response.interact_pointer_pos() {
                    let note =
                        self.note_for_position(&from_screen, steps_horiz, notes_vert, pointer_pos);

                    if let Some(hovered) = &hovered_note {
                        self.remove(hovered.clone(), PerfectTimeUnit::default());
                    } else {
                        self.add(note, PerfectTimeUnit::default());
                    }
                    response.mark_changed();
                }
            }

            if response.drag_started() {
                if hovered_note.is_some() {
                    self.dragged_note = hovered_note.take();
                    if let Some(n) = &self.dragged_note {
                        self.remove(n.clone(), PerfectTimeUnit::default());
                    }
                } else {
                    self.dragged_note = None;
                }
            }
            if response.dragged() {
                if self.dragged_note.is_some() {
                    if let Some(pointer_pos) = response.interact_pointer_pos() {
                        let new_note = self.note_for_position(
                            &from_screen,
                            steps_horiz,
                            notes_vert,
                            pointer_pos,
                        );
                        painter.extend(self.make_note_shapes(&new_note, &to_screen, true));
                    }
                }
            }
            if response.drag_released() {
                if self.dragged_note.is_some() {
                    if let Some(pointer_pos) = response.interact_pointer_pos() {
                        let new_note = self.note_for_position(
                            &from_screen,
                            steps_horiz,
                            notes_vert,
                            pointer_pos,
                        );
                        self.add(new_note, PerfectTimeUnit::default());
                    }
                }
                self.dragged_note = None;
            }

            // if response.drag_started() {
            //     self.is_dragging = true;

            //     if let Some(pointer_pos) = response.interact_pointer_pos() {
            //         self.drag_start_point = Some(pointer_pos);
            //         self.drag_end_point = Some(pointer_pos);
            //     }
            //     // let note =
            //     //     self.note_for_position(&from_screen, steps_horiz, notes_vert, pointer_pos);
            //     // self.is_drag_deleting = self.notes.contains(&note);
            // }
            // if response.drag_released() {
            //     self.is_dragging = false;

            //     if let Some(pointer_pos) = response.interact_pointer_pos() {
            //         self.drag_end_point = Some(pointer_pos);
            //         let note = self.note_for_position(
            //             &from_screen,
            //             steps_horiz,
            //             notes_vert,
            //             self.drag_start_point,
            //             Some(pointer_pos),
            //         );
            //         self.drag_start_point = None;
            //     }
            // }

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
        ) -> NewNote {
            let canvas_pos = from_screen * pointer_pos;
            let key = (canvas_pos.y * notes_vert) as u8;
            let when = (canvas_pos.x * steps_horiz).floor() / 4.0;

            NewNote {
                key,
                velocity: 127,
                range: Range {
                    start: when,
                    end: when + 0.25,
                },
            }
        }
    }
}

/// [PatternProgrammer] knows how to insert a given [Pattern] into a given
/// [Sequencer], respecting the [groove_core::time::TimeSignature] that it was
/// given at creation.
#[derive(Debug)]
pub struct PatternProgrammer {
    time_signature: TimeSignature,
    cursor_beats: PerfectTimeUnit,
}
impl PatternProgrammer {
    const CURSOR_BEGIN: PerfectTimeUnit = PerfectTimeUnit(0.0);

    pub fn new_with(time_signature: &TimeSignatureParams) -> Self {
        Self {
            time_signature: TimeSignature {
                top: time_signature.top,
                bottom: time_signature.bottom,
            },
            cursor_beats: Self::CURSOR_BEGIN,
        }
    }

    // TODO: pub non-crate for Viewable...
    #[allow(dead_code)]
    pub fn cursor(&self) -> PerfectTimeUnit {
        self.cursor_beats
    }

    pub fn reset_cursor(&mut self) {
        self.cursor_beats = Self::CURSOR_BEGIN;
    }

    pub fn insert_pattern_at_cursor(
        &mut self,
        sequencer: &mut Sequencer,
        channel: &MidiChannel,
        pattern: &Pattern<Note>,
    ) {
        let pattern_note_value = if pattern.note_value.is_some() {
            pattern.note_value.as_ref().unwrap().clone()
        } else {
            self.time_signature.beat_value()
        };

        // If the time signature is 4/4 and the pattern is also quarter-notes,
        // then the multiplier is 1.0 because no correction is needed.
        //
        // If it's 4/4 and eighth notes, for example, the multiplier is 0.5,
        // because each pattern note represents only a half-beat.
        let pattern_multiplier = BeatValue::divisor(self.time_signature.beat_value())
            / BeatValue::divisor(pattern_note_value);

        let channel = *channel;
        let mut max_track_len = 0;
        for track in pattern.notes.iter() {
            max_track_len = cmp::max(max_track_len, track.len());
            for (i, note) in track.iter().enumerate() {
                if note.key == 0 {
                    // This is an empty slot in the pattern. Don't do anything.
                    continue;
                }
                let i: PerfectTimeUnit = i.into();
                let note_start = self.cursor_beats + i * PerfectTimeUnit(pattern_multiplier);
                sequencer.insert(
                    note_start,
                    channel,
                    MidiMessage::NoteOn {
                        key: note.key.into(),
                        vel: note.velocity.into(),
                    },
                );
                // This makes the dev-loop.yaml playback sound funny, since no
                // note lasts longer than the pattern's note value. I'm going to
                // leave it like this to force myself to implement duration
                // expression correctly, rather than continuing to hardcode 0.49
                // as the duration.
                sequencer.insert(
                    note_start + note.duration * PerfectTimeUnit(pattern_multiplier),
                    channel,
                    MidiMessage::NoteOff {
                        key: note.key.into(),
                        vel: note.velocity.into(),
                    },
                );
            }
        }

        // Round up to full measure, advance cursor, and make sure sequencer
        // knows we have filled this space.
        let top = self.time_signature.top as f64;
        let rounded_max_pattern_len =
            (max_track_len as f64 * pattern_multiplier / top).ceil() * top;
        self.cursor_beats = self.cursor_beats + PerfectTimeUnit(rounded_max_pattern_len);
        sequencer.set_min_end_time(self.cursor_beats);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controllers::SequencerParams;
    use groove_core::time::BeatValue;

    #[test]
    fn pattern_mainline() {
        let time_signature = TimeSignatureParams { top: 4, bottom: 4 };
        let mut sequencer = Sequencer::new_with(&SequencerParams { bpm: 128.0 });
        let mut programmer = PatternProgrammer::new_with(&time_signature);

        // note that this is five notes, but the time signature is 4/4. This
        // means that we should interpret this as TWO measures, the first having
        // four notes, and the second having just one note and three rests.
        let note_pattern = vec![
            Note {
                key: 1,
                velocity: 127,
                duration: PerfectTimeUnit(1.0),
            },
            Note {
                key: 2,
                velocity: 127,
                duration: PerfectTimeUnit(1.0),
            },
            Note {
                key: 3,
                velocity: 127,
                duration: PerfectTimeUnit(1.0),
            },
            Note {
                key: 4,
                velocity: 127,
                duration: PerfectTimeUnit(1.0),
            },
            Note {
                key: 5,
                velocity: 127,
                duration: PerfectTimeUnit(1.0),
            },
        ];
        let expected_note_count = note_pattern.len();
        let pattern = Pattern::<Note> {
            note_value: Some(BeatValue::Quarter),
            notes: vec![note_pattern],
        };
        assert_eq!(pattern.notes.len(), 1);
        assert_eq!(pattern.notes[0].len(), expected_note_count);

        // We don't need to call reset_cursor(), but we do just once to make
        // sure it's working.
        assert_eq!(programmer.cursor(), PatternProgrammer::CURSOR_BEGIN);
        programmer.reset_cursor();
        assert_eq!(programmer.cursor(), PatternProgrammer::CURSOR_BEGIN);

        programmer.insert_pattern_at_cursor(&mut sequencer, &0, &pattern);
        assert_eq!(
            programmer.cursor(),
            PerfectTimeUnit::from(2 * time_signature.top)
        );
        assert_eq!(sequencer.debug_events().len(), expected_note_count * 2); // one on, one off
    }

    #[test]
    fn multi_pattern_track() {
        let time_signature = TimeSignatureParams { top: 7, bottom: 8 };
        let mut sequencer = Sequencer::new_with(&SequencerParams { bpm: 128.0 });
        let mut programmer = PatternProgrammer::new_with(&time_signature);

        // since these patterns are denominated in a quarter notes, but the time
        // signature calls for eighth notes, they last twice as long as they
        // seem.
        //
        // four quarter-notes in 7/8 time = 8 beats = 2 measures
        let mut note_pattern_1 = Vec::new();
        for i in 1..=4 {
            note_pattern_1.push(Note {
                key: i,
                velocity: 127,
                duration: PerfectTimeUnit(1.0),
            });
        }
        // eight quarter-notes in 7/8 time = 16 beats = 3 measures
        let mut note_pattern_2 = Vec::new();
        for i in 11..=18 {
            note_pattern_2.push(Note {
                key: i,
                velocity: 127,
                duration: PerfectTimeUnit(1.0),
            });
        }
        let len_1 = note_pattern_1.len();
        let len_2 = note_pattern_2.len();
        let pattern = Pattern {
            note_value: Some(BeatValue::Quarter),
            notes: vec![note_pattern_1, note_pattern_2],
        };

        let expected_note_count = len_1 + len_2;
        assert_eq!(pattern.notes.len(), 2);
        assert_eq!(pattern.notes[0].len(), len_1);
        assert_eq!(pattern.notes[1].len(), len_2);

        programmer.insert_pattern_at_cursor(&mut sequencer, &0, &pattern);

        // expect max of (2, 3) measures
        assert_eq!(
            programmer.cursor(),
            PerfectTimeUnit::from(3 * time_signature.top)
        );
        assert_eq!(sequencer.debug_events().len(), expected_note_count * 2); // one on, one off
    }
}
