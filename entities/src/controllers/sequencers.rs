// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::EntityMessage;
use btreemultimap::BTreeMultiMap;
use groove_core::{
    midi::{new_note_off, u7, HandlesMidi, MidiChannel, MidiMessage},
    time::{Clock, ClockNano, MidiTicks, PerfectTimeUnit, TimeSignature},
    traits::{IsController, Performs, Resets, TicksWithMessages},
    ParameterType,
};
use groove_proc_macros::{Nano, Uid};
use midly::TrackEventKind;
use rustc_hash::FxHashMap;
use std::str::FromStr;
use std::{
    fmt::Debug,
    ops::Bound::{Excluded, Included},
};

use strum::EnumCount;
use strum_macros::{Display, EnumCount as EnumCountMacro, EnumString, FromRepr, IntoStaticStr};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "egui-framework")]
use {eframe::egui, groove_core::traits::Shows};

pub(crate) type BeatEventsMap = BTreeMultiMap<PerfectTimeUnit, (MidiChannel, MidiMessage)>;

/// [Sequencer] produces MIDI according to a programmed sequence. Its unit of
/// time is the beat.
#[derive(Debug, Nano, Uid)]
pub struct Sequencer {
    uid: usize,
    #[nano]
    bpm: ParameterType,
    next_instant: PerfectTimeUnit,
    events: BeatEventsMap,
    last_event_time: PerfectTimeUnit,
    is_disabled: bool,
    is_performing: bool,

    should_stop_pending_notes: bool,
    on_notes: FxHashMap<u7, MidiChannel>,

    temp_hack_clock: Clock,
}
impl IsController for Sequencer {}
impl HandlesMidi for Sequencer {}
impl Performs for Sequencer {
    fn play(&mut self) {
        self.is_performing = true;
    }

    fn stop(&mut self) {
        self.is_performing = false;
        self.should_stop_pending_notes = true;
    }

    fn skip_to_start(&mut self) {
        self.temp_hack_clock.seek(0);
        self.next_instant = PerfectTimeUnit::default();
        self.should_stop_pending_notes = true;
    }
}
impl Sequencer {
    pub fn new_with(params: SequencerNano) -> Self {
        Self {
            uid: Default::default(),
            bpm: params.bpm(),
            next_instant: Default::default(),
            events: Default::default(),
            last_event_time: Default::default(),
            is_disabled: Default::default(),
            is_performing: Default::default(),
            should_stop_pending_notes: Default::default(),
            on_notes: Default::default(),
            temp_hack_clock: Clock::new_with(ClockNano {
                bpm: params.bpm(),
                midi_ticks_per_second: 0,
                time_signature: TimeSignature { top: 4, bottom: 4 }, // TODO
            }),
        }
    }

    pub(crate) fn clear(&mut self) {
        self.events.clear();
        self.last_event_time = PerfectTimeUnit::default();
        self.skip_to_start();
    }

    pub(crate) fn cursor_in_beats(&self) -> f64 {
        self.temp_hack_clock.beats()
    }

    pub fn insert(&mut self, when: PerfectTimeUnit, channel: MidiChannel, message: MidiMessage) {
        self.events.insert(when, (channel, message));
        if when > self.last_event_time {
            self.last_event_time = when;
        }
    }

    pub fn is_enabled(&self) -> bool {
        !self.is_disabled
    }

    pub fn enable(&mut self, is_enabled: bool) {
        if !self.is_disabled && !is_enabled {
            self.should_stop_pending_notes = true;
        }
        self.is_disabled = !is_enabled;
    }

    fn is_finished(&self) -> bool {
        (self.events.is_empty() && self.last_event_time == PerfectTimeUnit(0.0))
            || self.next_instant > self.last_event_time
    }

    // In the case of a silent pattern, we don't ask the sequencer to insert any
    // notes, yet we do want the sequencer to run until the end of the measure.
    // So we provide a facility to advance the end-time marker (which might be a
    // no-op if it's already later than requested).
    pub fn set_min_end_time(&mut self, when: PerfectTimeUnit) {
        if self.last_event_time < when {
            self.last_event_time = when;
        }
    }

    pub fn next_instant(&self) -> PerfectTimeUnit {
        self.next_instant
    }

