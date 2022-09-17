use std::{cell::RefCell, collections::HashMap, rc::Rc};

use sorted_vec::SortedVec;

use crate::{
    common::OrderedMidiMessage,
    primitives::clock::{Clock, TimeSignature},
};

use super::traits::{MidiSink, MidiSource, TimeSlice};

pub struct Sequencer {
    midi_ticks_per_second: u32,
    beats_per_minute: f32,
    time_signature: TimeSignature,

    channels_to_sink_vecs: HashMap<u8, Vec<Rc<RefCell<dyn MidiSink>>>>,
    midi_messages: SortedVec<OrderedMidiMessage>,
}

impl Sequencer {
    pub fn new() -> Self {
        let mut result = Self {
            midi_ticks_per_second: 960,
            beats_per_minute: 120.0,
            time_signature: TimeSignature::new(4, 4),
            channels_to_sink_vecs: HashMap::new(),
            midi_messages: SortedVec::new(),
        };
        for channel in 0..Self::connected_channel_count() {
            result.channels_to_sink_vecs.insert(channel, Vec::new());
        }
        result
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

    pub fn set_midi_ticks_per_second(&mut self, tps: u32) {
        self.midi_ticks_per_second = tps;
    }

    pub fn add_message(&mut self, message: OrderedMidiMessage) {
        self.midi_messages.insert(message);
    }

    pub fn connect_midi_sink_for_channel(
        &mut self,
        device: Rc<RefCell<dyn MidiSink>>,
        channel: u8,
    ) {
        // https://users.rust-lang.org/t/lots-of-references-when-using-hashmap/68754
        // discusses why we have to do strange &u keys.
        let sink_vec = self.channels_to_sink_vecs.get_mut(&channel).unwrap();
        sink_vec.push(device);
    }

    fn dispatch_midi_message(&self, midi_message: &OrderedMidiMessage, clock: &Clock) {
        let sinks = self
            .channels_to_sink_vecs
            .get(&midi_message.message.channel)
            .unwrap();
        for sink in sinks {
            sink.borrow_mut()
                .handle_midi_message(&midi_message.message, clock);
        }
    }
}

impl MidiSource for Sequencer {
    // TODO: should this always require a channel? Or does the channel-less version mean sink all events?
    // fn connect_midi_sink(&mut self, device: Rc<RefCell<dyn DeviceTrait>>) {
    //     self.sinks[&0].push(device);
    // }
}

impl TimeSlice for Sequencer {
    fn tick(&mut self, clock: &Clock) -> bool {
        if self.midi_messages.is_empty() {
            // This is different from falling through the loop below because
            // it signals that we're done.
            return true;
        }
        let elapsed_midi_ticks = (clock.seconds * self.midi_ticks_per_second as f32) as u32;
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
        false
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use crate::{
        common::{MidiMessage, MidiNote, OrderedMidiMessage},
        devices::{tests::NullDevice, traits::TimeSlice},
        primitives::clock::Clock,
        settings::ClockSettings,
    };

    use super::Sequencer;

    impl Sequencer {
        pub(crate) fn tick_for_beat(&self, clock: &Clock, beat: u32) -> u32 {
            let tpb = self.midi_ticks_per_second as f32 / (clock.settings().bpm() / 60.0);
            (tpb * beat as f32) as u32
        }
    }

    fn advance_to_next_beat(clock: &mut Clock, sequencer: &mut Sequencer) {
        let next_beat = clock.beats.floor() + 1.0;
        while clock.beats < next_beat {
            clock.tick();
            sequencer.tick(&clock);
        }
        let _d = true;
    }

    #[test]
    fn test_sequencer() {
        let mut clock = Clock::new(&ClockSettings::new_defaults());
        let mut sequencer = Sequencer::new();

        let device = Rc::new(RefCell::new(NullDevice::new()));
        assert!(!device.borrow().is_playing);

        sequencer.add_message(OrderedMidiMessage {
            when: sequencer.tick_for_beat(&clock, 0),
            message: MidiMessage::note_on_c4(),
        });
        sequencer.add_message(OrderedMidiMessage {
            when: sequencer.tick_for_beat(&clock, 1),
            message: MidiMessage::note_off_c4(),
        });

        sequencer.connect_midi_sink_for_channel(device.clone(), 0);

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
        let mut clock = Clock::new(&ClockSettings::new_defaults());
        let mut sequencer = Sequencer::new();

        let device_1 = Rc::new(RefCell::new(NullDevice::new()));
        assert!(!device_1.borrow().is_playing);
        device_1.borrow_mut().set_channel(0);
        sequencer.connect_midi_sink_for_channel(device_1.clone(), 0);

        let device_2 = Rc::new(RefCell::new(NullDevice::new()));
        assert!(!device_2.borrow().is_playing);
        device_2.borrow_mut().set_channel(1);
        sequencer.connect_midi_sink_for_channel(device_2.clone(), 1);

        sequencer.add_message(OrderedMidiMessage {
            when: sequencer.tick_for_beat(&clock, 0),
            message: MidiMessage::new_note_on(0, 60, 0),
        });
        sequencer.add_message(OrderedMidiMessage {
            when: sequencer.tick_for_beat(&clock, 1),
            message: MidiMessage::new_note_on(1, 60, 0),
        });
        sequencer.add_message(OrderedMidiMessage {
            when: sequencer.tick_for_beat(&clock, 2),
            message: MidiMessage::new_note_off(0, MidiNote::C4 as u8, 0),
        });
        sequencer.add_message(OrderedMidiMessage {
            when: sequencer.tick_for_beat(&clock, 3),
            message: MidiMessage::new_note_off(1, MidiNote::C4 as u8, 0),
        });

        // TODO: this tick() doesn't match the Clock tick() in the sense that the clock is in the right state
        // right after init (without tick()), but the sequencer isn't (needs tick()). Maybe they shouldn't both
        // be called tick().
        assert_eq!(sequencer.midi_messages.len(), 4);
        sequencer.tick(&clock);
        assert_eq!(clock.beats, 0.0);
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
        assert_eq!(clock.beats.floor(), 1.0); // TODO: these floor() calls are a smell
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
        assert_eq!(clock.beats.floor(), 2.0);
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
        assert_eq!(clock.beats.floor(), 3.0);
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
