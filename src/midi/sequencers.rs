use crate::{
    clock::{Clock, MidiTicks, PerfectTimeUnit},
    common::{rrc, rrc_downgrade, weak_new, Rrc, Ww},
    controllers::BigMessage,
    messages::GrooveMessage,
    midi::{MidiChannel, MidiMessage, MIDI_CHANNEL_RECEIVE_ALL},
    orchestrator::OrchestratorMessage,
    traits::{
        EvenNewerCommand, EvenNewerIsUpdateable, HasOverhead, HasUid, MessageBounds,
        MessageGeneratorT, NewIsController, NewUpdateable, Overhead, SinksMidi, SourcesMidi,
        Terminates, WatchesClock,
    },
};
use btreemultimap::BTreeMultiMap;
use std::{
    collections::HashMap,
    fmt::Debug,
    ops::Bound::{Excluded, Included},
};

pub(crate) type BeatEventsMap = BTreeMultiMap<PerfectTimeUnit, (MidiChannel, MidiMessage)>;

#[derive(Debug)]
pub struct BeatSequencer<M: MessageBounds> {
    uid: usize,
    pub(crate) me: Ww<Self>,
    overhead: Overhead,
    channels_to_sink_vecs: HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>>,
    next_instant: PerfectTimeUnit,
    events: BeatEventsMap,
    last_event_time: PerfectTimeUnit,
}
impl<M: MessageBounds> NewIsController for BeatSequencer<M> {}
impl<M: MessageBounds> NewUpdateable for BeatSequencer<M> {
    default type Message = M;

    default fn update(
        &mut self,
        clock: &Clock,
        message: Self::Message,
    ) -> EvenNewerCommand<Self::Message> {
        EvenNewerCommand::none()
    }
}
impl<M: MessageBounds> Terminates for BeatSequencer<M> {
    fn is_finished(&self) -> bool {
        self.next_instant > self.last_event_time
    }
}
impl<M: MessageBounds> HasUid for BeatSequencer<M> {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}

impl<M: MessageBounds> Default for BeatSequencer<M> {
    fn default() -> Self {
        Self {
            uid: usize::default(),
            me: weak_new(),
            overhead: Overhead::default(),
            channels_to_sink_vecs: Default::default(),
            next_instant: Default::default(),
            events: Default::default(),
            last_event_time: Default::default(),
        }
    }
}

impl<M: MessageBounds> BeatSequencer<M> {
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
        self.last_event_time = PerfectTimeUnit::default();
    }

    pub(crate) fn insert(
        &mut self,
        when: PerfectTimeUnit,
        channel: MidiChannel,
        message: MidiMessage,
    ) {
        self.events.insert(when, (channel, message));
        if when > self.last_event_time {
            self.last_event_time = when;
        }
    }
}

// TODO: what does it mean for a MIDI device to be muted?
impl<M: MessageBounds> HasOverhead for BeatSequencer<M> {
    fn overhead(&self) -> &Overhead {
        &self.overhead
    }

    fn overhead_mut(&mut self) -> &mut Overhead {
        &mut self.overhead
    }
}

impl<M: MessageBounds> SourcesMidi for BeatSequencer<M> {
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

impl<M: MessageBounds> WatchesClock for BeatSequencer<M> {
    fn tick(&mut self, clock: &Clock) -> Vec<BigMessage> {
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
                self.issue_midi(clock, &event.0, &event.1);
            }
        }
        Vec::new()
    }
}

impl<M: MessageBounds> EvenNewerIsUpdateable for BeatSequencer<M> {
    type Message = OrchestratorMessage;

    fn update(&mut self, message: Self::Message) -> EvenNewerCommand<Self::Message> {
        match message {
            OrchestratorMessage::Tick(clock) => {
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
                    EvenNewerCommand::batch(events.into_iter().fold(
                        Vec::new(),
                        |mut vec: Vec<EvenNewerCommand<Self::Message>>,
                         (_when, (channel, message))| {
                            vec.push(EvenNewerCommand::single(OrchestratorMessage::Midi(
                                clock.clone(),
                                *channel,
                                *message,
                            )));
                            vec
                        },
                    ))
                } else {
                    EvenNewerCommand::none()
                }
            }
            _ => EvenNewerCommand::none(),
        }
    }

    fn message_for(&self, _param_name: &str) -> Box<dyn MessageGeneratorT<Self::Message>> {
        todo!()
    }
}
impl NewUpdateable for BeatSequencer<GrooveMessage> {
    type Message = GrooveMessage;

