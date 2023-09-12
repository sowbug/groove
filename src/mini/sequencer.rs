// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::{
    piano_roll::{Pattern, PatternUid, PianoRoll},
    selection_set::SelectionSet,
    widgets::timeline,
    DragDropManager, TrackUid, UidFactory,
};
use anyhow::anyhow;
use btreemultimap::BTreeMultiMap;
use derive_builder::Builder;
use eframe::{
    egui::{Response, ScrollArea, Sense, Ui, Widget, WidgetInfo, WidgetType},
    emath::{self, lerp},
    epaint::{pos2, vec2, Color32, Pos2, Rect, Rounding, Stroke, Vec2},
};
use groove_core::{
    midi::{new_note_off, new_note_on, MidiChannel, MidiMessage},
    time::{MusicalTime, TimeSignature},
    traits::{
        gui::{Displays, DisplaysInTimeline},
        Configurable, ControlEventsFn, Controls, HandlesMidi, Serializable,
    },
    IsUid, Uid,
};
use groove_proc_macros::{Control, IsController, Params, Uid};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::Display,
    ops::Range,
    sync::{Arc, RwLock},
};

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

/// A placement of a [Pattern] within an arrangement.
#[derive(Debug, Serialize, Deserialize, Builder)]
pub struct ArrangedPattern {
    /// The identifier of the underlying pattern being arranged.
    pattern_uid: PatternUid,
    /// Where to place the pattern.
    position: MusicalTime,
}
impl ArrangedPattern {
    fn ui_content(&self, ui: &mut Ui, pattern: &Pattern, is_selected: bool) -> Response {
        let steps_horiz = pattern.time_signature().bottom * 4;

        let desired_size = vec2((pattern.duration().total_beats() * 16) as f32, 64.0);
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

        let shapes = pattern.notes().iter().fold(Vec::default(), |mut v, note| {
            v.extend(pattern.make_note_shapes(note, &to_screen, false, false));
            v
        });

        painter.extend(shapes);

        response
    }

    /// The placement of the [ArrangedPattern].
    pub fn position(&self) -> MusicalTime {
        self.position
    }
}

#[derive(Debug)]
pub enum SequencerAction {
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
    // The source of [Pattern]s.
    piano_roll: Arc<RwLock<PianoRoll>>,

    view_range: Range<MusicalTime>,
}

/// [Sequencer] converts a chain of [Pattern]s into MIDI notes according to a
/// given [Tempo] and [TimeSignature]. It is read-only with respect to
/// [Pattern]s; the smallest unit of music it works with is a [Pattern].
#[derive(Debug, Default, Control, IsController, Params, Uid, Serialize, Deserialize, Builder)]
pub struct Sequencer {
    #[builder(default)]
    uid: Uid,
    #[builder(default)]
    midi_channel_out: MidiChannel,

    #[builder(default)]
    time_signature: TimeSignature,

    #[builder(setter(skip))]
    arranged_pattern_uid_factory: UidFactory<ArrangedPatternUid>,

    #[builder(setter(skip))]
    arranged_patterns: HashMap<ArrangedPatternUid, ArrangedPattern>,

    #[builder(setter(skip))]
    arranged_pattern_selection_set: SelectionSet<ArrangedPatternUid>,

    #[serde(skip)]
    #[builder(setter(skip))]
    e: SequencerEphemerals,
}
impl Sequencer {
    #[allow(missing_docs)]
    pub fn set_piano_roll(&mut self, piano_roll: Arc<RwLock<PianoRoll>>) {
        self.e.piano_roll = piano_roll;
    }

