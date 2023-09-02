// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::{piano_roll::Pattern, rng::Rng, Note};
use btreemultimap::BTreeMultiMap;
use derive_builder::Builder;
use eframe::{
    egui::{Sense, Ui},
    emath::RectTransform,
    epaint::{pos2, vec2, Color32, Rect, Stroke},
};
use groove_core::{
    midi::{MidiChannel, MidiMessage},
    time::{MusicalTime, TimeSignature},
    traits::{
        gui::{Displays, DisplaysInTimeline},
        Configurable, ControlEventsFn, Controls, HandlesMidi, Serializable,
    },
    Uid,
};
use groove_proc_macros::{Control, IsController, Params, Uid};
use serde::{Deserialize, Serialize};
use std::ops::Range;

impl ESSequencerBuilder {
    /// Produces a random sequence of quarter-note notes. For debugging.
    pub fn random(&mut self, range: Range<MusicalTime>) -> &mut Self {
        let mut rng = Rng::default();

        for _ in 0..32 {
            let beat_range = range.start.total_beats() as u64..range.end.total_beats() as u64;
            let note_start = MusicalTime::new_with_beats(rng.0.rand_range(beat_range) as usize);
            self.note(Note {
                key: rng.0.rand_range(16..100) as u8,
                range: note_start..note_start + MusicalTime::DURATION_QUARTER,
            });
        }
        self
    }
}

#[derive(Debug, Default)]
pub struct ESSequencerEphemerals {
    // The sequencer should be performing work for this time slice.
    range: Range<MusicalTime>,
    // The actual events that the sequencer emits.
    events: BTreeMultiMap<MusicalTime, MidiMessage>,
    // The latest end time (exclusive) of all the events.
    final_event_time: MusicalTime,
    // The next place to insert a note.
    cursor: MusicalTime,
    // Whether we're performing, in the [Performs] sense.
    is_performing: bool,

    // DisplaysInTimeline
    view_range: Range<MusicalTime>,
}

/// [ESSequencer] replays [MidiMessage]s according to [MusicalTime].
#[derive(Debug, Default, Control, IsController, Params, Uid, Serialize, Deserialize, Builder)]
pub struct ESSequencer {
    #[builder(default)]
    uid: Uid,
    #[builder(default)]
    midi_channel_out: MidiChannel,

    #[builder(default, setter(each(name = "note", into)))]
    notes: Vec<Note>,

    #[builder(default)]
    time_signature: TimeSignature,

    #[serde(skip)]
    #[builder(setter(skip))]
    e: ESSequencerEphemerals,
}
impl ESSequencer {
    fn cursor(&self) -> MusicalTime {
        self.e.cursor
    }

    /// Returns the duration of the inserted pattern.
    pub fn insert_pattern(
        &mut self,
        pattern: &Pattern,
        position: MusicalTime,
    ) -> anyhow::Result<MusicalTime> {
        pattern.notes().iter().for_each(|note| {
            self.notes.push(Note {
                key: note.key,
                range: (note.range.start + position)..(note.range.end + position),
            });
        });
        Ok(pattern.duration())
    }

    pub fn append_pattern(&mut self, pattern: &Pattern) -> anyhow::Result<()> {
        let position = self.e.cursor;
        let duration = self.insert_pattern(pattern, position)?;
        self.e.cursor += duration;
        self.calculate_events();
        Ok(())
    }

    fn calculate_events(&mut self) {
        self.e.events.clear();
        self.e.final_event_time = MusicalTime::START;
        self.notes.iter().for_each(|note| {
            self.e.events.insert(
                note.range.start,
                MidiMessage::NoteOn {
                    key: note.key.into(),
                    vel: 127.into(),
                },
            );
            self.e.events.insert(
                note.range.end,
                MidiMessage::NoteOff {
                    key: note.key.into(),
                    vel: 0.into(),
                },
            );
            if note.range.end > self.e.final_event_time {
                self.e.final_event_time = note.range.end;
            }
        })
    }
}
impl Displays for ESSequencer {
    fn ui(&mut self, ui: &mut Ui) -> eframe::egui::Response {
        ui.allocate_ui(vec2(ui.available_width(), 64.0), |ui| {
            let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::hover());
            let x_range_f32 = self.e.view_range.start.total_units() as f32
                ..=self.e.view_range.end.total_units() as f32;
            let y_range = 0.0..=127.0;
            let local_space_rect = Rect::from_x_y_ranges(x_range_f32, y_range);
            let to_screen = RectTransform::from_to(local_space_rect, response.rect);
            self.notes.iter().for_each(|note| {
                painter.line_segment(
                    [
                        to_screen * pos2(note.range.start.total_units() as f32, note.key as f32),
                        to_screen * pos2(note.range.end.total_units() as f32, note.key as f32),
                    ],
                    Stroke {
                        width: 1.0,
                        color: Color32::YELLOW,
                    },
                );
            });
            response
        })
        .inner
    }
}
impl DisplaysInTimeline for ESSequencer {
    fn set_view_range(&mut self, view_range: &std::ops::Range<groove_core::time::MusicalTime>) {
        self.e.view_range = view_range.clone();
    }
}
impl HandlesMidi for ESSequencer {}
impl Controls for ESSequencer {
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
impl Configurable for ESSequencer {}
impl Serializable for ESSequencer {
    fn after_deser(&mut self) {
        let _ = self.calculate_events();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mini::PatternBuilder;

    #[test]
    fn basic() {
        let s = ESSequencer::default();
        assert!(s.notes.is_empty(), "default sequencer has no notes");
        assert!(s.e.events.is_empty(), "default sequencer has no events");
    }

    #[test]
    fn adding_notes_translates_to_events() {
        let mut s = ESSequencer::default();
        let pattern = PatternBuilder::default()
            .note(Note {
                key: 1,
                range: MusicalTime::START..MusicalTime::START + MusicalTime::DURATION_QUARTER,
            })
            .build()
            .unwrap();
        assert!(s.append_pattern(&pattern).is_ok());
        assert_eq!(
            s.e.events.len(),
            2,
            "Adding one note should create two events"
        );
    }
}
