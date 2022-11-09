use crate::{
    clock::{BeatValue, Clock, PerfectTimeUnit, TimeSignature},
    common::{rrc, rrc_downgrade, weak_new, Rrc, Ww},
    midi::{MidiChannel, MidiMessage, MIDI_CHANNEL_RECEIVE_ALL},
    traits::{HasOverhead, Overhead, SinksMidi, SourcesMidi, Terminates, WatchesClock},
};
use btreemultimap::BTreeMultiMap;
use std::{
    cmp::{self, Ordering},
    collections::HashMap,
    fmt::Debug,
    ops::Bound::{Excluded, Included},
};

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

    fn note_to_value(note: &str) -> u8 {
        // TODO https://en.wikipedia.org/wiki/Scientific_pitch_notation labels,
        // e.g., for General MIDI percussion
        note.parse().unwrap_or_default()
    }
}

// TODO: I got eager with the <T> and then tired when I realized it would affect
// more stuff. Thus there's only an implementation for Note.
impl Pattern<Note> {
    pub(crate) fn from_settings(settings: &crate::settings::PatternSettings) -> Self {
        let mut r = Self {
            note_value: settings.note_value.clone(),
            notes: Vec::new(),
        };
        for note_sequence in settings.notes.iter() {
            let mut note_vec = Vec::new();
            for note in note_sequence.iter() {
                note_vec.push(Note {
                    key: Self::note_to_value(note),
                    velocity: 127,
                    duration: PerfectTimeUnit(1.0),
                });
            }
            r.notes.push(note_vec);
        }
        r
    }
}

#[derive(Debug, Default)]
pub struct Note {
    key: u8,
    velocity: u8,
    duration: PerfectTimeUnit, // expressed as multiple of the containing Pattern's note value.
}

#[derive(Debug)]
pub struct PatternProgrammer {
    beat_sequencer: Rrc<BeatSequencer>,
    time_signature: TimeSignature,
    cursor_beats: PerfectTimeUnit,
}

impl PatternProgrammer {
    const CURSOR_BEGIN: PerfectTimeUnit = PerfectTimeUnit(0.0);

    pub fn new_with(sequencer: Rrc<BeatSequencer>, time_signature: &TimeSignature) -> Self {
        Self {
            beat_sequencer: sequencer,
            time_signature: *time_signature,
            cursor_beats: Self::CURSOR_BEGIN,
        }
    }

    // TODO: pub non-crate for Viewable...
    #[allow(dead_code)]
    pub fn cursor(&self) -> PerfectTimeUnit {
        self.cursor_beats
    }

    pub(crate) fn reset_cursor(&mut self) {
        self.cursor_beats = Self::CURSOR_BEGIN;
    }

    pub(crate) fn insert_pattern_at_cursor(
        &mut self,
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
        let pattern_multiplier =
            self.time_signature.beat_value().divisor() / pattern_note_value.divisor();

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
                self.beat_sequencer.borrow_mut().insert(
                    note_start,
                    channel,
                    MidiMessage::NoteOn {
                        key: note.key.into(),
                        vel: note.velocity.into(),
                    },
                );
                // This makes the everything.yaml playback sound funny, since no
                // note lasts longer than the pattern's note value. I'm going to
                // leave it like this to force myself to implement duration
                // expression correctly, rather than continuing to hardcode 0.49
                // as the duration.
                self.beat_sequencer.borrow_mut().insert(
                    note_start + note.duration * PerfectTimeUnit(pattern_multiplier),
                    channel,
                    MidiMessage::NoteOff {
                        key: note.key.into(),
                        vel: note.velocity.into(),
                    },
                );
            }
        }

        // Round up to full measure and advance cursor
        let rounded_max_pattern_len =
            (max_track_len as f32 * pattern_multiplier / self.time_signature.top as f32).ceil()
                * self.time_signature.top as f32;
        self.cursor_beats = self.cursor_beats + PerfectTimeUnit::from(rounded_max_pattern_len);
    }
}

#[derive(Debug)]
pub struct BeatSequencer {
    pub(crate) me: Ww<Self>,
    overhead: Overhead,
    channels_to_sink_vecs: HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>>,
    next_instant: PerfectTimeUnit,
    events: BTreeMultiMap<PerfectTimeUnit, (MidiChannel, MidiMessage)>,
}

impl Default for BeatSequencer {
    fn default() -> Self {
        Self {
            me: weak_new(),
            overhead: Overhead::default(),
            channels_to_sink_vecs: Default::default(),
            next_instant: Default::default(),
            events: Default::default(),
        }
    }
}

impl BeatSequencer {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn new_wrapped() -> Rrc<Self> {
        let wrapped = rrc(Self::new());
        wrapped.borrow_mut().me = rrc_downgrade(&wrapped);
        wrapped
    }

    pub(crate) fn clear(&mut self) {
        // TODO: should this also disconnect sinks? I don't think so
        self.events.clear();
        self.next_instant = PerfectTimeUnit::default();
    }

