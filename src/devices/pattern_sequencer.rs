use std::{
    cell::RefCell,
    cmp::{self, Ordering},
    rc::Rc,
};

use sorted_vec::SortedVec;

use crate::{
    common::MidiMessage,
    primitives::clock::{BeatValue, Clock, TimeSignature},
};

use super::traits::DeviceTrait;

pub struct PatternSequencer {
    time_signature: TimeSignature,

    sinks: Vec<Rc<RefCell<dyn DeviceTrait>>>,
    sequenced_notes: SortedVec<OrderedNote>,
}

impl PatternSequencer {
    pub fn new(time_signature: &TimeSignature) -> Self {
        let result = Self {
            time_signature: time_signature.clone(),
            sinks: Vec::new(),
            sequenced_notes: SortedVec::new(),
        };
        result
    }

    pub fn insert_pattern(
        &mut self,
        pattern: Rc<RefCell<Pattern>>,
        channel: u8,
        beat_cursor_start: f32, // TODO: this should be a fixed-precision type
    ) -> f32 {
        let pattern_note_value = if pattern.borrow().note_value.is_some() {
            pattern.borrow().note_value.as_ref().unwrap().clone()
        } else {
            self.time_signature.beat_value()
        };
        let note_value_beats =
            self.time_signature.beat_value().divisor() / pattern_note_value.divisor();

        let mut max_pattern_len = 0;
        for note_sequence in pattern.borrow().notes.clone() {
            max_pattern_len = cmp::max(max_pattern_len, note_sequence.len());
            for (i, note) in note_sequence.iter().enumerate() {
                self.insert_short_note(
                    channel,
                    *note,
                    beat_cursor_start + i as f32 * note_value_beats,
                    note_value_beats * 2.0, // TODO: hack because we don't have duration
                );
            }
        }

        // Round up to full measure
        let rounded_max_pattern_len = ((max_pattern_len as f32 / self.time_signature.top as f32)
            .ceil()
            * self.time_signature.top as f32) as usize;
        beat_cursor_start + rounded_max_pattern_len as f32 * (note_value_beats)
    }

    fn insert_short_note(&mut self, channel: u8, note: u8, when_beats: f32, duration_beats: f32) {
        if note != 0 {
            self.sequenced_notes.insert(OrderedNote {
                when_beats,
                message: MidiMessage {
                    status: crate::common::MidiMessageType::NoteOn,
                    channel,
                    data1: note,
                    data2: 100,
                },
            });
            self.sequenced_notes.insert(OrderedNote {
                when_beats: when_beats + duration_beats,
                message: MidiMessage {
                    status: crate::common::MidiMessageType::NoteOff,
                    channel,
                    data1: note,
                    data2: 0,
                },
            });
        }
    }

    fn dispatch_note(&self, note: &OrderedNote, clock: &Clock) {
        for sink in self.sinks.clone() {
            sink.borrow_mut().handle_midi_message(&note.message, clock);
        }
    }
}

impl DeviceTrait for PatternSequencer {
    fn sources_midi(&self) -> bool {
        true
    }

    fn tick(&mut self, clock: &Clock) -> bool {
        // TODO: make this random-access by keeping sequenced_notes in place and scanning to find
        // next items to process. We will probably need some way to tell that the caller seeked.
        // Maybe Clock can tell us that!

        if self.sequenced_notes.is_empty() {
            return true;
        }
        if clock.beats > self.sequenced_notes.last().unwrap().when_beats {
            // This is different from falling through the loop below because
            // it signals that we're done.
            return true;
        }

        while !self.sequenced_notes.is_empty() {
            let note = self.sequenced_notes.first().unwrap();

            if clock.beats >= note.when_beats {
                dbg!(note);
                self.dispatch_note(note, clock);

                // TODO: this is violating a (future) rule that we can always randomly access
                // anything in the song. It's actually more than that, because it's destroying
                // information that would be needed to add that ability later.
                self.sequenced_notes.remove_index(0);
            } else {
                break;
            }
        }
        false
    }

    fn connect_midi_sink(&mut self, device: Rc<RefCell<dyn DeviceTrait>>) {
        self.sinks.push(device);
    }
}

#[derive(PartialEq, PartialOrd, Debug, Clone)]
pub struct OrderedNote {
    pub when_beats: f32,
    pub message: MidiMessage,
}

impl Ord for OrderedNote {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.when_beats > other.when_beats {
            return Ordering::Greater;
        }
        if self.when_beats < other.when_beats {
            return Ordering::Less;
        }
        return Ordering::Equal;
    }
}

impl Eq for OrderedNote {}

