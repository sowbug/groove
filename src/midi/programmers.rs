use super::{
    patterns::{Note, Pattern},
    MidiChannel,
};
use crate::{
    clock::{BeatValue, MidiTicks, PerfectTimeUnit},
    controllers::sequencers::{BeatSequencer, MidiTickSequencer},
    messages::EntityMessage,
    messages::MessageBounds,
    TimeSignature,
};
use midly::{MidiMessage, TrackEventKind};
use std::{cmp, marker::PhantomData};

pub struct MidiSmfReader {}

impl MidiSmfReader {
    pub fn program_sequencer(sequencer: &mut MidiTickSequencer<EntityMessage>, data: &[u8]) {
        let parse_result = midly::Smf::parse(data).unwrap();

        struct MetaInfo {
            // Pulses per quarter-note
            ppq: u32,

            // Microseconds per quarter-note
            tempo: u32,

            time_signature_numerator: u8,
            time_signature_denominator_exp: u8,
        }
        let mut meta_info = MetaInfo {
            ppq: match parse_result.header.timing {
                midly::Timing::Metrical(ticks_per_beat) => ticks_per_beat.as_int() as u32,
                _ => 0,
            },
            tempo: 0,

            // https://en.wikipedia.org/wiki/Time_signature
            time_signature_numerator: 0,
            time_signature_denominator_exp: 0,
        };
        for (track_number, track) in parse_result.tracks.iter().enumerate() {
            println!("Processing track {track_number}");
            let mut track_time_ticks: usize = 0; // The relative time references start over at zero with each track.

            for t in track.iter() {
                match t.kind {
                    TrackEventKind::Midi { channel, message } => {
                        let delta = t.delta.as_int() as usize;
                        track_time_ticks += delta;
                        sequencer.insert(MidiTicks(track_time_ticks), channel.into(), message);
                        // TODO: prior version of this code treated vel=0 as
                        // note-off. Do we need to handle that higher up?
                    }

                    TrackEventKind::Meta(meta_message) => match meta_message {
                        midly::MetaMessage::TimeSignature(numerator, denominator_exp, _cc, _bb) => {
                            meta_info.time_signature_numerator = numerator;
                            meta_info.time_signature_denominator_exp = denominator_exp;
                            //meta_info.ppq = cc; WHA???
                        }
                        midly::MetaMessage::Tempo(tempo) => {
                            meta_info.tempo = tempo.as_int();
                        }
                        midly::MetaMessage::TrackNumber(track_opt) => {
                            if track_opt.is_none() {
                                continue;
                            }
                        }
                        midly::MetaMessage::EndOfTrack => {
                            let _time_signature: (u32, u32) = (
                                meta_info.time_signature_numerator.into(),
                                2_u32.pow(meta_info.time_signature_denominator_exp.into()),
                            );
                            let ticks_per_quarter_note: f32 = meta_info.ppq as f32;
                            let seconds_per_quarter_note: f32 = meta_info.tempo as f32 / 1000000.0;
                            let _ticks_per_second =
                                ticks_per_quarter_note / seconds_per_quarter_note;

                            let _bpm: f32 = (60.0 * 1000000.0) / (meta_info.tempo as f32);

                            // sequencer.set_midi_ticks_per_second(ticks_per_second
                            // as usize);
                        }
                        _ => {}
                    },
                    TrackEventKind::SysEx(_data) => { // TODO
                    }
                    TrackEventKind::Escape(_data) => { // TODO
                    }
                }
            }
        }
        println!("Done processing MIDI file");
    }
}

#[derive(Debug)]
pub struct PatternProgrammer<M: MessageBounds> {
    time_signature: TimeSignature,
    cursor_beats: PerfectTimeUnit,

    _phantom: PhantomData<M>,
}

impl<M: MessageBounds> PatternProgrammer<M> {
    const CURSOR_BEGIN: PerfectTimeUnit = PerfectTimeUnit(0.0);