    fn stop_pending_notes(&mut self) -> Vec<EntityMessage> {
        let mut v = Vec::new();
        for on_note in &self.on_notes {
            let note = *on_note.0;
            let channel = *on_note.1;
            v.push(EntityMessage::Midi(channel, new_note_off(note.into(), 0)));
        }
        v
    }

    fn generate_midi_messages_for_interval(
        &mut self,
        begin: PerfectTimeUnit,
        end: PerfectTimeUnit,
    ) -> Option<Vec<(MidiChannel, MidiMessage)>> {
        let range = (Included(begin), Excluded(end));
        let v = self
            .events
            .range(range)
            .fold(Vec::new(), |mut vec, (_when, event)| {
                match event.1 {
                    MidiMessage::NoteOff { key, vel: _ } => {
                        self.on_notes.remove(&key);
                    }
                    MidiMessage::NoteOn { key, vel } => {
                        if vel == 0 {
                            self.on_notes.remove(&key);
                        }
                        self.on_notes.insert(key, event.0);
                    }
                    _ => {}
                }
                vec.push((event.0, event.1));
                vec
            });
        if v.is_empty() {
            None
        } else {
            Some(v)
        }
    }

    pub fn generate_midi_messages_for_current_frame(&mut self) -> Option<Vec<(u8, MidiMessage)>> {
        self.generate_midi_messages_for_interval(
            PerfectTimeUnit(self.temp_hack_clock.beats()),
            PerfectTimeUnit(self.temp_hack_clock.next_slice_in_beats()),
        )
    }

    pub fn debug_events(&self) -> &BeatEventsMap {
        &self.events
    }

    #[allow(dead_code)]
    pub fn debug_dump_events(&self) {
        println!("{:?}", self.events);
    }

    pub fn update(&mut self, message: SequencerMessage) {
        match message {
            SequencerMessage::Sequencer(s) => *self = Self::new_with(s),
            _ => self.derived_update(message),
        }
    }

    pub fn bpm(&self) -> f64 {
        self.bpm
    }

    pub fn set_bpm(&mut self, bpm: ParameterType) {
        self.bpm = bpm;
    }
}
impl Resets for Sequencer {
    fn reset(&mut self, sample_rate: usize) {
        self.temp_hack_clock.reset(sample_rate);

        // TODO: how can we make sure this stays in sync with the clock when the
        // clock is changed?
        self.next_instant = PerfectTimeUnit(self.temp_hack_clock.beats());
    }
}
impl TicksWithMessages for Sequencer {
    type Message = EntityMessage;

    fn tick(&mut self, tick_count: usize) -> (std::option::Option<Vec<Self::Message>>, usize) {
        if !self.is_performing {
            return (None, tick_count);
        }
        if self.is_finished() {
            // TODO: since this code ensures we'll end only on even frame
            // boundaries, it's likely to be masking edge cases. Consider
            // developing a smarter way to determine the exact last frame
            // without devolving to frame-by-frame iteration.
            return (None, 0);
        }
        let mut v = Vec::default();
        let this_instant = PerfectTimeUnit(self.temp_hack_clock.beats());
        self.temp_hack_clock.tick_batch(tick_count);
        self.next_instant = PerfectTimeUnit(self.temp_hack_clock.beats());

        if self.should_stop_pending_notes {
            self.should_stop_pending_notes = false;
            v.extend(self.stop_pending_notes());
        }

        if self.is_enabled() {
            if let Some(messages) =
                self.generate_midi_messages_for_interval(this_instant, self.next_instant)
            {
                v.extend(messages.iter().map(|m| EntityMessage::Midi(m.0, m.1)));
            }
        };
        if v.is_empty() {
            (None, tick_count)
        } else {
            (Some(v), tick_count)
        }
    }
}
#[cfg(feature = "egui-framework")]
impl Shows for Sequencer {
    fn show(&mut self, ui: &mut egui::Ui) {
        ui.label("Coming soon!");
    }
}

pub(crate) type MidiTickEventsMap = BTreeMultiMap<MidiTicks, (MidiChannel, MidiMessage)>;

/// [MidiTickSequencer] is another kind of sequencer whose time unit is the MIDI
/// tick. It exists to make it easy for [MidiSmfReader] to turn MIDI files into
/// sequences.
#[derive(Debug, Nano, Uid)]
pub struct MidiTickSequencer {
    uid: usize,

    #[nano]
    midi_ticks_per_second: usize,