#[derive(Clone)]
pub struct Pattern {
    pub note_value: Option<BeatValue>,
    pub notes: Vec<Vec<u8>>,
}

impl Pattern {
    pub(crate) fn from_settings(settings: &crate::settings::PatternSettings) -> Self {
        let mut r = Self {
            note_value: settings.note_value.clone(),
            notes: Vec::new(),
        };
        for note_sequence in settings.notes.clone() {
            let mut note_vec = Vec::new();
            for note in note_sequence.clone() {
                note_vec.push(Pattern::note_to_value(note));
            }
            r.notes.push(note_vec);
        }
        r
    }

    fn note_to_value(note: String) -> u8 {
        // TODO
        // https://en.wikipedia.org/wiki/Scientific_pitch_notation
        // labels, e.g., for General MIDI percussion
        note.parse().unwrap_or_default()
    }

    #[allow(dead_code)]
    fn value_to_note(value: u8) -> String {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use super::*;
    use crate::{primitives::clock::TimeSignature, settings::PatternSettings};

    #[test]
    fn test_pattern() {
        let time_signature = TimeSignature::new_defaults();
        let mut sequencer = PatternSequencer::new(&time_signature);

        // note that this is five notes, but the time signature is 4/4. This means
        // that we should interpret this as TWO measures, the first having four notes, and
        // the second having just one note and three rests.
        let note_pattern = vec![
            Pattern::value_to_note(1),
            Pattern::value_to_note(2),
            Pattern::value_to_note(3),
            Pattern::value_to_note(4),
            Pattern::value_to_note(5),
        ];
        let pattern_settings = PatternSettings {
            id: String::from("test-pattern"),
            note_value: Some(BeatValue::Quarter),
            notes: vec![note_pattern.clone()],
        };

        // TODO: is there any way to avoid Rc/RefCell leaking into this class's API boundary?
        let pattern = Rc::new(RefCell::new(Pattern::from_settings(&pattern_settings)));

        let expected_note_count = note_pattern.len();
        assert_eq!(pattern.borrow().notes.len(), 1);
        assert_eq!(pattern.borrow().notes[0].len(), expected_note_count);

        let mut beat_cursor = 0f32;
        beat_cursor = sequencer.insert_pattern(pattern, 0, beat_cursor);
        assert_eq!(beat_cursor, (2 * time_signature.top) as f32);
        assert_eq!(sequencer.sequenced_notes.len(), expected_note_count * 2); // one on, one off
    }

    #[test]
    fn test_multi_pattern_track() {
        let time_signature = TimeSignature::new_defaults();
        let mut sequencer = PatternSequencer::new(&time_signature);

        let note_pattern_1 = vec![
            Pattern::value_to_note(1),
            Pattern::value_to_note(2),
            Pattern::value_to_note(3),
            Pattern::value_to_note(4),
        ];
        let note_pattern_2 = vec![
            Pattern::value_to_note(11),
            Pattern::value_to_note(12),
            Pattern::value_to_note(13),
            Pattern::value_to_note(14),
        ];
        let pattern_settings = PatternSettings {
            id: String::from("test-pattern"),
            note_value: Some(BeatValue::Quarter),
            notes: vec![note_pattern_1.clone(), note_pattern_2.clone()],
        };

        // TODO: is there any way to avoid Rc/RefCell leaking into this class's API boundary?
        let pattern = Rc::new(RefCell::new(Pattern::from_settings(&pattern_settings)));

        let expected_note_count = note_pattern_1.len() + note_pattern_2.len();
        assert_eq!(pattern.borrow().notes.len(), 2);
        assert_eq!(pattern.borrow().notes[0].len(), note_pattern_1.len());
        assert_eq!(pattern.borrow().notes[1].len(), note_pattern_2.len());

        let mut beat_cursor = 0f32;
        beat_cursor = sequencer.insert_pattern(pattern, 0, beat_cursor);
        assert_eq!(beat_cursor, (1 * time_signature.top) as f32);
        assert_eq!(sequencer.sequenced_notes.len(), expected_note_count * 2); // one on, one off
    }

    #[test]
    fn test_pattern_default_note_value() {
        let time_signature = TimeSignature::new(7, 4);
        let mut sequencer = PatternSequencer::new(&time_signature);
        let pattern = Rc::new(RefCell::new(Pattern::from_settings(&PatternSettings {
            id: String::from("test-pattern-inherit"),
            note_value: None,
            notes: vec![vec![String::from("1")]],
        })));
        let mut beat_cursor: f32 = 0f32;
        beat_cursor = sequencer.insert_pattern(pattern, 0, beat_cursor);

        assert_eq!(beat_cursor as usize, time_signature.top as usize);
    }
}