    fn update(&mut self, clock: &Clock, message: Self::Message) -> EvenNewerCommand<Self::Message> {
        match message {
            GrooveMessage::Tick => {
                self.next_instant = PerfectTimeUnit(clock.next_slice_in_beats());

                return if self.overhead.is_enabled() {
                    // If the last instant marks a new interval, then we want to include
                    // any events scheduled at exactly that time. So the range is
                    // inclusive.
                    let range = (
                        Included(PerfectTimeUnit(clock.beats())),
                        Excluded(self.next_instant),
                    );
                    EvenNewerCommand::batch(self.events.range(range).into_iter().fold(
                        Vec::new(),
                        |mut vec, (_when, event)| {
                            vec.push(EvenNewerCommand::single(GrooveMessage::Midi(
                                event.0, event.1,
                            )));
                            vec
                        },
                    ))
                } else {
                    EvenNewerCommand::none()
                };
            }
            _ => todo!(),
        }
    }
}

pub(crate) type MidiTickEventsMap = BTreeMultiMap<MidiTicks, (MidiChannel, MidiMessage)>;

#[derive(Debug)]
pub struct MidiTickSequencer<M: MessageBounds> {
    uid: usize,
    pub(crate) me: Ww<Self>,
    overhead: Overhead,
    channels_to_sink_vecs: HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>>,
    next_instant: MidiTicks,
    events: MidiTickEventsMap,
    last_event_time: MidiTicks,
}
impl<M: MessageBounds> NewIsController for MidiTickSequencer<M> {}
impl<M: MessageBounds> NewUpdateable for MidiTickSequencer<M> {
    default type Message = M;

    default fn update(
        &mut self,
        clock: &Clock,
        message: Self::Message,
    ) -> EvenNewerCommand<Self::Message> {
        EvenNewerCommand::none()
    }
}
impl<M: MessageBounds> Terminates for MidiTickSequencer<M> {
    fn is_finished(&self) -> bool {
        self.next_instant > self.last_event_time
    }
}
impl<M: MessageBounds> HasUid for MidiTickSequencer<M> {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}

impl<M: MessageBounds> Default for MidiTickSequencer<M> {
    fn default() -> Self {
        Self {
            uid: usize::default(),
            me: weak_new(),
            overhead: Overhead::default(),
            channels_to_sink_vecs: Default::default(),
            next_instant: MidiTicks::MIN,
            events: Default::default(),
            last_event_time: MidiTicks::MIN,
        }
    }
}

impl<M: MessageBounds> MidiTickSequencer<M> {
    #[allow(dead_code)]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub(crate) fn new_wrapped() -> Rrc<Self> {
        let wrapped = rrc(Self::new());
        wrapped.borrow_mut().me = rrc_downgrade(&wrapped);
        wrapped
    }

    #[allow(dead_code)]
    pub(crate) fn clear(&mut self) {
        // TODO: should this also disconnect sinks? I don't think so
        self.events.clear();
        self.next_instant = MidiTicks::MIN;
        self.last_event_time = MidiTicks::MIN;
    }

    pub(crate) fn insert(&mut self, when: MidiTicks, channel: MidiChannel, message: MidiMessage) {
        self.events.insert(when, (channel, message));
        if when >= self.last_event_time {
            self.last_event_time = when;
        }
    }
}

// TODO: what does it mean for a MIDI device to be muted?
impl<M: MessageBounds> HasOverhead for MidiTickSequencer<M> {
    fn overhead(&self) -> &Overhead {
        &self.overhead
    }

    fn overhead_mut(&mut self) -> &mut Overhead {
        &mut self.overhead
    }
}

