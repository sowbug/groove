use crate::{
    clock::{BeatValue, Clock, TimeSignature},
    common::{rrc, rrc_downgrade, Rrc, Ww},
    midi::{MidiChannel, MidiMessage, MIDI_CHANNEL_RECEIVE_ALL},
    traits::{SinksMidi, SourcesMidi, Terminates, WatchesClock},
};
use midly::num::u7;
use sorted_vec::SortedVec;
use std::{
    cmp::{self, Ordering},
    collections::HashMap,
};

#[derive(Debug, Default)]
pub struct PatternSequencer {
    pub(crate) me: Ww<Self>,
    time_signature: TimeSignature,
    cursor_beats: f32, // TODO: this should be a fixed-precision type

    channels_to_sink_vecs: HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>>,
    sequenced_notes: SortedVec<OrderedEvent<f32>>,
}

impl PatternSequencer {
    const CURSOR_BEGIN: f32 = 0.0;

    pub fn new_with(time_signature: &TimeSignature) -> Self {
        Self {
            time_signature: *time_signature,
            cursor_beats: Self::CURSOR_BEGIN,
            ..Default::default()
        }
    }

    pub fn new_wrapped_with(time_signature: &TimeSignature) -> Rrc<Self> {
        let wrapped = rrc(Self::new_with(time_signature));
        wrapped.borrow_mut().me = rrc_downgrade(&wrapped);
        wrapped
    }

    pub fn add_pattern(&mut self, pattern: &Pattern, channel: u8) {
        let pattern_note_value = if pattern.note_value.is_some() {
            pattern.note_value.as_ref().unwrap().clone()
        } else {
            self.time_signature.beat_value()
        };

        // If the time signature is 4/4 and the pattern is also quarter-notes, then the
        // multiplier is 1.0 because no correction is needed.
        //
        // If it's 4/4 and eighth notes, for example, the multiplier is 0.5, because
        // each pattern note represents only a half-beat.
        let pattern_multiplier =
            self.time_signature.beat_value().divisor() / pattern_note_value.divisor();

        let mut max_pattern_len = 0;
        for note_sequence in pattern.notes.iter() {
            max_pattern_len = cmp::max(max_pattern_len, note_sequence.len());
            for (i, note) in note_sequence.iter().enumerate() {
                self.insert_short_note(
                    channel,
                    *note,
                    self.cursor_beats + i as f32 * pattern_multiplier,
                    0.49, // TODO: hack because we don't have duration
                );
            }
        }

        // Round up to full measure and advance cursor
        let rounded_max_pattern_len =
            ((max_pattern_len as f32 * pattern_multiplier / self.time_signature.top as f32).ceil()
                * self.time_signature.top as f32) as usize;
        self.cursor_beats += rounded_max_pattern_len as f32;
    }

    // TODO: if there is an existing note-off message already scheduled for this note that happens
    // after this note-on event, then we should delete that event; otherwise, this note will get
    // released early (and then released again, which does nothing). That's probably not what we want.
    fn insert_short_note(
        &mut self,
        channel: MidiChannel,
        note: u8,
        when_beats: f32,
        duration_beats: f32,
    ) {
        if note != 0 {
            self.sequenced_notes.insert(OrderedEvent {
                when: when_beats,
                channel,
                event: MidiMessage::NoteOn {
                    key: u7::from(note),
                    vel: 100.into(),
                },
            });
            self.sequenced_notes.insert(OrderedEvent {
                when: when_beats + duration_beats,
                channel,
                event: MidiMessage::NoteOff {
                    key: u7::from(note),
                    vel: 0.into(),
                },
            });
        }
    }

    fn dispatch_note(&mut self, note: &OrderedEvent<f32>, clock: &Clock) {
        self.issue_midi(clock, &note.channel, &note.event);
    }

    pub(crate) fn reset_cursor(&mut self) {
        self.cursor_beats = Self::CURSOR_BEGIN;
    }

    // TODO remove when we have more substantial interaction with GUI
    // #[cfg(test)]
    pub(crate) fn cursor(&self) -> f32 {
        self.cursor_beats
    }
}

impl WatchesClock for PatternSequencer {
    fn tick(&mut self, clock: &Clock) {
        // TODO: make this random-access by keeping sequenced_notes in place and scanning to find
        // next items to process. We will probably need some way to tell that the caller seeked.
        // Maybe Clock can tell us that!

        while !self.sequenced_notes.is_empty() {
            let note = *(self.sequenced_notes.first().unwrap());

            if clock.beats() >= note.when {
                self.dispatch_note(&note, clock);

                // TODO: this is violating a (future) rule that we can always randomly access
                // anything in the song. It's actually more than that, because it's destroying
                // information that would be needed to add that ability later.
                self.sequenced_notes.remove_index(0);
            } else {
                break;
            }
        }
    }
}

impl Terminates for PatternSequencer {
    fn is_finished(&self) -> bool {
        self.sequenced_notes.is_empty()
    }
}

impl SourcesMidi for PatternSequencer {
    fn midi_sinks_mut(&mut self) -> &mut HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>> {
        &mut self.channels_to_sink_vecs
    }

