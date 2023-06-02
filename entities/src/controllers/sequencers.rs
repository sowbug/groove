// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::EntityMessage;
use btreemultimap::BTreeMultiMap;
use groove_core::{
    midi::{HandlesMidi, MidiChannel, MidiMessage, MidiNoteMinder},
    time::{Clock, ClockParams, MusicalTime, PerfectTimeUnit, SampleRate, TimeSignatureParams},
    traits::{Configurable, Controls, IsController, Performs},
    ParameterType,
};
use groove_proc_macros::{Control, Params, Uid};
use std::{
    fmt::Debug,
    ops::{
        Bound::{Excluded, Included},
        Range,
    },
};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

pub(crate) type BeatEventsMap = BTreeMultiMap<MusicalTime, (MidiChannel, MidiMessage)>;

/// [Sequencer] produces MIDI according to a programmed sequence. Its unit of
/// time is the beat.
#[derive(Debug, Control, Params, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Sequencer {
    uid: usize,
    #[control]
    #[params]
    bpm: ParameterType,
    #[cfg_attr(feature = "serialization", serde(skip))]
    next_instant: PerfectTimeUnit,
    #[cfg_attr(feature = "serialization", serde(skip))]
    events: BeatEventsMap,
    #[cfg_attr(feature = "serialization", serde(skip))]
    last_event_time: MusicalTime,
    is_disabled: bool,
    is_performing: bool,

    #[cfg_attr(feature = "serialization", serde(skip))]
    should_stop_pending_notes: bool,
    #[cfg_attr(feature = "serialization", serde(skip))]
    active_notes: [MidiNoteMinder; 16],

    loop_range: Option<Range<PerfectTimeUnit>>,
    is_loop_enabled: bool,

    temp_hack_clock: Clock,

    #[cfg_attr(feature = "serialization", serde(skip))]
    time_range: Range<MusicalTime>,
    #[cfg_attr(feature = "serialization", serde(skip))]
    time_range_handled: bool,
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

    fn set_loop(&mut self, range: &std::ops::Range<PerfectTimeUnit>) {
        self.loop_range = Some(range.clone());
    }

    fn clear_loop(&mut self) {
        self.loop_range = None;
    }

    fn set_loop_enabled(&mut self, is_enabled: bool) {
        self.is_loop_enabled = is_enabled;
    }

    fn is_performing(&self) -> bool {
        self.is_performing
    }
}
impl Sequencer {
    pub fn new_with(params: &SequencerParams) -> Self {
        Self {
            uid: Default::default(),
            bpm: params.bpm(),
            next_instant: Default::default(),
            events: Default::default(),
            last_event_time: Default::default(),
            is_disabled: Default::default(),
            is_performing: Default::default(),
            should_stop_pending_notes: Default::default(),
            active_notes: Default::default(),
            loop_range: Default::default(),
            is_loop_enabled: Default::default(),
            temp_hack_clock: Clock::new_with(&ClockParams {
                bpm: params.bpm(),
                midi_ticks_per_second: 0,
                time_signature: TimeSignatureParams { top: 4, bottom: 4 }, // TODO
            }),
            time_range: Default::default(),
            time_range_handled: Default::default(),
        }
    }

    pub(crate) fn clear(&mut self) {
        self.events.clear();
        self.last_event_time = Default::default();
        self.skip_to_start();
    }

    pub(crate) fn cursor(&self) -> &MusicalTime {
        &self.time_range.start
    }