    pub(crate) fn insert(
        &mut self,
        when: PerfectTimeUnit,
        channel: MidiChannel,
        message: MidiMessage,
    ) {
        self.events.insert(when, (channel, message));
    }
}

// TODO: what does it mean for a MIDI device to be muted?
impl HasOverhead for BeatSequencer {
    fn overhead(&self) -> &Overhead {
        &self.overhead
    }

    fn overhead_mut(&mut self) -> &mut Overhead {
        &mut self.overhead
    }
}

impl SourcesMidi for BeatSequencer {
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

impl WatchesClock for BeatSequencer {
    fn tick(&mut self, clock: &Clock) {
        self.next_instant = PerfectTimeUnit(clock.next_slice_in_beats());

        if self.overhead.is_enabled() {
            // If the last instant marks a new interval, then we want to include
            // any events scheduled at exactly that time. So the range is
            // inclusive.
            let range = (
                Included(PerfectTimeUnit(clock.beats())),
                Excluded(self.next_instant),
            );
            let events = self.events.range(range);
            for (_when, event) in events {
                dbg!(&range, &event);
                self.issue_midi(clock, &event.0, &event.1);
            }
        }
    }
}

impl Terminates for BeatSequencer {
    fn is_finished(&self) -> bool {
        // TODO: This looks like it could be expensive.
        let mut the_rest = self.events.range((
            Included(self.next_instant),
            Included(PerfectTimeUnit(f32::MAX)),
        ));
        the_rest.next().is_none()
    }
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;

    use super::*;
    use crate::{
        clock::{TimeSignature, WatchedClock},
        common::rrc_clone,
        settings::PatternSettings,
        utils::tests::TestMidiSink,
    };

    #[allow(dead_code)]
    impl Pattern<PerfectTimeUnit> {
        fn value_to_note(value: u8) -> Note {
            Note {
                key: value,
                velocity: 127,
                duration: PerfectTimeUnit(0.25),
            }
        }
    }

    impl BeatSequencer {
        pub fn debug_dump_events(&self) {
            println!("{:?}", self.events);
        }
    }

    #[test]
    fn test_pattern() {
        let time_signature = TimeSignature::new_defaults();
        let sequencer = BeatSequencer::new_wrapped();
        let mut programmer = PatternProgrammer::new_with(rrc_clone(&sequencer), &time_signature);

        // note that this is five notes, but the time signature is 4/4. This
        // means that we should interpret this as TWO measures, the first having
        // four notes, and the second having just one note and three rests.
        let note_pattern = vec![
            "1".to_string(),
            "2".to_string(),
            "3".to_string(),
            "4".to_string(),
            "5".to_string(),
        ];
        let expected_note_count = note_pattern.len();
        let pattern_settings = PatternSettings {
            id: String::from("test-pattern"),
            note_value: Some(BeatValue::Quarter),
            notes: vec![note_pattern],
        };

        let pattern = Pattern::from_settings(&pattern_settings);

        assert_eq!(pattern.notes.len(), 1);
        assert_eq!(pattern.notes[0].len(), expected_note_count);

        // We don't need to call reset_cursor(), but we do just once to make
        // sure it's working.
        assert_eq!(programmer.cursor(), PatternProgrammer::CURSOR_BEGIN);
        programmer.reset_cursor();
        assert_eq!(programmer.cursor(), PatternProgrammer::CURSOR_BEGIN);

        programmer.insert_pattern_at_cursor(&0, &pattern);
        assert_eq!(
            programmer.cursor(),
            PerfectTimeUnit::from(2 * time_signature.top)
        );
        assert_eq!(sequencer.borrow().events.len(), expected_note_count * 2); // one on, one off
    }

    #[test]
    fn test_multi_pattern_track() {
        let time_signature = TimeSignature::new_with(7, 8);
        let sequencer = BeatSequencer::new_wrapped();
        let mut programmer = PatternProgrammer::new_with(rrc_clone(&sequencer), &time_signature);

        // since these patterns are denominated in a quarter notes, but the time
        // signature calls for eighth notes, they last twice as long as they
        // seem.
        //
        // four quarter-notes in 7/8 time = 8 beats = 2 measures
        let mut note_pattern_1 = Vec::new();
        for i in 1..=4 {
            note_pattern_1.push(i.to_string());
        }
        // eight quarter-notes in 7/8 time = 16 beats = 3 measures
        let mut note_pattern_2 = Vec::new();
        for i in 11..=18 {
            note_pattern_2.push(i.to_string());
        }
        let len_1 = note_pattern_1.len();
        let len_2 = note_pattern_2.len();
        let pattern_settings = PatternSettings {
            id: String::from("test-pattern"),
            note_value: Some(BeatValue::Quarter),
            notes: vec![note_pattern_1, note_pattern_2],
        };

        let pattern = Pattern::from_settings(&pattern_settings);

        let expected_note_count = len_1 + len_2;
        assert_eq!(pattern.notes.len(), 2);
        assert_eq!(pattern.notes[0].len(), len_1);
        assert_eq!(pattern.notes[1].len(), len_2);

        programmer.insert_pattern_at_cursor(&0, &pattern);

        // expect max of (2, 3) measures
        assert_eq!(
            programmer.cursor(),
            PerfectTimeUnit::from(3 * time_signature.top)
        );
        assert_eq!(sequencer.borrow().events.len(), expected_note_count * 2); // one on, one off
    }