impl<M: MessageBounds> SourcesMidi for MidiTickSequencer<M> {
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

impl<M: MessageBounds> WatchesClock for MidiTickSequencer<M> {
    fn tick(&mut self, clock: &Clock) -> Vec<BigMessage> {
        self.next_instant = MidiTicks(clock.next_slice_in_midi_ticks());

        if self.overhead.is_enabled() {
            // If the last instant marks a new interval, then we want to include
            // any events scheduled at exactly that time. So the range is
            // inclusive.
            //
            // TODO: see comment in Clock::next_slice_in_midi_ticks about these
            // ranges firing MIDI events late.
            let range = (
                Included(MidiTicks(clock.midi_ticks())),
                Excluded(self.next_instant),
            );
            let events = self.events.range(range);
            for (_when, event) in events {
                self.issue_midi(clock, &event.0, &event.1);
            }
        }
        Vec::new()
    }
}

impl<M: MessageBounds> EvenNewerIsUpdateable for MidiTickSequencer<M> {
    type Message = OrchestratorMessage;

    fn update(&mut self, message: Self::Message) -> EvenNewerCommand<Self::Message> {
        match message {
            OrchestratorMessage::Tick(clock) => {
                self.next_instant = MidiTicks(clock.next_slice_in_midi_ticks());

                if self.overhead.is_enabled() {
                    // If the last instant marks a new interval, then we want to include
                    // any events scheduled at exactly that time. So the range is
                    // inclusive.
                    let range = (
                        Included(MidiTicks(clock.midi_ticks())),
                        Excluded(self.next_instant),
                    );
                    let events = self.events.range(range);
                    EvenNewerCommand::batch(events.into_iter().fold(
                        Vec::new(),
                        |mut vec: Vec<EvenNewerCommand<Self::Message>>,
                         (_when, (channel, message))| {
                            vec.push(EvenNewerCommand::single(OrchestratorMessage::Midi(
                                clock.clone(),
                                *channel,
                                *message,
                            )));
                            vec
                        },
                    ))
                } else {
                    EvenNewerCommand::none()
                }
            }
            _ => EvenNewerCommand::none(),
        }
    }

    fn message_for(&self, _param_name: &str) -> Box<dyn MessageGeneratorT<Self::Message>> {
        todo!()
    }
}

impl NewUpdateable for MidiTickSequencer<GrooveMessage> {
    type Message = GrooveMessage;