    next_instant: MidiTicks,
    events: MidiTickEventsMap,
    last_event_time: MidiTicks,
    is_disabled: bool,
    is_performing: bool,

    temp_hack_clock: Clock,
}
impl IsController for MidiTickSequencer {}
impl HandlesMidi for MidiTickSequencer {}
impl Performs for MidiTickSequencer {
    fn play(&mut self) {
        self.is_performing = true;
    }

    fn stop(&mut self) {
        self.is_performing = false;
    }

    fn skip_to_start(&mut self) {
        self.temp_hack_clock.seek(0);
        self.next_instant = MidiTicks::MIN;
    }
}

impl MidiTickSequencer {
    pub fn new_with(params: MidiTickSequencerNano) -> Self {
        Self {
            uid: Default::default(),
            midi_ticks_per_second: params.midi_ticks_per_second(),
            next_instant: Default::default(),
            events: Default::default(),
            last_event_time: Default::default(),
            is_disabled: Default::default(),
            is_performing: Default::default(),
            temp_hack_clock: Clock::new_with(ClockNano {
                bpm: 0.0,
                midi_ticks_per_second: params.midi_ticks_per_second(),
                time_signature: TimeSignature { top: 4, bottom: 4 }, // TODO
            }),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn clear(&mut self) {
        // TODO: should this also disconnect sinks? I don't think so
        self.events.clear();
        self.last_event_time = MidiTicks::MIN;
        self.skip_to_start();
    }

    pub fn insert(&mut self, when: MidiTicks, channel: MidiChannel, message: MidiMessage) {
        self.events.insert(when, (channel, message));
        if when >= self.last_event_time {
            self.last_event_time = when;
        }
    }

    pub fn is_enabled(&self) -> bool {
        !self.is_disabled
    }

    #[allow(dead_code)]
    pub fn enable(&mut self, is_enabled: bool) {
        self.is_disabled = !is_enabled;
    }

    fn is_finished(&self) -> bool {
        self.next_instant > self.last_event_time
    }

    pub fn update(&mut self, message: MidiTickSequencerMessage) {
        match message {
            MidiTickSequencerMessage::MidiTickSequencer(s) => *self = Self::new_with(s),
            _ => self.derived_update(message),
        }
    }

    pub fn midi_ticks_per_second(&self) -> usize {
        self.midi_ticks_per_second
    }

    pub fn set_midi_ticks_per_second(&mut self, midi_ticks_per_second: usize) {
        self.midi_ticks_per_second = midi_ticks_per_second;
    }
}
impl Resets for MidiTickSequencer {
    fn reset(&mut self, sample_rate: usize) {
        self.temp_hack_clock.set_sample_rate(sample_rate);
        self.temp_hack_clock.reset(sample_rate);

        // TODO: how can we make sure this stays in sync with the clock when the
        // clock is changed?
        self.next_instant = MidiTicks(0);
    }
}
impl TicksWithMessages for MidiTickSequencer {
    type Message = EntityMessage;

    fn tick(&mut self, tick_count: usize) -> (std::option::Option<Vec<Self::Message>>, usize) {
        if self.is_finished() || !self.is_performing {
            return (None, 0);
        }
        let mut v = Vec::default();
        let this_instant = MidiTicks(self.temp_hack_clock.midi_ticks());
        self.temp_hack_clock.tick_batch(tick_count);
        self.next_instant = MidiTicks(self.temp_hack_clock.midi_ticks());

        if self.is_enabled() {
            // If the last instant marks a new interval, then we want to include
            // any events scheduled at exactly that time. So the range is
            // inclusive.
            let range = (Included(this_instant), Excluded(self.next_instant));
            let events = self.events.range(range);
            v.extend(events.into_iter().fold(
                Vec::default(),
                |mut vec, (_when, (channel, message))| {
                    vec.push(EntityMessage::Midi(*channel, *message));
                    vec
                },
            ));
        }
        if v.is_empty() {
            (None, tick_count)
        } else {
            (Some(v), tick_count)
        }
    }
}

/// [MidiSmfReader] parses MIDI SMF files and programs [MidiTickSequencer] with
/// the data it finds.
pub struct MidiSmfReader {}
impl MidiSmfReader {
    pub fn program_sequencer(sequencer: &mut MidiTickSequencer, data: &[u8]) {
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

#[cfg(test)]
mod tests {
    use super::{MidiTickEventsMap, MidiTickSequencer};
    use crate::{
        messages::EntityMessage,
        tests::{DEFAULT_BPM, DEFAULT_MIDI_TICKS_PER_SECOND},
    };
    use groove_core::{
        midi::MidiChannel,
        time::{Clock, ClockNano, MidiTicks, TimeSignature},
        traits::{IsController, Ticks},
    };

    impl MidiTickSequencer {
        #[allow(dead_code)]
        pub(crate) fn debug_events(&self) -> &MidiTickEventsMap {
            &self.events
        }
    }

    impl MidiTickSequencer {
        pub(crate) fn tick_for_beat(&self, clock: &Clock, beat: usize) -> MidiTicks {
            //            let tpb = self.midi_ticks_per_second.0 as f32 /
            //            (clock.bpm() / 60.0);
            let tpb = 960.0 / (clock.bpm() / 60.0); // TODO: who should own the number of ticks/second?
            MidiTicks::from(tpb * beat as f64)
        }
    }

    fn advance_to_next_beat(
        clock: &mut Clock,
        sequencer: &mut dyn IsController<Message = EntityMessage>,
    ) {
        let next_beat = clock.beats().floor() + 1.0;
        while clock.beats() < next_beat {
            // TODO: a previous version of this utility function had
            // clock.tick() first, meaning that the sequencer never got the 0th
            // (first) tick. No test ever cared, apparently. Fix this.
            let _ = sequencer.tick(1);
            clock.tick(1);
        }
    }

    // We're papering over the issue that MIDI events are firing a little late.
    // See Clock::next_slice_in_midi_ticks().
    fn advance_one_midi_tick(
        clock: &mut Clock,
        sequencer: &mut dyn IsController<Message = EntityMessage>,
    ) {
        let next_midi_tick = clock.midi_ticks() + 1;
        while clock.midi_ticks() < next_midi_tick {
            let _ = sequencer.tick(1);
            clock.tick(1);
        }
    }

    #[test]
    fn sequencer_mainline() {
        const DEVICE_MIDI_CHANNEL: MidiChannel = 7;
        let mut clock = Clock::new_with(ClockNano {
            bpm: DEFAULT_BPM,
            midi_ticks_per_second: DEFAULT_MIDI_TICKS_PER_SECOND,
            time_signature: TimeSignature { top: 4, bottom: 4 },
        });
        // let mut o = Orchestrator::new_with(DEFAULT_BPM);
        // let mut sequencer = Box::new(MidiTickSequencer::new_with(
        //     DEFAULT_SAMPLE_RATE,
        //     DEFAULT_MIDI_TICKS_PER_SECOND,
        // ));
        // let instrument = Box::new(ToyInstrument::new_with(clock.sample_rate()));
        // let device_uid = o.add(None, Entity::ToyInstrument(instrument));

        // sequencer.insert(
        //     sequencer.tick_for_beat(&clock, 0),
        //     DEVICE_MIDI_CHANNEL,
        //     new_note_on(MidiNote::C4 as u8, 127),
        // );
        // sequencer.insert(
        //     sequencer.tick_for_beat(&clock, 1),
        //     DEVICE_MIDI_CHANNEL,
        //     new_note_off(MidiNote::C4 as u8, 0),
        // );
        // const SEQUENCER_ID: &str = "seq";
        // let _sequencer_uid = o.add(Some(SEQUENCER_ID), Entity::MidiTickSequencer(sequencer));
        // o.connect_midi_downstream(device_uid, DEVICE_MIDI_CHANNEL);

        // // TODO: figure out a reasonable way to test these things once they're
        // // inside Store, and their type information has been erased. Maybe we
        // // can send messages asking for state. Maybe we can send things that the
        // // entities themselves assert.
        // if let Some(entity) = o.get_mut(SEQUENCER_ID) {
        //     if let Some(sequencer) = entity.as_is_controller_mut() {
        //         advance_one_midi_tick(&mut clock, sequencer);
        //         {
        //             // assert!(instrument.is_playing);
        //             // assert_eq!(instrument.received_count, 1);
        //             // assert_eq!(instrument.handled_count, 1);
        //         }
        //     }
        // }

        // if let Some(entity) = o.get_mut(SEQUENCER_ID) {
        //     if let Some(sequencer) = entity.as_is_controller_mut() {
        //         advance_to_next_beat(&mut clock, sequencer);
        //         {
        //             // assert!(!instrument.is_playing);
        //             // assert_eq!(instrument.received_count, 2);
        //             // assert_eq!(&instrument.handled_count, &2);
        //         }
        //     }
        // }
    }

    // TODO: re-enable later.......................................................................
    // #[test]
    // fn sequencer_multichannel() {
    //     let mut clock = Clock::default();
    //     let mut sequencer = MidiTickSequencer::<TestMessage>::default();

    //     let device_1 = rrc(TestMidiSink::default());
    //     assert!(!device_1.borrow().is_playing);
    //     device_1.borrow_mut().set_midi_channel(0);
    //     sequencer.add_midi_sink(0, rrc_downgrade::<TestMidiSink<TestMessage>>(&device_1));

    //     let device_2 = rrc(TestMidiSink::default());
    //     assert!(!device_2.borrow().is_playing);
    //     device_2.borrow_mut().set_midi_channel(1);
    //     sequencer.add_midi_sink(1, rrc_downgrade::<TestMidiSink<TestMessage>>(&device_2));

    //     sequencer.insert(
    //         sequencer.tick_for_beat(&clock, 0),
    //         0,
    //         new_note_on(60, 0),
    //     );
    //     sequencer.insert(
    //         sequencer.tick_for_beat(&clock, 1),
    //         1,
    //         new_note_on(60, 0),
    //     );
    //     sequencer.insert(
    //         sequencer.tick_for_beat(&clock, 2),
    //         0,
    //         new_note_off(MidiNote::C4 as u8, 0),
    //     );
    //     sequencer.insert(
    //         sequencer.tick_for_beat(&clock, 3),
    //         1,
    //         new_note_off(MidiNote::C4 as u8, 0),
    //     );
    //     assert_eq!(sequencer.debug_events().len(), 4);

    //     // Let the tick #0 event(s) fire.
    //     assert_eq!(clock.samples(), 0);
    //     assert_eq!(clock.midi_ticks(), 0);
    //     advance_one_midi_tick(&mut clock, &mut sequencer);
    //     {
    //         let dp_1 = device_1.borrow();
    //         assert!(dp_1.is_playing);
    //         assert_eq!(dp_1.received_count, 1);
    //         assert_eq!(dp_1.handled_count, 1);

    //         let dp_2 = device_2.borrow();
    //         assert!(!dp_2.is_playing);
    //         assert_eq!(dp_2.received_count, 0);
    //         assert_eq!(dp_2.handled_count, 0);
    //     }

    //     advance_to_next_beat(&mut clock, &mut sequencer);
    //     assert_eq!(clock.beats().floor(), 1.0); // TODO: these floor() calls are a smell
    //     {
    //         let dp = device_1.borrow();
    //         assert!(dp.is_playing);
    //         assert_eq!(dp.received_count, 1);
    //         assert_eq!(dp.handled_count, 1);

    //         let dp_2 = device_2.borrow();
    //         assert!(dp_2.is_playing);
    //         assert_eq!(dp_2.received_count, 1);
    //         assert_eq!(dp_2.handled_count, 1);
    //     }

    //     advance_to_next_beat(&mut clock, &mut sequencer);
    //     assert_eq!(clock.beats().floor(), 2.0);
    //     // assert_eq!(sequencer.tick_sequencer.events.len(), 1);
    //     {
    //         let dp = device_1.borrow();
    //         assert!(!dp.is_playing);
    //         assert_eq!(dp.received_count, 2);
    //         assert_eq!(dp.handled_count, 2);

    //         let dp_2 = device_2.borrow();
    //         assert!(dp_2.is_playing);
    //         assert_eq!(dp_2.received_count, 1);
    //         assert_eq!(dp_2.handled_count, 1);
    //     }

    //     advance_to_next_beat(&mut clock, &mut sequencer);
    //     assert_eq!(clock.beats().floor(), 3.0);
    //     // assert_eq!(sequencer.tick_sequencer.events.len(), 0);
    //     {
    //         let dp = device_1.borrow();
    //         assert!(!dp.is_playing);
    //         assert_eq!(dp.received_count, 2);
    //         assert_eq!(dp.handled_count, 2);

    //         let dp_2 = device_2.borrow();
    //         assert!(!dp_2.is_playing);
    //         assert_eq!(dp_2.received_count, 2);
    //         assert_eq!(dp_2.handled_count, 2);
    //     }
    // }
}