    #[test]
    fn test_pattern_default_note_value() {
        let time_signature = TimeSignature::new_with(7, 4);
        let sequencer = BeatSequencer::new_wrapped();
        let mut programmer = PatternProgrammer::new_with(sequencer, &time_signature);
        let pattern = Pattern::<Note>::from_settings(&PatternSettings {
            id: String::from("test-pattern-inherit"),
            note_value: None,
            notes: vec![vec![String::from("1")]],
        });
        programmer.insert_pattern_at_cursor(&0, &pattern);

        assert_eq!(
            programmer.cursor(),
            PerfectTimeUnit::from(time_signature.top)
        );
    }

    #[test]
    fn test_random_access() {
        let sequencer = BeatSequencer::new_wrapped();
        let mut programmer =
            PatternProgrammer::new_with(rrc_clone(&sequencer), &TimeSignature::new_defaults());
        let mut pattern = Pattern::<Note>::new();

        const NOTE_VALUE: BeatValue = BeatValue::Quarter;
        pattern.note_value = Some(NOTE_VALUE);
        pattern.notes.push(vec![
            // Normal duration
            Note {
                key: 1,
                velocity: 40,
                duration: PerfectTimeUnit(1.0),
            },
            // A little bit shorter
            Note {
                key: 2,
                velocity: 41,
                duration: PerfectTimeUnit(0.99),
            },
            // A little bit longer
            Note {
                key: 3,
                velocity: 42,
                duration: PerfectTimeUnit(1.01),
            },
            // Zero duration!
            Note {
                key: 4,
                velocity: 43,
                duration: PerfectTimeUnit(0.0),
            },
        ]);

        let midi_recorder = TestMidiSink::new_wrapped();
        let midi_channel = midi_recorder.borrow().midi_channel();
        let sink = rrc_downgrade(&midi_recorder);
        sequencer.borrow_mut().add_midi_sink(midi_channel, sink);

        programmer.insert_pattern_at_cursor(&midi_channel, &pattern);

        // Test recorder has seen nothing to start with.
        assert!(midi_recorder.borrow().messages.is_empty());

        let mut clock = WatchedClock::new();
        let sample_rate = clock.inner_clock().sample_rate();
        let watcher = rrc_clone(&sequencer);
        clock.add_watcher(watcher);

        loop {
            if clock.visit_watchers() {
                break;
            }
            clock.tick();
        }

        // We should have gotten one on and one off for each note in the
        // pattern.
        assert_eq!(
            midi_recorder.borrow().messages.len(),
            pattern.notes[0].len() * 2
        );

        sequencer.borrow().debug_dump_events();

        // The clock should stop at the last note-off, which is 1.01 beats past
        // the start of the third note, which started at 2.0. Since the fourth
        // note is zero-duration, it actually ends at 3.0, before the third
        // note's note-off event happens.
        let last_beat = 3.01;
        assert_approx_eq!(
            clock.inner_clock().beats(),
            last_beat,
            1.5 / sample_rate as f32 // The extra 0.5 is for f32 precision
        );
        assert_eq!(
            clock.inner_clock().samples(),
            clock.inner_clock().settings().beats_to_samples(last_beat)
        );

        // Start test recorder over again.
        midi_recorder.borrow_mut().messages.clear();

        // Rewind clock to start.
        clock.reset();

        // This shouldn't explode.
        assert!(!clock.visit_watchers());

        // Only the first time slice's events should have fired.
        assert_eq!(midi_recorder.borrow().messages.len(), 1);

        // Fast-forward to the end. Nothing else should fire. This is because
        // any tick() should do work for just the slice specified.
        clock.inner_clock_mut().debug_set_seconds(10.0);
        assert!(clock.visit_watchers());
        assert_eq!(midi_recorder.borrow().messages.len(), 1);

        // Start test recorder over again.
        midi_recorder.borrow_mut().messages.clear();

        // Move just past first note.
        clock.inner_clock_mut().debug_set_samples(1);

        // Keep going until just before half of second beat. We should see the
        // first note off (not on!) and the second note on/off.
        while clock.inner_clock().next_slice_in_beats() < 2.0 {
            clock.visit_watchers();
            clock.tick();
        }
        assert_eq!(midi_recorder.borrow().messages.len(), 3);

        // Keep ticking through start of second beat. Should see one more event:
        // #3 on.
        while clock.inner_clock().beats() <= 2.0 {
            clock.visit_watchers();
            clock.tick();
        }
        dbg!(&midi_recorder.borrow().messages);
        assert_eq!(midi_recorder.borrow().messages.len(), 4);
    }
}
