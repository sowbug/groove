// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::Sequencer;
use crate::messages::EntityMessage;
use groove_core::{
    midi::{HandlesMidi, MidiChannel, MidiMessage},
    time::{BeatValue, PerfectTimeUnit, TimeSignature},
    traits::{IsController, Resets, TicksWithMessages},
};
use groove_macros::Uid;
use std::{cmp, fmt::Debug, str::FromStr};
use struct_sync_macros::Synchronization;
use strum::EnumCount;
use strum_macros::{Display, EnumCount as EnumCountMacro, EnumString, FromRepr, IntoStaticStr};

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
pub struct Note {
    pub key: u8,
    pub velocity: u8,
    pub duration: PerfectTimeUnit, // expressed as multiple of the containing Pattern's note value.
}

/// A [Pattern] is a series of [Note] rows that play simultaneously.
/// [PatternManager] uses [Patterns](Pattern) to program a [Sequencer].
#[derive(Clone, Debug, Default)]
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

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Synchronization)]
#[cfg_attr(
    feature = "serialization",
    derive(Serialize, Deserialize),
    serde(rename = "pattern-manager", rename_all = "kebab-case")
)]
pub struct PatternManagerParams {}

impl PatternManagerParams {}

// There is so much paperwork for a vector because this will eventually become a
// substantial part of the GUI experience.
/// [PatternManager] stores all the [Patterns] that make up a song.
#[derive(Clone, Debug, Default, Uid)]
pub struct PatternManager {
    uid: usize,
    params: PatternManagerParams,
    patterns: Vec<Pattern<Note>>,
}
impl IsController<EntityMessage> for PatternManager {}
impl HandlesMidi for PatternManager {}
impl Resets for PatternManager {}
impl TicksWithMessages<EntityMessage> for PatternManager {
    type Message = EntityMessage;

    #[allow(unused_variables)]
    fn tick(&mut self, tick_count: usize) -> (Option<Vec<Self::Message>>, usize) {
        (None, 0)
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

    pub fn params(&self) -> PatternManagerParams {
        self.params
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

    pub fn new_with(time_signature: &TimeSignature) -> Self {
        Self {
            time_signature: *time_signature,
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
    use crate::tests::DEFAULT_SAMPLE_RATE;
    use groove_core::time::{BeatValue, TimeSignature};

    #[test]
    fn test_pattern() {
        let time_signature = TimeSignature::default();
        let mut sequencer = Sequencer::new_with(
            DEFAULT_SAMPLE_RATE,
            crate::controllers::SequencerParams { bpm: 128.0 },
        );
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
    fn test_multi_pattern_track() {
        let time_signature = TimeSignature::new_with(7, 8).expect("failed");
        let mut sequencer = Sequencer::new_with(
            DEFAULT_SAMPLE_RATE,
            crate::controllers::SequencerParams { bpm: 128.0 },
        );
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
