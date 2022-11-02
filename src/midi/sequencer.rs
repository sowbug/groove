use super::MIDI_CHANNEL_RECEIVE_ALL;
use crate::{
    clock::{Clock, TimeSignature},
    common::Ww,
    midi::MidiChannel,
    patterns::OrderedEvent,
    traits::{SinksMidi, SourcesMidi, Terminates, WatchesClock},
};
use sorted_vec::SortedVec;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct MidiSequencer {
    midi_ticks_per_second: usize,
    beats_per_minute: f32,
    time_signature: TimeSignature,

    // TODO: if this gets too unwieldy, consider https://crates.io/crates/multimap
    channels_to_sink_vecs: HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>>,

    midi_messages: SortedVec<OrderedEvent<usize>>,
}

impl MidiSequencer {
    pub fn new() -> Self {
        Self {
            midi_ticks_per_second: 960,
            beats_per_minute: 120.0,
            ..Default::default()
        }
    }

    pub fn connected_channel_count() -> u8 {
        16
    }

    #[allow(dead_code)]
    pub fn set_tempo(&mut self, beats_per_minute: f32) {
        self.beats_per_minute = beats_per_minute;
    }

    #[allow(dead_code)]
    pub fn set_time_signature(&mut self, time_signature: TimeSignature) {
        self.time_signature = time_signature;
    }

    pub fn set_midi_ticks_per_second(&mut self, tps: usize) {
        self.midi_ticks_per_second = tps;
    }

    pub fn add_message(&mut self, message: OrderedEvent<usize>) {
        self.midi_messages.insert(message);
    }

    fn dispatch_midi_message(&self, message: &OrderedEvent<usize>, clock: &Clock) {
        self.issue_midi(clock, &message.channel, &message.event);
    }
}

impl SourcesMidi for MidiSequencer {
    fn midi_sinks_mut(&mut self) -> &mut HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>> {
        &mut self.channels_to_sink_vecs
    }

    fn midi_sinks(&self) -> &HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>> {
        &self.channels_to_sink_vecs
    }

    fn midi_output_channel(&self) -> MidiChannel {
        MIDI_CHANNEL_RECEIVE_ALL // TODO: this is poorly named
    }

    fn set_midi_output_channel(&mut self, _midi_channel: MidiChannel) {}
}

impl WatchesClock for MidiSequencer {
    fn tick(&mut self, clock: &Clock) {
        if self.midi_messages.is_empty() {
            // This is different from falling through the loop below because
            // it signals that we're done.
            return;
        }
        let elapsed_midi_ticks = (clock.seconds() * self.midi_ticks_per_second as f32) as usize;
        while !self.midi_messages.is_empty() {
            let midi_message = self.midi_messages.first().unwrap();

            // TODO(miket): should Clock manage elapsed_midi_ticks?
            if elapsed_midi_ticks >= midi_message.when {
                self.dispatch_midi_message(midi_message, clock);

                // TODO: this is violating a (future) rule that we can always randomly access
                // anything in the song. It's actually more than that, because it's destroying
                // information that would be needed to add that ability later.
                self.midi_messages.remove_index(0);
            } else {
                break;
            }
        }
    }
}

impl Terminates for MidiSequencer {
    fn is_finished(&self) -> bool {
        self.midi_messages.is_empty()
    }
}
#[cfg(test)]
mod tests {

    use super::MidiSequencer;
    use crate::{
        clock::Clock,
        common::{rrc, rrc_downgrade},
        midi::{MidiNote, MidiUtils},
        patterns::OrderedEvent,
        traits::{SinksMidi, SourcesMidi, WatchesClock},
        utils::tests::TestMidiSink,
    };

    impl MidiSequencer {
        pub(crate) fn tick_for_beat(&self, clock: &Clock, beat: usize) -> usize {
            let tpb = self.midi_ticks_per_second as f32 / (clock.settings().bpm() / 60.0);
            (tpb * beat as f32) as usize
        }
    }

    fn advance_to_next_beat(clock: &mut Clock, sequencer: &mut MidiSequencer) {
        let next_beat = clock.beats().floor() + 1.0;
        while clock.beats() < next_beat {
            clock.tick();
            sequencer.tick(clock);
        }
        let _d = true;
    }

    #[test]
    fn test_sequencer() {
        let mut clock = Clock::new();
        let mut sequencer = MidiSequencer::new();

        let device = rrc(TestMidiSink::new_with(0));
        assert!(!device.borrow().is_playing);

        // These helpers create messages on channel zero.
        sequencer.add_message(OrderedEvent {
            when: sequencer.tick_for_beat(&clock, 0),
            channel: 0,
            event: MidiUtils::note_on_c4(),
        });
        sequencer.add_message(OrderedEvent {
            when: sequencer.tick_for_beat(&clock, 1),
            channel: 0,
            event: MidiUtils::note_off_c4(),
        });

        let sink = rrc_downgrade(&device);
        sequencer.add_midi_sink(0, sink);

        sequencer.tick(&clock);
        {
            let dp = device.borrow();
            assert!(dp.is_playing);
            assert_eq!(dp.midi_messages_received, 1);
            assert_eq!(dp.midi_messages_handled, 1);
        }

        advance_to_next_beat(&mut clock, &mut sequencer);
        {
            let dp = device.borrow();
            assert!(!dp.is_playing);
            assert_eq!(dp.midi_messages_received, 2);
            assert_eq!(dp.midi_messages_handled, 2);
        }
    }