    pub fn new_with(time_signature: &TimeSignature) -> Self {
        Self {
            time_signature: *time_signature,
            cursor_beats: Self::CURSOR_BEGIN,
            _phantom: PhantomData::default(),
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
        sequencer: &mut BeatSequencer<M>,
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
        let rounded_max_pattern_len =
            (max_track_len as f32 * pattern_multiplier / self.time_signature.top as f32).ceil()
                * self.time_signature.top as f32;
        self.cursor_beats = self.cursor_beats + PerfectTimeUnit::from(rounded_max_pattern_len);
        sequencer.set_min_end_time(self.cursor_beats);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        clock::{BeatValue, Clock, TimeSignature},
        controllers::orchestrator::tests::TestOrchestrator,
        entities::BoxedEntity,
        messages::tests::TestMessage,
        settings::PatternSettings,
        traits::{TestInstrument, Updateable},
        utils::Timer,
        Orchestrator,
    };
    use assert_approx_eq::assert_approx_eq;

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

    #[test]
    fn test_pattern() {
        let time_signature = TimeSignature::default();
        let mut sequencer = BeatSequencer::default();
        let mut programmer = PatternProgrammer::<TestMessage>::new_with(&time_signature);

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
        assert_eq!(
            programmer.cursor(),
            PatternProgrammer::<TestMessage>::CURSOR_BEGIN
        );
        programmer.reset_cursor();
        assert_eq!(
            programmer.cursor(),
            PatternProgrammer::<TestMessage>::CURSOR_BEGIN
        );

        programmer.insert_pattern_at_cursor(&mut sequencer, &0, &pattern);
        assert_eq!(
            programmer.cursor(),
            PerfectTimeUnit::from(2 * time_signature.top)
        );
        assert_eq!(sequencer.debug_events().len(), expected_note_count * 2); // one on, one off
    }

    // A pattern of all zeroes should last as long as a pattern of nonzeroes.
    #[test]
    fn test_empty_pattern() {
        let time_signature = TimeSignature::default();
        let mut sequencer = Box::new(BeatSequencer::<EntityMessage>::default());
        let mut programmer = PatternProgrammer::<EntityMessage>::new_with(&time_signature);

        let note_pattern = vec!["0".to_string()];
        let pattern_settings = PatternSettings {
            id: String::from("test-pattern"),
            note_value: Some(BeatValue::Quarter),
            notes: vec![note_pattern],
        };

        let pattern = Pattern::from_settings(&pattern_settings);

        assert_eq!(pattern.notes.len(), 1); // one track of notes
        assert_eq!(pattern.notes[0].len(), 1); // one note in track

        programmer.insert_pattern_at_cursor(&mut sequencer, &0, &pattern);
        assert_eq!(
            programmer.cursor(),
            PerfectTimeUnit::from(time_signature.top)
        );
        assert_eq!(sequencer.debug_events().len(), 0);

        let mut o = TestOrchestrator::default();
        let _ = o.add(None, BoxedEntity::BeatSequencer(sequencer));
        let mut clock = Clock::default();
        if let Ok(result) = o.run(&mut clock) {
            assert_eq!(
                result.len(),
                ((60.0 * 4.0 / clock.bpm()) * clock.sample_rate() as f32) as usize
            );
        }
    }

    #[test]
    fn test_multi_pattern_track() {
        let time_signature = TimeSignature::new_with(7, 8).expect("failed");
        let mut sequencer = BeatSequencer::default();
        let mut programmer = PatternProgrammer::<TestMessage>::new_with(&time_signature);

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

        programmer.insert_pattern_at_cursor(&mut sequencer, &0, &pattern);

        // expect max of (2, 3) measures
        assert_eq!(
            programmer.cursor(),
            PerfectTimeUnit::from(3 * time_signature.top)
        );
        assert_eq!(sequencer.debug_events().len(), expected_note_count * 2); // one on, one off
    }

    #[test]
    fn test_pattern_default_note_value() {
        let time_signature = TimeSignature::new_with(7, 4).expect("failed");
        let mut sequencer = BeatSequencer::default();
        let mut programmer = PatternProgrammer::<TestMessage>::new_with(&time_signature);
        let pattern = Pattern::<Note>::from_settings(&PatternSettings {
            id: String::from("test-pattern-inherit"),
            note_value: None,
            notes: vec![vec![String::from("1")]],
        });
        programmer.insert_pattern_at_cursor(&mut sequencer, &0, &pattern);

        assert_eq!(
            programmer.cursor(),
            PerfectTimeUnit::from(time_signature.top)
        );
    }

    #[test]
    fn test_random_access() {
        const INSTRUMENT_MIDI_CHANNEL: MidiChannel = 7;
        let mut o = Orchestrator::<TestMessage>::default();
        let mut sequencer = Box::new(BeatSequencer::<EntityMessage>::default());
        let mut programmer =
            PatternProgrammer::<EntityMessage>::new_with(&TimeSignature::default());
        let mut pattern = Pattern::<Note>::default();

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
        programmer.insert_pattern_at_cursor(&mut sequencer, &INSTRUMENT_MIDI_CHANNEL, &pattern);

        let midi_recorder = Box::new(TestInstrument::default());
        let midi_recorder_uid = o.add(None, BoxedEntity::TestInstrument(midi_recorder));
        o.connect_midi_downstream(midi_recorder_uid, INSTRUMENT_MIDI_CHANNEL);

        // Test recorder has seen nothing to start with.
        // TODO assert!(midi_recorder.debug_messages.is_empty());

        let mut clock = Clock::default();
        let sample_rate = clock.sample_rate();
        let mut o = Box::new(Orchestrator::<TestMessage>::default());
        let _sequencer_uid = o.add(None, BoxedEntity::BeatSequencer(sequencer));

        assert!(o.run(&mut clock,).is_ok());

        // We should have gotten one on and one off for each note in the
        // pattern.
        // TODO
        // assert_eq!(
        //     midi_recorder.debug_messages.len(),
        //     pattern.notes[0].len() * 2
        // );

        // TODO sequencer.debug_dump_events();

        // The comment below is incorrect; it was true when the beat sequencer
        // ended after sending the last note event, rather than thinking in
        // terms of full measures.
        //
        // WRONG: The clock should stop at the last note-off, which is 1.01
        // WRONG: beats past the start of the third note, which started at 2.0.
        // WRONG: Since the fourth note is zero-duration, it actually ends at 3.0,
        // WRONG: before the third note's note-off event happens.
        let last_beat = 4.0;
        assert_approx_eq!(
            clock.beats(),
            last_beat,
            1.5 / sample_rate as f32 // The extra 0.5 is for f32 precision
        );
        assert_eq!(
            clock.samples() - 1, // TODO: -1 is probably wrong
            clock.settings().beats_to_samples(last_beat)
        );

        // Start test recorder over again.
        // TODO midi_recorder.debug_messages.clear();

        // Rewind clock to start.
        clock.reset();
        // This shouldn't explode.
        let _ = o.update(&clock, TestMessage::Tick);

        // Only the first time slice's events should have fired.
        // TODO assert_eq!(midi_recorder.debug_messages.len(), 1);

        // Fast-forward to the end. Nothing else should fire. This is because
        // any tick() should do work for just the slice specified.
        clock.debug_set_seconds(10.0);
        let _ = o.update(&clock, TestMessage::Tick);
        // TODO assert_eq!(midi_recorder.debug_messages.len(), 1);

        // Start test recorder over again.
        // TODO midi_recorder.debug_messages.clear();

        // Move just past first note.
        clock.set_samples(1);

        // Keep going until just before half of second beat. We should see the
        // first note off (not on!) and the second note on/off.
        let _ = o.add(None, BoxedEntity::Timer(Box::new(Timer::new_with(2.0))));
        assert!(o.run(&mut clock).is_ok());
        // TODO assert_eq!(midi_recorder.debug_messages.len(), 3);

        // Keep ticking through start of second beat. Should see one more event:
        // #3 on.
        assert!(o.run(&mut clock).is_ok());
        // TODO dbg!(&midi_recorder.debug_messages);
        // TODO assert_eq!(midi_recorder.debug_messages.len(), 4);
    }
}
