use crate::{
    clock::{Clock, MidiTicks, PerfectTimeUnit},
    messages::EntityMessage,
    messages::MessageBounds,
    midi::{MidiChannel, MidiMessage},
    traits::{HasUid, IsController, Response, Terminates, Updateable},
};
use btreemultimap::BTreeMultiMap;
use midly::num::u7;
use rustc_hash::FxHashMap;
use std::{
    fmt::Debug,
    marker::PhantomData,
    ops::Bound::{Excluded, Included},
};

pub(crate) type BeatEventsMap = BTreeMultiMap<PerfectTimeUnit, (MidiChannel, MidiMessage)>;

#[derive(Debug, Default)]
pub struct BeatSequencer<M: MessageBounds> {
    uid: usize,
    next_instant: PerfectTimeUnit,
    events: BeatEventsMap,
    last_event_time: PerfectTimeUnit,
    is_disabled: bool,

    should_stop_pending_notes: bool,
    on_notes: FxHashMap<u7, MidiChannel>,

    _phantom: PhantomData<M>,
}
impl<M: MessageBounds> IsController for BeatSequencer<M> {}
impl<M: MessageBounds> Updateable for BeatSequencer<M> {
    default type Message = M;

    default fn update(
        &mut self,
        _clock: &Clock,
        _message: Self::Message,
    ) -> Response<Self::Message> {
        Response::none()
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

impl<M: MessageBounds> BeatSequencer<M> {
    #[allow(dead_code)]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn clear(&mut self) {
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

    pub fn is_enabled(&self) -> bool {
        !self.is_disabled
    }

    pub fn enable(&mut self, is_enabled: bool) {
        if !self.is_disabled && !is_enabled {
            self.should_stop_pending_notes = true;
        }
        self.is_disabled = !is_enabled;
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

    fn stop_pending_notes(&mut self) -> Response<EntityMessage> {
        let mut v = Vec::new();
        for on_note in &self.on_notes {
            v.push(Response::single(EntityMessage::Midi(
                *on_note.1,
                MidiMessage::NoteOff {
                    key: *on_note.0,
                    vel: 0.into(),
                },
            )));
        }
        Response::batch(v)
    }
}

impl Updateable for BeatSequencer<EntityMessage> {
    type Message = EntityMessage;

    fn update(&mut self, clock: &Clock, message: Self::Message) -> Response<Self::Message> {
        match message {
            Self::Message::Tick => {
                self.next_instant = PerfectTimeUnit(clock.next_slice_in_beats());

                if self.should_stop_pending_notes {
                    self.should_stop_pending_notes = false;
                    return self.stop_pending_notes();
                }

                return if self.is_enabled() {
                    // If the last instant marks a new interval, then we want to include
                    // any events scheduled at exactly that time. So the range is
                    // inclusive.
                    let range = (
                        Included(PerfectTimeUnit(clock.beats())),
                        Excluded(self.next_instant),
                    );
                    Response::batch(self.events.range(range).into_iter().fold(
                        Vec::new(),
                        |mut vec, (_when, event)| {
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
                            vec.push(Response::single(Self::Message::Midi(event.0, event.1)));
                            vec
                        },
                    ))
                } else {
                    Response::none()
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
    next_instant: MidiTicks,
    events: MidiTickEventsMap,
    last_event_time: MidiTicks,
    is_disabled: bool,
    _phantom: PhantomData<M>,
}
impl<M: MessageBounds> IsController for MidiTickSequencer<M> {}
impl<M: MessageBounds> Updateable for MidiTickSequencer<M> {
    default type Message = M;

    default fn update(
        &mut self,
        _clock: &Clock,
        _message: Self::Message,
    ) -> Response<Self::Message> {
        Response::none()
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
            next_instant: MidiTicks::MIN,
            events: Default::default(),
            last_event_time: MidiTicks::MIN,
            is_disabled: Default::default(),
            _phantom: Default::default(),
        }
    }
}

impl<M: MessageBounds> MidiTickSequencer<M> {
    #[allow(dead_code)]
    pub(crate) fn new() -> Self {
        Self::default()
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

    pub fn is_enabled(&self) -> bool {
        !self.is_disabled
    }

    #[allow(dead_code)]
    pub fn enable(&mut self, is_enabled: bool) {
        self.is_disabled = !is_enabled;
    }
}

impl Updateable for MidiTickSequencer<EntityMessage> {
    type Message = EntityMessage;

    fn update(&mut self, clock: &Clock, message: Self::Message) -> Response<Self::Message> {
        match message {
            Self::Message::Tick => {
                self.next_instant = MidiTicks(clock.next_slice_in_midi_ticks());

                if self.is_enabled() {
                    // If the last instant marks a new interval, then we want to include
                    // any events scheduled at exactly that time. So the range is
                    // inclusive.
                    let range = (
                        Included(MidiTicks(clock.midi_ticks())),
                        Excluded(self.next_instant),
                    );
                    let events = self.events.range(range);
                    Response::batch(events.into_iter().fold(
                        Vec::new(),
                        |mut vec: Vec<Response<Self::Message>>, (_when, (channel, message))| {
                            vec.push(Response::single(Self::Message::Midi(*channel, *message)));
                            vec
                        },
                    ))
                } else {
                    Response::none()
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
        clock::{Clock, MidiTicks},
        entities::BoxedEntity,
        messages::EntityMessage,
        messages::{tests::TestMessage, MessageBounds},
        midi::{MidiChannel, MidiUtils},
        traits::{IsController, TestInstrument},
        Orchestrator,
    };

    impl<M: MessageBounds> BeatSequencer<M> {
        pub fn debug_events(&self) -> &BeatEventsMap {
            &self.events
        }

        #[allow(dead_code)]
        pub fn debug_dump_events(&self) {
            println!("{:?}", self.events);
        }
    }

    impl<M: MessageBounds> MidiTickSequencer<M> {
        #[allow(dead_code)]
        pub(crate) fn debug_events(&self) -> &MidiTickEventsMap {
            &self.events
        }
    }

    impl<M: MessageBounds> MidiTickSequencer<M> {
        pub(crate) fn tick_for_beat(&self, clock: &Clock, beat: usize) -> MidiTicks {
            //            let tpb = self.midi_ticks_per_second.0 as f32 /
            //            (clock.bpm() / 60.0);
            let tpb = 960.0 / (clock.bpm() / 60.0); // TODO: who should own the number of ticks/second?
            MidiTicks::from(tpb * beat as f32)
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
            let _ = sequencer.update(clock, EntityMessage::Tick);
            clock.tick();
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
            let _ = sequencer.update(clock, EntityMessage::Tick);
            clock.tick();
        }
    }

    #[test]
    fn test_sequencer() {
        const DEVICE_MIDI_CHANNEL: MidiChannel = 7;
        let mut clock = Clock::default();
        let mut o = Orchestrator::<TestMessage>::default();
        let mut sequencer = Box::new(MidiTickSequencer::<EntityMessage>::default());
        let instrument = Box::new(TestInstrument::<EntityMessage>::default());
        let device_uid = o.add(None, BoxedEntity::TestInstrument(instrument));

        sequencer.insert(
            sequencer.tick_for_beat(&clock, 0),
            DEVICE_MIDI_CHANNEL,
            MidiUtils::note_on_c4(),
        );
        sequencer.insert(
            sequencer.tick_for_beat(&clock, 1),
            DEVICE_MIDI_CHANNEL,
            MidiUtils::note_off_c4(),
        );
        const SEQUENCER_ID: &str = "seq";
        let _sequencer_uid = o.add(
            Some(SEQUENCER_ID),
            BoxedEntity::MidiTickSequencer(sequencer),
        );
        o.connect_midi_downstream(device_uid, DEVICE_MIDI_CHANNEL);

        // TODO: figure out a reasonable way to test these things once they're
        // inside Store, and their type information has been erased. Maybe we
        // can send messages asking for state. Maybe we can send things that the
        // entities themselves assert.
        if let Some(entity) = o.get_mut(SEQUENCER_ID) {
            if let Some(sequencer) = entity.as_is_controller_mut() {
                advance_one_midi_tick(&mut clock, sequencer);
                {
                    // assert!(instrument.is_playing);
                    // assert_eq!(instrument.received_count, 1);
                    // assert_eq!(instrument.handled_count, 1);
                }
            }
        }

        if let Some(entity) = o.get_mut(SEQUENCER_ID) {
            if let Some(sequencer) = entity.as_is_controller_mut() {
                advance_to_next_beat(&mut clock, sequencer);
                {
                    // assert!(!instrument.is_playing);
                    // assert_eq!(instrument.received_count, 2);
                    // assert_eq!(&instrument.handled_count, &2);
                }
            }
        }
    }

    // TODO: re-enable later.......................................................................
    // #[test]
    // fn test_sequencer_multichannel() {
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
    //         MidiUtils::new_note_on(60, 0),
    //     );
    //     sequencer.insert(
    //         sequencer.tick_for_beat(&clock, 1),
    //         1,
    //         MidiUtils::new_note_on(60, 0),
    //     );
    //     sequencer.insert(
    //         sequencer.tick_for_beat(&clock, 2),
    //         0,
    //         MidiUtils::new_note_off(MidiNote::C4 as u8, 0),
    //     );
    //     sequencer.insert(
    //         sequencer.tick_for_beat(&clock, 3),
    //         1,
    //         MidiUtils::new_note_off(MidiNote::C4 as u8, 0),
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