    #[test]
    fn test_sequencer_multichannel() {
        let mut clock = Clock::new();
        let mut sequencer = MidiSequencer::new();

        let device_1 = rrc(TestMidiSink::new());
        assert!(!device_1.borrow().is_playing);
        device_1.borrow_mut().set_midi_channel(0);
        let sink = rrc_downgrade(&device_1);
        sequencer.add_midi_sink(0, sink);

        let device_2 = rrc(TestMidiSink::new());
        assert!(!device_2.borrow().is_playing);
        device_2.borrow_mut().set_midi_channel(1);
        let sink = rrc_downgrade(&device_2);
        sequencer.add_midi_sink(1, sink);

        sequencer.add_message(OrderedEvent {
            when: sequencer.tick_for_beat(&clock, 0),
            channel: 0,
            event: MidiUtils::new_note_on2(60, 0),
        });
        sequencer.add_message(OrderedEvent {
            when: sequencer.tick_for_beat(&clock, 1),
            channel: 1,
            event: MidiUtils::new_note_on2(60, 0),
        });
        sequencer.add_message(OrderedEvent {
            when: sequencer.tick_for_beat(&clock, 2),
            channel: 0,
            event: MidiUtils::new_note_off2(MidiNote::C4 as u8, 0),
        });
        sequencer.add_message(OrderedEvent {
            when: sequencer.tick_for_beat(&clock, 3),
            channel: 1,
            event: MidiUtils::new_note_off2(MidiNote::C4 as u8, 0),
        });

        // TODO: this tick() doesn't match the Clock tick() in the sense that the clock is in the right state
        // right after init (without tick()), but the sequencer isn't (needs tick()). Maybe they shouldn't both
        // be called tick().
        assert_eq!(sequencer.midi_messages.len(), 4);
        sequencer.tick(&clock);
        assert_eq!(clock.beats(), 0.0);
        assert_eq!(sequencer.midi_messages.len(), 3);
        {
            let dp_1 = device_1.borrow();
            assert!(dp_1.is_playing);
            assert_eq!(dp_1.midi_messages_received, 1);
            assert_eq!(dp_1.midi_messages_handled, 1);

            let dp_2 = device_2.borrow();
            assert!(!dp_2.is_playing);
            assert_eq!(dp_2.midi_messages_received, 0);
            assert_eq!(dp_2.midi_messages_handled, 0);
        }

        advance_to_next_beat(&mut clock, &mut sequencer);
        assert_eq!(clock.beats().floor(), 1.0); // TODO: these floor() calls are a smell
        assert_eq!(sequencer.midi_messages.len(), 2);
        {
            let dp = device_1.borrow();
            assert!(dp.is_playing);
            assert_eq!(dp.midi_messages_received, 1);
            assert_eq!(dp.midi_messages_handled, 1);

            let dp_2 = device_2.borrow();
            assert!(dp_2.is_playing);
            assert_eq!(dp_2.midi_messages_received, 1);
            assert_eq!(dp_2.midi_messages_handled, 1);
        }

        advance_to_next_beat(&mut clock, &mut sequencer);
        assert_eq!(clock.beats().floor(), 2.0);
        assert_eq!(sequencer.midi_messages.len(), 1);
        {
            let dp = device_1.borrow();
            assert!(!dp.is_playing);
            assert_eq!(dp.midi_messages_received, 2);
            assert_eq!(dp.midi_messages_handled, 2);

            let dp_2 = device_2.borrow();
            assert!(dp_2.is_playing);
            assert_eq!(dp_2.midi_messages_received, 1);
            assert_eq!(dp_2.midi_messages_handled, 1);
        }

        advance_to_next_beat(&mut clock, &mut sequencer);
        assert_eq!(clock.beats().floor(), 3.0);
        assert_eq!(sequencer.midi_messages.len(), 0);
        {
            let dp = device_1.borrow();
            assert!(!dp.is_playing);
            assert_eq!(dp.midi_messages_received, 2);
            assert_eq!(dp.midi_messages_handled, 2);

            let dp_2 = device_2.borrow();
            assert!(!dp_2.is_playing);
            assert_eq!(dp_2.midi_messages_received, 2);
            assert_eq!(dp_2.midi_messages_handled, 2);
        }
    }
}
