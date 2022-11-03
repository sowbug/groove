use crate::{
    clock::{BeatValue, Clock, TimeSignature},
    common::{rrc, rrc_downgrade, weak_new, Rrc, Ww},
    midi::{MidiChannel, MidiMessage, MIDI_CHANNEL_RECEIVE_ALL},
    traits::{SinksMidi, SourcesMidi, Terminates, WatchesClock},
};
use btreemultimap::BTreeMultiMap;
use midly::num::u7;
use sorted_vec::SortedVec;
use std::{
    cmp::{self, Ordering},
    collections::HashMap,
    fmt::Display,
    ops::Bound::{Excluded, Included},
    ops::{Add, Mul},
};

#[derive(Debug, Default)]
pub struct PatternSequencerOld {
    pub(crate) me: Ww<Self>,
    time_signature: TimeSignature,
    cursor_beats: f32, // TODO: this should be a fixed-precision type

    channels_to_sink_vecs: HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>>,
    sequenced_notes: SortedVec<OrderedEvent<f32>>,
}

impl PatternSequencerOld {
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

    pub fn add_pattern(&mut self, pattern: &PatternOld, channel: u8) {
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

    // TODO: if there is an existing note-off message already scheduled for this
    // note that happens after this note-on event, then we should delete that
    // event; otherwise, this note will get released early (and then released
    // again, which does nothing). That's probably not what we want.
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

impl WatchesClock for PatternSequencerOld {
    fn tick(&mut self, clock: &Clock) {
        // TODO: make this random-access by keeping sequenced_notes in place and
        // scanning to find next items to process. We will probably need some
        // way to tell that the caller seeked. Maybe Clock can tell us that!

        while !self.sequenced_notes.is_empty() {
            let note = *(self.sequenced_notes.first().unwrap());

            if clock.beats() >= note.when {
                self.dispatch_note(&note, clock);

                // TODO: this is violating a (future) rule that we can always
                // randomly access anything in the song. It's actually more than
                // that, because it's destroying information that would be
                // needed to add that ability later.
                self.sequenced_notes.remove_index(0);
            } else {
                break;
            }
        }
    }
}

impl Terminates for PatternSequencerOld {
    fn is_finished(&self) -> bool {
        self.sequenced_notes.is_empty()
    }
}

impl SourcesMidi for PatternSequencerOld {
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
pub struct PatternOld {
    pub note_value: Option<BeatValue>,
    pub notes: Vec<Vec<u8>>,
}

impl PatternOld {
    pub(crate) fn from_settings(settings: &crate::settings::PatternSettings) -> Self {
        let mut r = Self {
            note_value: settings.note_value.clone(),
            notes: Vec::new(),
        };
        for note_sequence in settings.notes.iter() {
            let mut note_vec = Vec::new();
            for note in note_sequence.iter() {
                note_vec.push(PatternOld::note_to_value(note));
            }
            r.notes.push(note_vec);
        }
        r
    }

    fn note_to_value(note: &str) -> u8 {
        // TODO https://en.wikipedia.org/wiki/Scientific_pitch_notation labels,
        // e.g., for General MIDI percussion
        note.parse().unwrap_or_default()
    }
}

#[derive(Clone, Debug, Default)]
pub struct PatternNew<T: Default> {
    pub note_value: Option<BeatValue>,
    pub notes: Vec<Vec<T>>,
}

impl<T: Default> PatternNew<T> {
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
impl PatternNew<Note> {
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
                    duration: PerfectTimeUnit(0.25),
                });
            }
            r.notes.push(note_vec);
        }
        r
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct PerfectTimeUnit(f32);

impl Display for PerfectTimeUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
impl From<f32> for PerfectTimeUnit {
    fn from(value: f32) -> Self {
        PerfectTimeUnit(value)
    }
}
impl From<usize> for PerfectTimeUnit {
    fn from(value: usize) -> Self {
        PerfectTimeUnit(value as f32)
    }
}
impl Add for PerfectTimeUnit {
    type Output = PerfectTimeUnit;
    fn add(self, rhs: Self) -> Self::Output {
        PerfectTimeUnit(self.0 + rhs.0)
    }
}

impl Mul for PerfectTimeUnit {
    type Output = PerfectTimeUnit;
    fn mul(self, rhs: Self) -> Self::Output {
        PerfectTimeUnit(self.0 * rhs.0)
    }
}

impl Ord for PerfectTimeUnit {
    fn cmp(&self, other: &Self) -> Ordering {
        if self > other {
            return Ordering::Greater;
        }
        if self < other {
            return Ordering::Less;
        }
        Ordering::Equal
    }
}

impl Eq for PerfectTimeUnit {}

#[derive(Debug, Default)]
pub struct Note {
    key: u8,
    velocity: u8,
    duration: PerfectTimeUnit,
}

#[derive(Debug)]
pub enum Event {
    NoteOn { key: u8, velocity: u8 },
    NoteOff { key: u8 },
}

#[derive(Debug)]
pub struct PatternSequencerNew {
    pub(crate) me: Ww<Self>,