    fn update(&mut self, clock: &Clock, message: Self::Message) -> EvenNewerCommand<Self::Message> {
        match message {
            GrooveMessage::Tick => {
                self.next_instant = MidiTicks(clock.next_slice_in_midi_ticks());

                if self.overhead.is_enabled() {
                    // If the last instant marks a new interval, then we want to include
                    // any events scheduled at exactly that time. So the range is
                    // inclusive.
                    let range = (
                        Included(MidiTicks(clock.midi_ticks())),
                        Excluded(self.next_instant),
                    );
                    let events = self.events.range(range);
                    EvenNewerCommand::batch(events.into_iter().fold(
                        Vec::new(),
                        |mut vec: Vec<EvenNewerCommand<Self::Message>>,
                         (_when, (channel, message))| {
                            vec.push(EvenNewerCommand::single(GrooveMessage::Midi(
                                *channel, *message,
                            )));
                            vec
                        },
                    ))
                } else {
                    EvenNewerCommand::none()
                }
            }
            _ => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::{BeatEventsMap, BeatSequencer, MidiTickEventsMap, MidiTickSequencer};
    use crate::{
        clock::{Clock, MidiTicks, PerfectTimeUnit},
        common::{rrc, rrc_downgrade},
        messages::tests::TestMessage,
        midi::{MidiNote, MidiUtils},
        traits::{
            EvenNewerCommand, MessageBounds, NewUpdateable, SinksMidi, SourcesMidi, WatchesClock,
        },
        utils::tests::TestMidiSink,
    };
    use std::ops::Bound::{Excluded, Included};

    impl<M: MessageBounds> BeatSequencer<M> {
        pub fn debug_events(&self) -> &BeatEventsMap {
            &self.events
        }

        pub fn debug_dump_events(&self) {
            println!("{:?}", self.events);
        }
    }

    impl<M: MessageBounds> MidiTickSequencer<M> {
        pub(crate) fn debug_events(&self) -> &MidiTickEventsMap {
            &self.events
        }
    }

    impl<M: MessageBounds> MidiTickSequencer<M> {
        pub(crate) fn tick_for_beat(&self, clock: &Clock, beat: usize) -> MidiTicks {
            //            let tpb = self.midi_ticks_per_second.0 as f32 /
            //            (clock.settings().bpm() / 60.0);
            let tpb = 960.0 / (clock.settings().bpm() / 60.0); // TODO: who should own the number of ticks/second?
            MidiTicks::from(tpb * beat as f32)
        }
    }

    impl NewUpdateable for BeatSequencer<TestMessage> {
        type Message = TestMessage;

        fn update(
            &mut self,
            clock: &Clock,
            message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            match message {
                TestMessage::Tick => {
                    self.next_instant = PerfectTimeUnit(clock.next_slice_in_beats());

                    return if self.overhead.is_enabled() {
                        // If the last instant marks a new interval, then we want to include
                        // any events scheduled at exactly that time. So the range is
                        // inclusive.
                        let range = (
                            Included(PerfectTimeUnit(clock.beats())),
                            Excluded(self.next_instant),
                        );
                        EvenNewerCommand::batch(self.events.range(range).into_iter().fold(
                            Vec::new(),
                            |mut vec, (_when, event)| {
                                vec.push(EvenNewerCommand::single(TestMessage::Midi(
                                    event.0, event.1,
                                )));
                                vec
                            },
                        ))
                    } else {
                        EvenNewerCommand::none()
                    };
                }
                _ => todo!(),
            }
        }
    }

    impl NewUpdateable for MidiTickSequencer<TestMessage> {
        type Message = TestMessage;

        fn update(
            &mut self,
            clock: &Clock,
            message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            match message {
                TestMessage::Tick => {
                    self.next_instant = MidiTicks(clock.next_slice_in_midi_ticks());

                    if self.overhead.is_enabled() {
                        // If the last instant marks a new interval, then we want to include
                        // any events scheduled at exactly that time. So the range is
                        // inclusive.
                        let range = (
                            Included(MidiTicks(clock.midi_ticks())),
                            Excluded(self.next_instant),
                        );
                        let events = self.events.range(range);
                        EvenNewerCommand::batch(events.into_iter().fold(
                            Vec::new(),
                            |mut vec: Vec<EvenNewerCommand<Self::Message>>,
                             (_when, (channel, message))| {
                                vec.push(EvenNewerCommand::single(TestMessage::Midi(
                                    *channel, *message,
                                )));
                                vec
                            },
                        ))
                    } else {
                        EvenNewerCommand::none()
                    }
                }
                _ => todo!(),
            }
        }
    }

    fn advance_to_next_beat(clock: &mut Clock, sequencer: &mut MidiTickSequencer<TestMessage>) {
        let next_beat = clock.beats().floor() + 1.0;
        while clock.beats() < next_beat {
            clock.tick();
            sequencer.tick(clock);
        }
    }

    // We're papering over the issue that MIDI events are firing a little late.
    // See Clock::next_slice_in_midi_ticks().
    fn advance_one_midi_tick(clock: &mut Clock, sequencer: &mut MidiTickSequencer<TestMessage>) {
        let next_midi_tick = clock.midi_ticks() + 1;
        while clock.midi_ticks() < next_midi_tick {
            clock.tick();
            sequencer.tick(clock);
        }
    }

    #[test]
    fn test_sequencer() {
        let mut clock = Clock::new();
        let mut sequencer = MidiTickSequencer::new();

        let device = rrc(TestMidiSink::new_with(0));
        assert!(!device.borrow().is_playing);

        // These helpers create messages on channel zero.
        sequencer.insert(
            sequencer.tick_for_beat(&clock, 0),
            0,
            MidiUtils::note_on_c4(),
        );
        sequencer.insert(
            sequencer.tick_for_beat(&clock, 1),
            0,
            MidiUtils::note_off_c4(),
        );

        sequencer.add_midi_sink(0, rrc_downgrade::<TestMidiSink<TestMessage>>(&device));

        advance_one_midi_tick(&mut clock, &mut sequencer);
        {
            let dp = device.borrow();
            assert!(dp.is_playing);
            assert_eq!(dp.received_count, 1);
            assert_eq!(dp.handled_count, 1);
        }

        advance_to_next_beat(&mut clock, &mut sequencer);
        {
            let dp = device.borrow();
            assert!(!dp.is_playing);
            assert_eq!(dp.received_count, 2);
            assert_eq!(dp.handled_count, 2);
        }
    }

    #[test]
    fn test_sequencer_multichannel() {
        let mut clock = Clock::new();
        let mut sequencer = MidiTickSequencer::new();

        let device_1 = rrc(TestMidiSink::new());
        assert!(!device_1.borrow().is_playing);
        device_1.borrow_mut().set_midi_channel(0);
        sequencer.add_midi_sink(0, rrc_downgrade::<TestMidiSink<TestMessage>>(&device_1));

        let device_2 = rrc(TestMidiSink::new());
        assert!(!device_2.borrow().is_playing);
        device_2.borrow_mut().set_midi_channel(1);
        sequencer.add_midi_sink(1, rrc_downgrade::<TestMidiSink<TestMessage>>(&device_2));

        sequencer.insert(
            sequencer.tick_for_beat(&clock, 0),
            0,
            MidiUtils::new_note_on(60, 0),
        );
        sequencer.insert(
            sequencer.tick_for_beat(&clock, 1),
            1,
            MidiUtils::new_note_on(60, 0),
        );
        sequencer.insert(
            sequencer.tick_for_beat(&clock, 2),
            0,
            MidiUtils::new_note_off(MidiNote::C4 as u8, 0),
        );
        sequencer.insert(
            sequencer.tick_for_beat(&clock, 3),
            1,
            MidiUtils::new_note_off(MidiNote::C4 as u8, 0),
        );
        assert_eq!(sequencer.debug_events().len(), 4);

        // Let the tick #0 event(s) fire.
        assert_eq!(clock.samples(), 0);
        assert_eq!(clock.midi_ticks(), 0);
        advance_one_midi_tick(&mut clock, &mut sequencer);
        {
            let dp_1 = device_1.borrow();
            assert!(dp_1.is_playing);
            assert_eq!(dp_1.received_count, 1);
            assert_eq!(dp_1.handled_count, 1);

            let dp_2 = device_2.borrow();
            assert!(!dp_2.is_playing);
            assert_eq!(dp_2.received_count, 0);
            assert_eq!(dp_2.handled_count, 0);
        }

        advance_to_next_beat(&mut clock, &mut sequencer);
        assert_eq!(clock.beats().floor(), 1.0); // TODO: these floor() calls are a smell
        {
            let dp = device_1.borrow();
            assert!(dp.is_playing);
            assert_eq!(dp.received_count, 1);
            assert_eq!(dp.handled_count, 1);

            let dp_2 = device_2.borrow();
            assert!(dp_2.is_playing);
            assert_eq!(dp_2.received_count, 1);
            assert_eq!(dp_2.handled_count, 1);
        }

        advance_to_next_beat(&mut clock, &mut sequencer);
        assert_eq!(clock.beats().floor(), 2.0);
        // assert_eq!(sequencer.tick_sequencer.events.len(), 1);
        {
            let dp = device_1.borrow();
            assert!(!dp.is_playing);
            assert_eq!(dp.received_count, 2);
            assert_eq!(dp.handled_count, 2);

            let dp_2 = device_2.borrow();
            assert!(dp_2.is_playing);
            assert_eq!(dp_2.received_count, 1);
            assert_eq!(dp_2.handled_count, 1);
        }

        advance_to_next_beat(&mut clock, &mut sequencer);
        assert_eq!(clock.beats().floor(), 3.0);
        // assert_eq!(sequencer.tick_sequencer.events.len(), 0);
        {
            let dp = device_1.borrow();
            assert!(!dp.is_playing);
            assert_eq!(dp.received_count, 2);
            assert_eq!(dp.handled_count, 2);

            let dp_2 = device_2.borrow();
            assert!(!dp_2.is_playing);
            assert_eq!(dp_2.received_count, 2);
            assert_eq!(dp_2.handled_count, 2);
        }
    }
}