    fn next_arrangement_position(&self) -> MusicalTime {
        self.e.arrangement_cursor
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

    fn arrange_pattern_append(&mut self, uid: &PatternUid) -> anyhow::Result<ArrangedPatternUid> {
        if let Ok(apuid) = self.arrange_pattern(
            uid,
            self.next_arrangement_position().bars(&self.time_signature),
        ) {
            if let Some(pattern) = self.e.piano_roll.read().unwrap().get_pattern(uid) {
                self.e.arrangement_cursor += pattern.duration();
            }
            Ok(apuid)
        } else {
            Err(anyhow!("something went wrong"))
        }
    }

    /// Arranges the given [Pattern] at the specified position, in bars.
    pub fn arrange_pattern(
        &mut self,
        uid: &PatternUid,
        position_in_bars: usize,
    ) -> anyhow::Result<ArrangedPatternUid> {
        let position = MusicalTime::new_with_bars(&self.time_signature, position_in_bars);
        if self.e.piano_roll.read().unwrap().get_pattern(uid).is_some() {
            let arranged_pattern_uid = self.arranged_pattern_uid_factory.mint_next();
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
        let action = None;
        ui.allocate_ui(ui.available_size_before_wrap(), |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                // let patterns = &mut self.patterns;
                // if patterns.is_empty() {
                //     ui.label("Add a pattern and start editing it");
                // } else {
                //     patterns.iter_mut().for_each(|(uid, p)| {
                //         if ui.button("Add to track").clicked() {
                //             action = Some(SequencerAction::ArrangePatternAppend(*uid))
                //         }
                //         p.show(ui);
                //     });
                // }
                ui.label("let it rip");
            });
        });
        action
    }

    /// Renders the owning track's arrangement view.
    pub fn ui_arrangement(
        &mut self,
        ui: &mut Ui,
        _track_uid: TrackUid,
    ) -> (Response, Option<SequencerAction>) {
        (
            ui.horizontal_top(|ui| {
                let mut time_pointer = self.e.view_range.start;
                while time_pointer < self.e.view_range.end {
                    let range_size = self.e.view_range.end
                        - self.e.view_range.start
                        - MusicalTime::new_with_units(1);
                    let half_range_size = MusicalTime::new_with_units(range_size.total_units() / 2);
                    let section_end = (time_pointer + half_range_size).min(self.e.view_range.end);

                    ui.add(timeline::empty_space(
                        time_pointer..section_end,
                        self.e.view_range.clone(),
                    ));
                    time_pointer = section_end;
                }
            })
            .response,
            None,
        )
    }

    /// Renders the owning track's arrangement view.
    #[must_use]
    pub fn ui_arrangement_old(
        &mut self,
        ui: &mut Ui,
        _track_uid: TrackUid,
        view_range: &Range<MusicalTime>,
    ) -> (Response, Option<SequencerAction>) {
        let desired_size = ui.available_size();
        let (id, rect) = ui.allocate_space(desired_size);
        let painter = ui.painter_at(rect);

        let response = ui.interact(rect, id, Sense::click_and_drag());

        let start_beat = view_range.start.total_beats();
        let end_beat = view_range.end.total_beats();
        let to_screen = emath::RectTransform::from_to(
            Rect::from_x_y_ranges(start_beat as f32..=end_beat as f32, 0.0..=1.0),
            rect,
        );

        painter.rect_filled(rect, Rounding::default(), Color32::GRAY);

        // This is a near copy of the label code in
        // Orchestrator::ui_arrangement_labels(). TODO refactor
        let start_beat = view_range.start.total_beats();
        let end_beat = view_range.end.total_beats();
        let beat_count = end_beat - start_beat;
        let to_screen_beats = emath::RectTransform::from_to(
            Rect::from_x_y_ranges(
                view_range.start.total_beats() as f32..=view_range.end.total_beats() as f32,
                0.0..=1.0,
            ),
            rect,
        );

        let skip = self.time_signature.top;
        let shapes = Vec::default();
        let mut last_segment = [
            to_screen_beats * pos2(start_beat as f32, 0.0),
            to_screen_beats * pos2(start_beat as f32, 1.0),
        ];
        ui.horizontal(|ui| {
            for (i, beat) in (start_beat..end_beat).enumerate() {
                if i != 0 && i != beat_count - 1 && i % skip != 0 {
                    continue;
                }
                let this_segment = [
                    to_screen_beats * pos2(beat as f32, 0.0),
                    to_screen_beats * pos2(beat as f32, 1.0),
                ];
                let _hover_rect = Rect::from_two_pos(last_segment[0], this_segment[1]);
                // let can_accept = if let Some(source) = dd.source() {
                //     match source {
                //         super::DragDropSource::NewDevice(_) => false,
                //         super::DragDropSource::Pattern(_) => true,
                //     }
                // } else {
                //     false
                // };
                let can_accept = false; // TODO: commented out the block above here
                let r = DragDropManager::drop_target(ui, can_accept, |ui| {
                    ui.add(space())
                    // shapes.push(Shape::LineSegment {
                    //     points: this_segment,
                    //     stroke: if ui.interact(hover_rect, id, Sense::hover()).hovered() {
                    //         ui.style().visuals.widgets.active.bg_stroke
                    //     } else {
                    //         ui.style().visuals.widgets.inactive.bg_stroke
                    //     },
                    // });

                    // if let Some(source) = source {
                    //     match source {
                    //         DragDropSource::NewDevice(key) => {
                    //             eprintln!("nope - I'm a pattern target {:?}", source)
                    //         }
                    //         DragDropSource::Pattern(_) => {
                    //             eprintln!("sure - I'm a pattern target {:?}", source)
                    //         }
                    //     }
                    // }

                    // if ui.interact(hover_rect, id, Sense::hover()).hovered() {
                    //     shapes.push(Shape::Rect(RectShape {
                    //         rect: hover_rect,
                    //         rounding: Rounding::none(),
                    //         fill: Color32::DARK_GRAY,
                    //         stroke: Stroke {
                    //             width: 2.0,
                    //             color: Color32::YELLOW,
                    //         },
                    //     }));
                    // }

                    // super::drag_drop::DragDropTarget::TrackLocation(
                    //     track_uid,
                    //     MusicalTime::new_with_beats(beat),
                    // )

                    // if ui.interact(hover_rect, id, Sense::hover()).hovered() {
                    //     if let Some(source) = source {
                    //         eprintln!("track beat {beat} - {:?}", source);
                    //     }
                    // };
                });

                if DragDropManager::is_dropped(ui, &r.response) {
                    eprintln!("something happened");
                    //                    eprintln!("dropped at track beat {beat}: {:#?}", dd.source());
                    DragDropManager::reset();
                }
                last_segment = this_segment;
            }
        });

        painter.extend(shapes);

        for (_arranged_pattern_uid, arranged_pattern) in self.arranged_patterns.iter() {
            if let Some(pattern) = self
                .e
                .piano_roll
                .read()
                .unwrap()
                .get_pattern(&arranged_pattern.pattern_uid)
            {
                let start = arranged_pattern.position;
                let end = start + pattern.duration();
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

                let shapes = pattern.notes().iter().fold(Vec::default(), |mut v, note| {
                    v.extend(pattern.make_note_shapes(note, &to_screen_ap, false, false));
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

        (response, None)
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
                        if let Some(pattern) = self
                            .e
                            .piano_roll
                            .read()
                            .unwrap()
                            .get_pattern(&arranged_pattern.pattern_uid)
                        {
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
            if let Some(pattern) = self.e.piano_roll.read().unwrap().get_pattern(&uid) {
                for note in pattern.notes() {
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

    fn show_small(&mut self, ui: &mut Ui) -> eframe::egui::Response {
        ui.label("Sequencer")
    }

    fn show_and_handle(&mut self, ui: &mut Ui) -> eframe::egui::Response {
        if let Some(action) = self.ui_content(ui) {
            match action {
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
        ui.label("TODO")
    }

    fn show_medium(&mut self, ui: &mut Ui) -> eframe::egui::Response {
        self.show_and_handle(ui)
    }

    fn show_full(&mut self, ui: &mut Ui) -> eframe::egui::Response {
        self.show_and_handle(ui)
    }
}
impl Displays for Sequencer {
    fn ui(&mut self, ui: &mut Ui) -> eframe::egui::Response {
        ui.allocate_ui(vec2(ui.available_width(), 64.0), |ui| {
            self.arranged_patterns.values().for_each(|ap| {
                if let Some(pattern) = self
                    .e
                    .piano_roll
                    .read()
                    .unwrap()
                    .get_pattern(&ap.pattern_uid)
                {
                    ap.ui_content(ui, pattern, false);
                }
            })
        })
        .response
    }
}
impl DisplaysInTimeline for Sequencer {
    fn set_view_range(&mut self, view_range: &std::ops::Range<groove_core::time::MusicalTime>) {
        self.e.view_range = view_range.clone();
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
                groove_core::traits::EntityEvent::Midi(MidiChannel(0), *event.1),
            );
        }
    }

    fn is_finished(&self) -> bool {
        // both these are exclusive range bounds
        self.e.range.end >= self.e.final_event_time
    }

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
impl Configurable for Sequencer {}
impl Serializable for Sequencer {
    fn after_deser(&mut self) {
        let _ = self.calculate_events();
    }
}

fn space_ui(ui: &mut Ui) -> Response {
    let mut on_it = true;
    let on = &mut on_it;
    let desired_size = ui.spacing().interact_size.y * vec2(2.0, 1.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, *on, ""));

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool(response.id, *on);
        let visuals = ui.style().interact_selectable(&response, *on);
        let rect = rect.expand(visuals.expansion);
        let radius = 0.5 * rect.height();
        ui.painter()
            .rect(rect, radius, visuals.bg_fill, visuals.bg_stroke);
        let circle_x = lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let center = pos2(circle_x, rect.center().y);
        ui.painter()
            .circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);
    }

    response
}

fn space() -> impl Widget {
    space_ui
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Sequencer {
        fn piano_roll(&self) -> &RwLock<PianoRoll> {
            &self.e.piano_roll
        }
    }

    #[test]
    fn basic() {
        let s = Sequencer::default();
        assert!(
            s.arranged_patterns.is_empty(),
            "default sequencer has no arranged patterns"
        );
        assert!(s.e.events.is_empty(), "default sequencer has no events");
    }

    #[test]
    fn sequencer_translates_patterns_to_events() {
        let mut s = Sequencer::default();

        let (pid0, p0_note_count, p0_duration) =
            s.piano_roll().write().unwrap().populate_pattern(0);
        let (pid1, p1_note_count, p1_duration) =
            s.piano_roll().write().unwrap().populate_pattern(1);

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
        let (pid0, _, p0_duration) = s.piano_roll().write().unwrap().populate_pattern(0);
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
    fn shift_pattern() {
        let mut s = SequencerBuilder::default().build().unwrap();
        let (puid, _, _) = s.piano_roll().write().unwrap().populate_pattern(0);
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
        let (puid0, _, _) = s.piano_roll().write().unwrap().populate_pattern(0);

        let uid0 = s.arrange_pattern(&puid0, 0).unwrap();
        assert_eq!(s.arranged_patterns.len(), 1);

        s.remove_arranged_pattern(&uid0);
        assert!(s.arranged_patterns.is_empty());

        let (puid1, _, _) = s.piano_roll().write().unwrap().populate_pattern(1);

        let uid1 = s.arrange_pattern(&puid1, 0).unwrap();
        let uid0 = s.arrange_pattern(&puid0, 1).unwrap();
        assert_eq!(s.arranged_patterns.len(), 2);

        s.arranged_pattern_selection_set.click(&uid1, false);
        s.remove_selected_arranged_patterns();
        assert_eq!(s.arranged_patterns.len(), 1);

        s.arranged_pattern_selection_set.click(&uid0, false);
        s.remove_selected_arranged_patterns();
        assert!(s.arranged_patterns.is_empty());
    }

    #[test]
    fn arranged_pattern_selection_works() {
        let mut s = SequencerBuilder::default().build().unwrap();
        assert!(s.arranged_pattern_selection_set.is_empty());

        let (puid0, _, _) = s.piano_roll().write().unwrap().populate_pattern(0);
        let (puid1, _, _) = s.piano_roll().write().unwrap().populate_pattern(1);

        let uid0 = s.arrange_pattern(&puid0, 0).unwrap();
        let uid1 = s.arrange_pattern(&puid1, 1).unwrap();

        assert!(s.arranged_pattern_selection_set.is_empty());

        s.arranged_pattern_selection_set.click(&uid0, false);
        assert_eq!(s.arranged_pattern_selection_set.len(), 1);
        assert!(s.arranged_pattern_selection_set.contains(&uid0));
        assert!(!s.arranged_pattern_selection_set.contains(&uid1));

        s.arranged_pattern_selection_set.click(&uid1, true);
        assert_eq!(s.arranged_pattern_selection_set.len(), 2);
        assert!(s.arranged_pattern_selection_set.contains(&uid0));
        assert!(s.arranged_pattern_selection_set.contains(&uid1));

        s.arranged_pattern_selection_set.click(&uid1, true);
        assert_eq!(s.arranged_pattern_selection_set.len(), 1);
        assert!(s.arranged_pattern_selection_set.contains(&uid0));
        assert!(!s.arranged_pattern_selection_set.contains(&uid1));

        s.arranged_pattern_selection_set.click(&uid1, false);
        assert_eq!(s.arranged_pattern_selection_set.len(), 1);
        assert!(!s.arranged_pattern_selection_set.contains(&uid0));
        assert!(s.arranged_pattern_selection_set.contains(&uid1));
    }
}