    time_signature: TimeSignature,

    channels_to_sink_vecs: HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>>,
    events: BTreeMultiMap<PerfectTimeUnit, (MidiChannel, Event)>,

    cursor_beats: PerfectTimeUnit,
    last_instant_handled: PerfectTimeUnit,
    last_instant_handled_not_handled: bool,
}
impl Default for PatternSequencerNew {
    fn default() -> Self {
        Self {
            me: weak_new(),
            time_signature: TimeSignature::default(),
            channels_to_sink_vecs: HashMap::default(),
            events: BTreeMultiMap::default(),
            cursor_beats: Self::CURSOR_BEGIN,
            last_instant_handled: PerfectTimeUnit::default(),
            last_instant_handled_not_handled: true,
        }
    }
}
impl PatternSequencerNew {
    const CURSOR_BEGIN: PerfectTimeUnit = PerfectTimeUnit(0.0);

    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with(time_signature: &TimeSignature) -> Self {
        Self {
            time_signature: *time_signature,
            ..Default::default()
        }
    }

    pub fn new_wrapped_with(time_signature: &TimeSignature) -> Rrc<Self> {
        let wrapped = rrc(Self::new_with(time_signature));
        wrapped.borrow_mut().me = rrc_downgrade(&wrapped);
        wrapped
    }

    // TODO: pub non-crate for Viewable...
    pub fn cursor(&self) -> PerfectTimeUnit {
        self.cursor_beats
    }

    pub(crate) fn reset_cursor(&mut self) {
        self.cursor_beats = Self::CURSOR_BEGIN;
    }

    fn clear(&mut self) {
        // TODO: should this also disconnect sinks? I don't think so
        self.events.clear();
        self.last_instant_handled = PerfectTimeUnit::default();
        self.last_instant_handled_not_handled = true;
    }

    pub(crate) fn insert_pattern_at_cursor(
        &mut self,
        channel: &MidiChannel,
        pattern: &PatternNew<Note>,
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
        let note_value_in_beats = self.duration_for_beat(&pattern_note_value);
        let mut max_track_len = 0;
        for track in pattern.notes.iter() {
            max_track_len = cmp::max(max_track_len, track.len());
            for (i, note) in track.iter().enumerate() {
                let i: PerfectTimeUnit = i.into();
                let note_start = self.cursor_beats + i * note_value_in_beats;
                self.events.insert(
                    note_start,
                    (
                        channel,
                        Event::NoteOn {
                            key: note.key,
                            velocity: note.velocity,
                        },
                    ),
                );
                self.events.insert(
                    note_start + note.duration,
                    (channel, Event::NoteOff { key: note.key }),
                );
            }
        }

        // Round up to full measure and advance cursor
        let rounded_max_pattern_len =
            (max_track_len as f32 * pattern_multiplier / self.time_signature.top as f32).ceil()
                * self.time_signature.top as f32;
        self.cursor_beats = self.cursor_beats + PerfectTimeUnit::from(rounded_max_pattern_len);
    }

    fn duration_for_beat(&self, note_value: &BeatValue) -> PerfectTimeUnit {
        PerfectTimeUnit(1.0 / note_value.divisor())
    }

