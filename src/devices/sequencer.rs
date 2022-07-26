use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use sorted_vec::SortedVec;

use crate::{
    common::{MidiMessage, OrderedMidiMessage},
    primitives::clock::Clock,
};

use super::traits::DeviceTrait;

pub struct Sequencer {
    midi_ticks_per_second: u32,
    channels_to_sink_vecs: HashMap<u8, Vec<Rc<RefCell<dyn DeviceTrait>>>>,
    midi_messages: SortedVec<OrderedMidiMessage>,
}

impl Sequencer {
    pub fn new() -> Self {
        let mut result = Self {
            midi_ticks_per_second: 960,
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

    pub fn set_midi_ticks_per_second(&mut self, tps: u32) {
        self.midi_ticks_per_second = tps;
    }

    pub fn add_message(&mut self, message: OrderedMidiMessage) {
        self.midi_messages.insert(message);
    }

    pub fn connect_midi_sink_for_channel(
        &mut self,
        device: Rc<RefCell<dyn DeviceTrait>>,
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

    fn insert_short_note(&mut self, channel: u8, note: u8, when: &mut u32) {
        if note != 0 {
            self.add_message(OrderedMidiMessage {
                when: *when,
                message: MidiMessage::new_note_on(channel, note, 100),
            });
            self.add_message(OrderedMidiMessage {
                when: *when + 960 / 4, // TODO
                message: MidiMessage::new_note_off(channel, note, 100),
            });
        }
    }

    pub fn insert_pattern(
        &mut self,
        pattern: Rc<RefCell<Pattern>>,
        channel: u8,
        insertion_point: &mut u32,
    ) {
        let start_insertion_point: u32 = *insertion_point;
        for note_sequence in pattern.borrow().notes.clone() {
            *insertion_point = start_insertion_point;
            for note in note_sequence {
                self.insert_short_note(channel, note, insertion_point);
                *insertion_point += 960 / 4;
            }
        }
    }
}

impl DeviceTrait for Sequencer {
    fn sources_midi(&self) -> bool {
        true
    }

    fn tick(&mut self, clock: &Clock) -> bool {
        if self.midi_messages.is_empty() {
            return true;
        }
        let elapsed_midi_ticks = (clock.seconds * self.midi_ticks_per_second as f32) as u32;
        while !self.midi_messages.is_empty() {
            let midi_message = self.midi_messages.first().unwrap();

            // TODO(miket): should Clock manage elapsed_midi_ticks?
            if elapsed_midi_ticks >= midi_message.when {
                dbg!("dispatching {:?}", midi_message);
                self.dispatch_midi_message(midi_message, clock);
                self.midi_messages.remove_index(0);
            } else {
                break;
            }
        }
        false
    }

    // TODO: should this always require a channel? Or does the channel-less version mean sink all events?
    // fn connect_midi_sink(&mut self, device: Rc<RefCell<dyn DeviceTrait>>) {
    //     self.sinks[&0].push(device);
    // }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum BeatValue {
    WholeNote,
    HalfNote,
    QuarterNote,
    EighthNote,
    SixteenthNote,
    ThirtySecondNote,
    SixtyFourthNote,
    OneHundredTwentyEighthNote,
    TwoHundredFiftySixthNote,
    FiveHundredTwelfthNote,
}

#[derive(Clone)]
pub struct Pattern {
    pub beat_value: BeatValue,
    pub notes: Vec<Vec<u8>>,
}

impl Pattern {
    pub(crate) fn from_settings(settings: &crate::settings::PatternSettings) -> Self {
        let mut r = Self {
            beat_value: settings.beat_value.clone(),
            notes: Vec::new(),
        };
        for note_sequence in settings.notes.clone() {
            let mut note_vec = Vec::new();
            for note in note_sequence.clone() {
                note_vec.push(note);
            }
            r.notes.push(note_vec);
        }
        r
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use crate::{
        common::{MidiMessage, MidiNote, OrderedMidiMessage},
        devices::{tests::NullDevice, traits::DeviceTrait},
        primitives::clock::{Clock, ClockSettings},
        settings::PatternSettings,
    };

    use super::{BeatValue, Pattern, Sequencer};

    impl Sequencer {
        pub(crate) fn tick_for_beat(&self, clock: &Clock, beat: u32) -> u32 {
            let tpb = self.midi_ticks_per_second as f32 / (clock.settings().bpm() / 60.0);
            (tpb * beat as f32) as u32
        }
    }

    fn advance_one_beat(clock: &mut Clock, sequencer: &mut Sequencer) {
        let old_time = clock.seconds;
        let beat = clock.beats;
        while clock.beats == beat {
            clock.tick();
            sequencer.tick(&clock);
        }
        dbg!("Beat clock is now {} {}", beat, clock.beats);
        dbg!("Time clock is now {} {}", old_time, clock.seconds);
        let _d = true;
    }

    #[test]
    fn test_sequencer() {
        let mut clock = Clock::new(ClockSettings::new_defaults());
        let mut sequencer = Sequencer::new();
        assert!(sequencer.sources_midi());
        assert!(!sequencer.sources_audio());

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

        advance_one_beat(&mut clock, &mut sequencer);
        {
            let dp = device.borrow();
            assert!(!dp.is_playing);
            assert_eq!(dp.midi_messages_received, 2);
            assert_eq!(dp.midi_messages_handled, 2);
        }
    }

    #[test]
    fn test_sequencer_multichannel() {
        let mut clock = Clock::new(ClockSettings::new_defaults());
        let mut sequencer = Sequencer::new();
        assert!(sequencer.sources_midi());
        assert!(!sequencer.sources_audio());

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
        assert_eq!(clock.beats, 0);
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

        advance_one_beat(&mut clock, &mut sequencer);
        assert_eq!(clock.beats, 1);
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

        advance_one_beat(&mut clock, &mut sequencer);
        assert_eq!(clock.beats, 2);
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

        advance_one_beat(&mut clock, &mut sequencer);
        assert_eq!(clock.beats, 3);
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

    #[test]
    fn test_pattern() {
        let mut sequencer = Sequencer::new();
        let pattern_settings = PatternSettings {
            id: String::from("test-pattern"),
            beat_value: BeatValue::EighthNote,
            notes: vec![vec![1, 2, 3, 4, 5]],
        };

        // TODO: is there any way to avoid Rc/RefCell leaking into this class's API boundary?
        let pattern = Rc::new(RefCell::new(Pattern::from_settings(&pattern_settings)));

        const EXPECTED_NOTE_COUNT: usize = 5;
        assert_eq!(pattern.borrow().notes.len(), 1);
        assert_eq!(pattern.borrow().notes[0].len(), EXPECTED_NOTE_COUNT);

        let mut insertion_point: u32 = 0;
        sequencer.insert_pattern(pattern, 0, &mut insertion_point);

        assert_eq!(sequencer.midi_messages.len(), EXPECTED_NOTE_COUNT * 2); // one on, one off
    }
}