    pub fn insert(&mut self, when: &MusicalTime, channel: MidiChannel, message: MidiMessage) {
        self.events.insert(*when, (channel, message));
        if *when > self.last_event_time {
            self.last_event_time = *when;
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

    // fn is_finished(&self) -> bool {
    //     (self.events.is_empty() && self.last_event_time == Default::default())
    //         || self.next_instant > self.last_event_time
    // }

    // In the case of a silent pattern, we don't ask the sequencer to insert any
    // notes, yet we do want the sequencer to run until the end of the measure.
    // So we provide a facility to advance the end-time marker (which might be a
    // no-op if it's already later than requested).
    pub fn set_min_end_time(&mut self, when: &MusicalTime) {
        if &self.last_event_time < when {
            self.last_event_time = when.clone();
        }
    }

    pub fn next_instant(&self) -> PerfectTimeUnit {
        self.next_instant
    }

    fn stop_pending_notes(&mut self) -> Vec<EntityMessage> {
        let mut v = Vec::new();
        for channel in 0..MidiChannel::MAX {
            let channel_msgs = self.active_notes[channel as usize].generate_off_messages();
            for msg in channel_msgs.into_iter() {
                v.push(EntityMessage::Midi(channel.into(), msg));
            }
        }
        v
    }

    fn generate_midi_messages_for_interval(
        &mut self,
        range: &Range<MusicalTime>,
        messages_fn: &mut dyn FnMut(MidiChannel, MidiMessage),
    ) {
        let range = (Included(range.start), Excluded(range.end));
        self.events.range(range).for_each(|(_when, event)| {
            self.active_notes[event.0.value() as usize].watch_message(&event.1);
            messages_fn(event.0, event.1);
        });
    }

    pub fn generate_midi_messages_for_current_frame(
        &mut self,
        messages_fn: &mut dyn FnMut(MidiChannel, MidiMessage),
    ) {
        let time_range = self.time_range.clone();
        self.generate_midi_messages_for_interval(&time_range, messages_fn)
    }

    pub fn debug_events(&self) -> &BeatEventsMap {
        &self.events
    }

    #[allow(dead_code)]
    pub fn debug_dump_events(&self) {
        println!("{:?}", self.events);
    }

    #[cfg(feature = "iced-framework")]
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
impl Configurable for Sequencer {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.temp_hack_clock.update_sample_rate(sample_rate);

        // TODO: how can we make sure this stays in sync with the clock when the
        // clock is changed?
        self.next_instant = PerfectTimeUnit(self.temp_hack_clock.beats());
    }
}
impl Controls for Sequencer {
    type Message = EntityMessage;

    fn update_time(&mut self, range: &Range<MusicalTime>) {
        if &self.time_range != range {
            self.time_range = range.clone();
            self.time_range_handled = false;
        }
    }

    fn work(&mut self) -> Option<Vec<Self::Message>> {
        if !self.is_performing || self.is_finished() {
            return None;
        }
        let mut v = Vec::default();

        if self.should_stop_pending_notes {
            self.should_stop_pending_notes = false;
            v.extend(self.stop_pending_notes());
        }

        if self.is_enabled() {
            if !self.time_range_handled {
                self.time_range_handled = true;
                let time_range = self.time_range.clone();
                self.generate_midi_messages_for_interval(&time_range, &mut |channel, message| {
                    v.push(EntityMessage::Midi(channel, message))
                });
            }
        };

        // if self.is_loop_enabled {
        //     // This code block is a little weird because we needed to avoid the
        //     // mutable self method call while we are borrowing loop_range.
        //     let should_loop_now = if let Some(lr) = &self.loop_range {
        //         if lr.contains(&this_instant) && !lr.contains(&self.next_instant) {
        //             self.next_instant = lr.start;
        //             self.temp_hack_clock.seek_beats(lr.start.0);
        //             true
        //         } else {
        //             false
        //         }
        //     } else {
        //         false
        //     };
        //     if should_loop_now {
        //         v.extend(self.stop_pending_notes());
        //     }
        // }

        if v.is_empty() {
            None
        } else {
            Some(v)
        }
    }

    fn is_finished(&self) -> bool {
        self.time_range.start >= self.last_event_time
    }
}

#[cfg(feature = "egui-framework")]
mod gui {
    use super::Sequencer;
    use eframe::egui::{RichText, Ui};
    use groove_core::traits::gui::Shows;

    impl Shows for Sequencer {
        fn show(&mut self, ui: &mut Ui) {
            for (when, (channel, message)) in &self.events {
                let has_played = when < &self.time_range.start;
                let mut text = RichText::new(format!("{}: {} -> {:?}", when, channel, message));
                if has_played {
                    text = text.italics();
                }
                ui.label(text);
            }
        }
    }
}

#[cfg(tired)]
mod tired {
    pub(crate) type MidiTickEventsMap = BTreeMultiMap<MidiTicks, (MidiChannel, MidiMessage)>;

    /// [MidiTickSequencer] is another kind of sequencer whose time unit is the MIDI
    /// tick. It exists to make it easy for [MidiSmfReader] to turn MIDI files into
    /// sequences.
    #[derive(Debug, Control, Params, Uid)]
    #[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
    pub struct MidiTickSequencer {
        uid: usize,

        #[control]
        #[params]
        midi_ticks_per_second: usize,

        #[cfg_attr(feature = "serialization", serde(skip))]
        next_instant: MidiTicks,
        #[cfg_attr(feature = "serialization", serde(skip))]
        events: MidiTickEventsMap,
        #[cfg_attr(feature = "serialization", serde(skip))]
        last_event_time: MidiTicks,
        #[cfg_attr(feature = "serialization", serde(skip))]
        is_disabled: bool,
        #[cfg_attr(feature = "serialization", serde(skip))]
        is_performing: bool,
        #[cfg_attr(feature = "serialization", serde(skip))]
        active_notes: [MidiNoteMinder; 16],

        loop_range: Option<Range<PerfectTimeUnit>>,
        is_loop_enabled: bool,

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

        fn set_loop(&mut self, range: &std::ops::Range<PerfectTimeUnit>) {
            self.loop_range = Some(range.clone());
        }

        fn clear_loop(&mut self) {
            self.loop_range = None;
        }

        fn set_loop_enabled(&mut self, is_enabled: bool) {
            self.is_loop_enabled = is_enabled;
        }

        fn is_performing(&self) -> bool {
            self.is_performing
        }
    }

    impl MidiTickSequencer {
        pub fn new_with(params: &MidiTickSequencerParams) -> Self {
            Self {
                uid: Default::default(),
                midi_ticks_per_second: params.midi_ticks_per_second(),
                next_instant: Default::default(),
                events: Default::default(),
                last_event_time: Default::default(),
                is_disabled: Default::default(),
                is_performing: Default::default(),
                active_notes: Default::default(),
                loop_range: Default::default(),
                is_loop_enabled: Default::default(),
                temp_hack_clock: Clock::new_with(&ClockParams {
                    bpm: 0.0,
                    midi_ticks_per_second: params.midi_ticks_per_second(),
                    time_signature: TimeSignatureParams { top: 4, bottom: 4 }, // TODO
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

        #[cfg(feature = "iced-framework")]
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
    impl Configurable for MidiTickSequencer {
        fn reset(&mut self, sample_rate: SampleRate) {
            self.temp_hack_clock.set_sample_rate(sample_rate);
            self.temp_hack_clock.reset(sample_rate);

            // TODO: how can we make sure this stays in sync with the clock when the
            // clock is changed?
            self.next_instant = MidiTicks(0);
        }
    }
    impl Controls for MidiTickSequencer {
        type Message = EntityMessage;

        fn work(&mut self) -> Option<Vec<Self::Message>> {
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
                        self.active_notes[*channel as usize].watch_message(message);
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
                            midly::MetaMessage::TimeSignature(
                                numerator,
                                denominator_exp,
                                _cc,
                                _bb,
                            ) => {
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
                                let seconds_per_quarter_note: f32 =
                                    meta_info.tempo as f32 / 1000000.0;
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
}

#[cfg(test)]
mod tests {
    #[cfg(tired)]
    use super::{MidiTickEventsMap, MidiTickSequencer};
    use crate::tests::{DEFAULT_BPM, DEFAULT_MIDI_TICKS_PER_SECOND};
    #[cfg(tired)]
    use groove_core::time::MidiTicks;
    use groove_core::{
        midi::MidiChannel,
        time::{Clock, ClockParams, TimeSignatureParams},
    };

    #[cfg(tired)]
    impl MidiTickSequencer {
        #[allow(dead_code)]
        pub(crate) fn debug_events(&self) -> &MidiTickEventsMap {
            &self.events
        }
    }

    #[cfg(tired)]
    impl MidiTickSequencer {
        pub(crate) fn tick_for_beat(&self, clock: &Clock, beat: usize) -> MidiTicks {
            //            let tpb = self.midi_ticks_per_second.0 as f32 /
            //            (clock.bpm() / 60.0);
            let tpb = 960.0 / (clock.bpm() / 60.0); // TODO: who should own the number of ticks/second?
            MidiTicks::from(tpb * beat as f64)
        }
    }

    // fn advance_to_next_beat(
    //     clock: &mut Clock,
    //     sequencer: &mut dyn IsController<Message = EntityMessage>,
    // ) {
    //     let next_beat = clock.beats().floor() + 1.0;
    //     while clock.beats() < next_beat {
    //         // TODO: a previous version of this utility function had
    //         // clock.tick() first, meaning that the sequencer never got the 0th
    //         // (first) tick. No test ever cared, apparently. Fix this.
    //         let _ = sequencer.work(1);
    //         clock.tick(1);
    //     }
    // }

    // // We're papering over the issue that MIDI events are firing a little late.
    // // See Clock::next_slice_in_midi_ticks().
    // fn advance_one_midi_tick(
    //     clock: &mut Clock,
    //     sequencer: &mut dyn IsController<Message = EntityMessage>,
    // ) {
    //     let next_midi_tick = clock.midi_ticks() + 1;
    //     while clock.midi_ticks() < next_midi_tick {
    //         let _ = sequencer.work(1);
    //         clock.tick(1);
    //     }
    // }

    #[allow(dead_code)]
    #[allow(unused_variables)]
    #[test]
    fn sequencer_mainline() {
        const DEVICE_MIDI_CHANNEL: MidiChannel = MidiChannel::new(7);
        let mut clock = Clock::new_with(&ClockParams {
            bpm: DEFAULT_BPM,
            midi_ticks_per_second: DEFAULT_MIDI_TICKS_PER_SECOND,
            time_signature: TimeSignatureParams { top: 4, bottom: 4 },
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