    fn handle_event(&self, clock: &Clock, channel: &MidiChannel, event: &Event) {
        let note = match event {
            Event::NoteOn { key, velocity } => MidiMessage::NoteOn {
                key: u7::from(*key),
                vel: u7::from(*velocity),
            },
            Event::NoteOff { key } => MidiMessage::NoteOff {
                key: u7::from(*key),
                vel: u7::from(0),
            },
        };
        self.issue_midi(clock, channel, &note);
    }
}

impl SourcesMidi for PatternSequencerNew {
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

impl WatchesClock for PatternSequencerNew {
    fn tick(&mut self, clock: &Clock) {
        let current_instant = PerfectTimeUnit(clock.beats());
        if current_instant < self.last_instant_handled {
            // The clock has jumped back. TODO: turn off notes that are
            // currently playing.
            self.last_instant_handled = current_instant;
            self.last_instant_handled_not_handled = true;
        }
        // TODO: if clock has jumped far forward

        // If the last instant marks a new interval, then we want to include any
        // events scheduled at exactly that time. So the range is inclusive.
        let range = if self.last_instant_handled_not_handled {
            self.last_instant_handled_not_handled = false;
            (
                Included(self.last_instant_handled),
                Included(PerfectTimeUnit(clock.beats())),
            )
        } else {
            (
                Excluded(self.last_instant_handled),
                Included(PerfectTimeUnit(clock.beats())),
            )
        };
        let events = self.events.range(range);
        for (_when, event) in events {
            self.handle_event(clock, &event.0, &event.1);
        }
        self.last_instant_handled = PerfectTimeUnit(clock.beats());
    }
}

impl Terminates for PatternSequencerNew {
    fn is_finished(&self) -> bool {
        // TODO: This looks like it could be expensive.
        let mut the_rest = self.events.range((
            Excluded(self.last_instant_handled),
            Included(PerfectTimeUnit(f32::MAX)),
        ));
        the_rest.next().is_none()
    }
}

#[cfg(test)]
mod tests {
    use more_asserts::{assert_gt, assert_lt};

    use super::*;
    use crate::{
        clock::{TimeSignature, WatchedClock},
        common::rrc_clone,
        settings::PatternSettings,
        utils::tests::TestMidiSink,
    };

    impl PatternOld {
        fn value_to_note(value: u8) -> String {
            value.to_string()
        }
    }

    impl PatternNew<PerfectTimeUnit> {
        fn value_to_note(value: u8) -> Note {
            Note {
                key: value,
                velocity: 127,
                duration: PerfectTimeUnit(0.25),
            }
        }
    }

    #[test]
    fn test_pattern() {
        let time_signature = TimeSignature::new_defaults();
        let mut sequencer = PatternSequencerNew::new_with(&time_signature);

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

        let pattern = PatternNew::from_settings(&pattern_settings);

        assert_eq!(pattern.notes.len(), 1);
        assert_eq!(pattern.notes[0].len(), expected_note_count);

        // We don't need to call reset_cursor(), but we do just once to make
        // sure it's working.
        assert_eq!(sequencer.cursor(), PatternSequencerNew::CURSOR_BEGIN);
        sequencer.reset_cursor();
        assert_eq!(sequencer.cursor(), PatternSequencerNew::CURSOR_BEGIN);

        sequencer.insert_pattern_at_cursor(&0, &pattern);
        assert_eq!(
            sequencer.cursor(),
            PerfectTimeUnit::from(2 * time_signature.top)
        );
        assert_eq!(sequencer.events.len(), expected_note_count * 2); // one on, one off
    }

    #[test]
    fn test_multi_pattern_track() {
        let time_signature = TimeSignature::new_with(7, 8);
        let mut sequencer = PatternSequencerNew::new_with(&time_signature);

        // since these patterns are denominated in a quarter notes, but the time
        // signature calls for eighth notes, they last twice as long as they
        // seem.
        //
        // four quarter-notes in 7/8 time = 8 beats = 2 measures
        let note_pattern_1 = vec![
            "1".to_string(),
            "2".to_string(),
            "3".to_string(),
            "4".to_string(),
        ];
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

        let pattern = PatternNew::from_settings(&pattern_settings);

        let expected_note_count = len_1 + len_2;
        assert_eq!(pattern.notes.len(), 2);
        assert_eq!(pattern.notes[0].len(), len_1);
        assert_eq!(pattern.notes[1].len(), len_2);

        sequencer.insert_pattern_at_cursor(&0, &pattern);

        // expect max of (2, 3) measures
        assert_eq!(
            sequencer.cursor(),
            PerfectTimeUnit::from(3 * time_signature.top)
        );
        assert_eq!(sequencer.events.len(), expected_note_count * 2); // one on, one off
    }

