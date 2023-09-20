// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::entity_factory::EntityStore;
use anyhow::anyhow;
use groove_core::{
    midi::{MidiChannel, MidiMessage},
    Uid,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct MidiRouter {
    /// MIDI connections
    midi_channel_to_receiver_uid: HashMap<MidiChannel, Vec<Uid>>,
}
impl MidiRouter {
    /// The entities receiving on the given MIDI channel.
    pub fn receivers(&mut self, channel: &MidiChannel) -> &Vec<Uid> {
        self.midi_channel_to_receiver_uid
            .entry(*channel)
            .or_default()
    }

    /// Connect an entity to the given MIDI channel.
    pub fn connect(&mut self, receiver_uid: Uid, channel: MidiChannel) {
        self.midi_channel_to_receiver_uid
            .entry(channel)
            .or_default()
            .push(receiver_uid);
    }

    /// Disconnect an entity from the given MIDI channel.
    #[allow(dead_code)]
    pub fn disconnect(&mut self, receiver_uid: Uid, channel: MidiChannel) {
        self.midi_channel_to_receiver_uid
            .entry(channel)
            .or_default()
            .retain(|&uid| uid != receiver_uid);
    }

    /// Route the [MidiMessage] to everyone listening on the [MidiChannel],
    /// using the provided closure to map [Uid] to [HandlesMidi]. Also routes
    /// all the [MidiMessage]s that are produced in response.
    //
    // TODO: I think this is incomplete. If an external sequencer drives an
    // internal arpeggiator that drives an external instrument, then I don't
    // think the arp's MIDI gets back to the outside world.
    pub fn route(
        &mut self,
        entity_store: &mut EntityStore,
        channel: MidiChannel,
        message: MidiMessage,
    ) -> anyhow::Result<()> {
        let mut loop_detected = false;
        let mut v = Vec::default();
        v.push((channel, message));
        while let Some((channel, message)) = v.pop() {
            let receiver_uids = self.receivers(&channel);
            receiver_uids.iter().for_each(|uid| {
                if let Some(e) = entity_store.get_mut(uid) {
                    if let Some(e) = e.as_handles_midi_mut() {
                        e.handle_midi_message(channel, message, &mut | response_channel, response_message| {
                            if channel != response_channel {
                                v.push((response_channel, response_message));
                            } else if !loop_detected {
                                loop_detected = true;
                                eprintln!("Warning: loop detected; while sending to channel {channel}, received request to send {:#?} to same channel", &response_message);
                            }
                        });
                    } else {
                        eprintln!("Warning: somehow device id {uid} that doesn't handle MIDI got on a receiver list");
                    }
                } else {
                    eprintln!("Warning: a receiver list refers to nonexistent entity id {uid}");
                }
            });
        }
        if loop_detected {
            Err(anyhow!("Device attempted to send MIDI message to itself"))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MidiRouter;
    use crate::mini::entity_factory::EntityStore;
    use ensnare::prelude::*;
    use groove_core::{
        midi,
        midi::{MidiChannel, MidiMessage, MidiMessagesFn},
        traits::{gui::Displays, Configurable, Generates, HandlesMidi, Serializable, Ticks},
        Uid,
    };
    use groove_proc_macros::{Control, IsInstrument, Uid};
    use serde::{Deserialize, Serialize};
    use std::sync::{Arc, RwLock};

    #[derive(Debug, Control, Default, IsInstrument, Uid, Serialize, Deserialize)]
    struct TestHandlesMidi {
        uid: Uid,

        rebroadcast_to: Option<MidiChannel>,

        #[serde(skip)]
        tracker: Arc<RwLock<Vec<(Uid, MidiChannel, MidiMessage)>>>,
    }
    impl TestHandlesMidi {
        fn new_with(
            uid: Uid,
            rebroadcast_to: Option<MidiChannel>,
            tracker: Arc<RwLock<Vec<(Uid, MidiChannel, MidiMessage)>>>,
        ) -> Self {
            Self {
                uid,
                rebroadcast_to,
                tracker,
            }
        }
    }
    impl HandlesMidi for TestHandlesMidi {
        fn handle_midi_message(
            &mut self,
            channel: MidiChannel,
            message: MidiMessage,
            midi_messages_fn: &mut MidiMessagesFn,
        ) {
            if let Ok(mut tracker) = self.tracker.write() {
                tracker.push((self.uid, channel, message))
            };
            if let Some(rebroadcast_channel) = self.rebroadcast_to {
                midi_messages_fn(rebroadcast_channel, message);
            }
        }
    }
    impl Serializable for TestHandlesMidi {}
    impl Configurable for TestHandlesMidi {}
    impl Displays for TestHandlesMidi {}
    impl Generates<StereoSample> for TestHandlesMidi {
        fn value(&self) -> StereoSample {
            todo!()
        }

        #[allow(unused_variables)]
        fn generate_batch_values(&mut self, values: &mut [StereoSample]) {
            todo!()
        }
    }
    impl Ticks for TestHandlesMidi {
        #[allow(unused_variables)]
        fn tick(&mut self, tick_count: usize) {
            todo!()
        }
    }

    #[test]
    fn routes_to_correct_channels() {
        let tracker = Arc::new(RwLock::new(Vec::default()));
        let mut es = EntityStore::default();
        let entity = Box::new(TestHandlesMidi::new_with(
            Uid(1),
            None,
            Arc::clone(&tracker),
        ));
        let _ = es.add(entity);
        let entity = Box::new(TestHandlesMidi::new_with(
            Uid(2),
            None,
            Arc::clone(&tracker),
        ));
        let _ = es.add(entity);

        let mut r = MidiRouter::default();
        r.connect(Uid(1), MidiChannel(1));
        r.connect(Uid(2), MidiChannel(2));

        let m = midi::new_note_on(1, 1);

        assert!(r.route(&mut es, MidiChannel(99), m).is_ok());
        if let Ok(t) = tracker.read() {
            assert!(
                t.is_empty(),
                "no messages received after routing to nonexistent MIDI channel"
            );
        }
        assert!(r.route(&mut es, MidiChannel(1), m).is_ok());
        if let Ok(t) = tracker.read() {
            assert_eq!(
                t.len(),
                1,
                "after routing to channel 1, only one listener should receive"
            );
            assert_eq!(
                t[0],
                (Uid(1), MidiChannel(1), m),
                "after routing to channel 1, only channel 1 listener should receive"
            );
        };
        assert!(r.route(&mut es, MidiChannel(2), m).is_ok());
        if let Ok(t) = tracker.read() {
            assert_eq!(
                t.len(),
                2,
                "after routing to channel 2, only one listener should receive"
            );
            assert_eq!(
                t[1],
                (Uid(2), MidiChannel(2), m),
                "after routing to channel 2, only channel 2 listener should receive"
            );
        };
    }

    #[test]
    fn also_routes_produced_messages() {
        let tracker = Arc::new(RwLock::new(Vec::default()));
        let mut es = EntityStore::default();
        let entity = Box::new(TestHandlesMidi::new_with(
            Uid(1),
            Some(MidiChannel(2)),
            Arc::clone(&tracker),
        ));
        let _ = es.add(entity);
        let entity = Box::new(TestHandlesMidi::new_with(
            Uid(2),
            None,
            Arc::clone(&tracker),
        ));
        let _ = es.add(entity);

        let mut r = MidiRouter::default();
        r.connect(Uid(1), MidiChannel(1));
        r.connect(Uid(2), MidiChannel(2));

        let m = midi::new_note_on(1, 1);

        assert!(r.route(&mut es, MidiChannel(1), m).is_ok());
        if let Ok(t) = tracker.read() {
            assert_eq!(
                t.len(),
                2,
                "routing to a producing receiver should produce and route a second message"
            );
            assert_eq!(
                t[0],
                (Uid(1), MidiChannel(1), m),
                "original message should be received"
            );
            assert_eq!(
                t[1],
                (Uid(2), MidiChannel(2), m),
                "produced message should be received"
            );
        };
        let m = midi::new_note_on(2, 3);
        assert!(r.route(&mut es, MidiChannel(2), m).is_ok());
        if let Ok(t) = tracker.read() {
            assert_eq!(
                t.len(),
                3,
                "routing to a non-producing receiver shouldn't produce anything"
            );
            assert_eq!(
                t[2],
                (Uid(2), MidiChannel(2), m),
                "after routing to channel 2, only channel 2 listener should receive"
            );
        };
    }

    #[test]
    fn detects_loops() {
        let tracker = Arc::new(RwLock::new(Vec::default()));
        let mut es = EntityStore::default();
        let entity = Box::new(TestHandlesMidi::new_with(
            Uid(1),
            Some(MidiChannel(1)),
            Arc::clone(&tracker),
        ));
        let _ = es.add(entity);

        let mut r = MidiRouter::default();
        r.connect(Uid(1), MidiChannel(1));

        let m = midi::new_note_on(1, 1);

        assert!(r.route(&mut es, MidiChannel(1), m).is_err());
    }
}