    fn midi_sinks(&self) -> &HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>> {
        &self.channels_to_sink_vecs
    }

    fn midi_output_channel(&self) -> MidiChannel {
        MIDI_CHANNEL_RECEIVE_ALL
    }

    fn set_midi_output_channel(&mut self, _midi_channel: MidiChannel) {}
}

#[derive(Clone, Copy, Debug)]
pub struct OrderedEvent<T: PartialOrd + PartialEq> {
    pub when: T,
    pub channel: MidiChannel,
    pub event: MidiMessage,
}

impl<T: PartialOrd> PartialOrd for OrderedEvent<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.when > other.when {
            return Some(Ordering::Greater);
        }
        if self.when < other.when {
            return Some(Ordering::Less);
        }
        Some(Ordering::Equal)
    }
}

impl<T: PartialOrd> Ord for OrderedEvent<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.when > other.when {
            return Ordering::Greater;
        }
        if self.when < other.when {
            return Ordering::Less;
        }
        Ordering::Equal
    }
}

impl<T: PartialOrd> PartialEq for OrderedEvent<T> {
    fn eq(&self, other: &Self) -> bool {
        self.when == other.when && self.event == other.event
    }
}

impl<T: PartialOrd> Eq for OrderedEvent<T> {}

#[derive(Clone, Debug, Default)]
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
        for note_sequence in settings.notes.iter() {
            let mut note_vec = Vec::new();
            for note in note_sequence.iter() {
                note_vec.push(Pattern::note_to_value(note));
            }
            r.notes.push(note_vec);
        }
        r
    }

    fn note_to_value(note: &str) -> u8 {
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

    use super::*;
    use crate::{clock::TimeSignature, settings::PatternSettings};

    #[test]
    fn test_pattern() {
        let time_signature = TimeSignature::new_defaults();
        let mut sequencer = PatternSequencer::new_with(&time_signature);

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

        let pattern = Pattern::from_settings(&pattern_settings);

        let expected_note_count = note_pattern.len();
        assert_eq!(pattern.notes.len(), 1);
        assert_eq!(pattern.notes[0].len(), expected_note_count);

        // We don't need to call reset_cursor(), but we do just once to make sure it's working.
        assert_eq!(sequencer.cursor(), PatternSequencer::CURSOR_BEGIN);
        sequencer.reset_cursor();
        assert_eq!(sequencer.cursor(), PatternSequencer::CURSOR_BEGIN);

        sequencer.add_pattern(&pattern, 0);
        assert_eq!(sequencer.cursor(), (2 * time_signature.top) as f32);
        assert_eq!(sequencer.sequenced_notes.len(), expected_note_count * 2); // one on, one off
    }

    #[test]
    fn test_multi_pattern_track() {
        let time_signature = TimeSignature::new_with(7, 8);
        let mut sequencer = PatternSequencer::new_with(&time_signature);

        // since these patterns are denominated in a quarter notes, but the time signature
        // calls for eighth notes, they last twice as long as they seem.
        //
        // four quarter-notes in 7/8 time = 8 beats = 2 measures
        let note_pattern_1 = vec![
            Pattern::value_to_note(1),
            Pattern::value_to_note(2),
            Pattern::value_to_note(3),
            Pattern::value_to_note(4),
        ];
        // eight quarter-notes in 7/8 time = 16 beats = 3 measures
        let note_pattern_2 = vec![
            Pattern::value_to_note(11),
            Pattern::value_to_note(12),
            Pattern::value_to_note(13),
            Pattern::value_to_note(14),
            Pattern::value_to_note(15),
            Pattern::value_to_note(16),
            Pattern::value_to_note(17),
            Pattern::value_to_note(18),
        ];
        let pattern_settings = PatternSettings {
            id: String::from("test-pattern"),
            note_value: Some(BeatValue::Quarter),
            notes: vec![note_pattern_1.clone(), note_pattern_2.clone()],
        };

        let pattern = Pattern::from_settings(&pattern_settings);

        let expected_note_count = note_pattern_1.len() + note_pattern_2.len();
        assert_eq!(pattern.notes.len(), 2);
        assert_eq!(pattern.notes[0].len(), note_pattern_1.len());
        assert_eq!(pattern.notes[1].len(), note_pattern_2.len());

        sequencer.add_pattern(&pattern, 0);

        // expect max of (2, 3) measures
        assert_eq!(sequencer.cursor(), (3 * time_signature.top) as f32);
        assert_eq!(sequencer.sequenced_notes.len(), expected_note_count * 2); // one on, one off
    }

    #[test]
    fn test_pattern_default_note_value() {
        let time_signature = TimeSignature::new_with(7, 4);
        let mut sequencer = PatternSequencer::new_with(&time_signature);
        let pattern = Pattern::from_settings(&PatternSettings {
            id: String::from("test-pattern-inherit"),
            note_value: None,
            notes: vec![vec![String::from("1")]],
        });
        sequencer.add_pattern(&pattern, 0);

        assert_eq!(sequencer.cursor(), time_signature.top as f32);
    }
}