    #[test]
    fn test_pattern_default_note_value() {
        let time_signature = TimeSignature::new_with(7, 4);
        let mut sequencer = PatternSequencerNew::new_with(&time_signature);
        let pattern = PatternNew::<Note>::from_settings(&PatternSettings {
            id: String::from("test-pattern-inherit"),
            note_value: None,
            notes: vec![vec![String::from("1")]],
        });
        sequencer.insert_pattern_at_cursor(&0, &pattern);

        assert_eq!(
            sequencer.cursor(),
            PerfectTimeUnit::from(time_signature.top)
        );
    }

    #[test]
    fn test_random_access() {
        let pattern_sequencer = rrc(PatternSequencerNew::new());
        let mut pattern = PatternNew::<Note>::new();

        pattern.note_value = Some(BeatValue::Quarter);
        pattern.notes.push(vec![
            Note {
                key: 1,
                velocity: 40,
                duration: PerfectTimeUnit(0.5),
            },
            Note {
                key: 2,
                velocity: 41,
                duration: PerfectTimeUnit(0.5),
            },
            Note {
                key: 3,
                velocity: 42,
                duration: PerfectTimeUnit(0.5),
            },
            Note {
                key: 4,
                velocity: 43,
                duration: PerfectTimeUnit(0.5),
            },
        ]);

        let midi_recorder = TestMidiSink::new_wrapped();
        let midi_channel = midi_recorder.borrow().midi_channel();
        let sink = rrc_downgrade(&midi_recorder);
        pattern_sequencer
            .borrow_mut()
            .add_midi_sink(midi_channel, sink);

        pattern_sequencer
            .borrow_mut()
            .insert_pattern_at_cursor(&midi_channel, &pattern);

        // Test recorder has seen nothing to start with.
        assert!(midi_recorder.borrow().messages.is_empty());

        let mut clock = WatchedClock::new();
        let watcher = rrc_clone(&pattern_sequencer);
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

        // The clock should stop just after the last note-off.
        assert_gt!(clock.inner_clock().beats(), 1.0);
        assert_lt!(clock.inner_clock().beats(), 1.01);
        assert_eq!(
            clock.inner_clock().samples(),
            ((60.0 * 1.0 / clock.inner_clock().settings().bpm())
                * (clock.inner_clock().settings().sample_rate() as f32)) as usize
                + 1
        );

        // Start test recorder over again.
        midi_recorder.borrow_mut().messages.clear();

        // Rewind clock to start.
        clock.reset();

        // This shouldn't explode.
        assert!(!clock.visit_watchers());

        // Only the first time slice's events should have fired.
        assert_eq!(midi_recorder.borrow().messages.len(), 1);

        // Fast-forward to the end. For now (until we have defined how jumping
        // forward should behave), the proper response is for everything to
        // fire.
        clock.inner_clock_mut().debug_set_seconds(10.0);
        assert!(clock.visit_watchers());
        assert_eq!(
            midi_recorder.borrow().messages.len(),
            pattern.notes[0].len() * 2
        );

        // Start test recorder over again.
        midi_recorder.borrow_mut().messages.clear();

        // Move just past first note.
        clock.inner_clock_mut().debug_set_beats(0.001);

        // Keep going until just before half of first beat.
        while clock.inner_clock().beats() <= 0.49 {
            clock.visit_watchers();
            clock.tick();
        }

        // Now we should see the first note off (not on!), and the second note
        // on.
        assert_eq!(midi_recorder.borrow().messages.len(), 2);

        // Keep ticking until halfway point. Should see two more events: #2 off,
        // #3 on.
        //
        // Note that we have a little fudge factor (.5001) because of f32
        // accuracy. TODO: for the thousandth time, switch over to something
        // more accurate.
        while clock.inner_clock().beats() <= 0.5001 {
            clock.visit_watchers();
            clock.tick();
        }

        assert_eq!(midi_recorder.borrow().messages.len(), 4);
    }
}
