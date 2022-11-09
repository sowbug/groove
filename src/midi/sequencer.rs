use crate::{
    clock::{Clock, MidiTicks},
    common::Ww,
    midi::MidiChannel,
    patterns::MidiTickSequencer,
    traits::{SinksMidi, SourcesMidi, Terminates, WatchesClock},
};
use midly::MidiMessage;
use std::collections::HashMap;

#[derive(Debug)]
pub struct MidiSequencer {
    midi_ticks_per_second: MidiTicks,
    tick_sequencer: MidiTickSequencer,
}

impl Default for MidiSequencer {
    fn default() -> Self {
        Self {
            midi_ticks_per_second: MidiTicks(960),
            tick_sequencer: MidiTickSequencer::default(),
        }
    }
}

impl MidiSequencer {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn connected_channel_count() -> u8 {
        16
    }

    pub(crate) fn set_midi_ticks_per_second(&mut self, tps: usize) {
        self.midi_ticks_per_second = MidiTicks(tps);
    }

    pub(crate) fn insert(&mut self, when: MidiTicks, channel: MidiChannel, message: MidiMessage) {
        self.tick_sequencer.insert(when, channel, message);
    }

    #[allow(dead_code)]
    pub(crate) fn clear(&mut self) {
        self.tick_sequencer.clear();
    }
}

impl SourcesMidi for MidiSequencer {
    fn midi_sinks_mut(&mut self) -> &mut HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>> {
        self.tick_sequencer.midi_sinks_mut()
    }

    fn midi_sinks(&self) -> &HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>> {
        self.tick_sequencer.midi_sinks()
    }

    fn midi_output_channel(&self) -> MidiChannel {
        self.tick_sequencer.midi_output_channel()
    }

    fn set_midi_output_channel(&mut self, midi_channel: MidiChannel) {
        self.tick_sequencer.set_midi_output_channel(midi_channel);
    }
}

impl WatchesClock for MidiSequencer {
    fn tick(&mut self, clock: &Clock) {
        self.tick_sequencer.tick(clock);
    }
}

impl Terminates for MidiSequencer {
    fn is_finished(&self) -> bool {
        self.tick_sequencer.is_finished()
    }
}

#[cfg(test)]
mod tests {

    use super::MidiSequencer;
    use crate::{
        clock::{Clock, MidiTicks},
        common::{rrc, rrc_downgrade},
        midi::{MidiNote, MidiUtils},
        traits::{SinksMidi, SourcesMidi, WatchesClock},
        utils::tests::TestMidiSink,
    };

    impl MidiSequencer {
        pub(crate) fn tick_for_beat(&self, clock: &Clock, beat: usize) -> MidiTicks {
            let tpb = self.midi_ticks_per_second.0 as f32 / (clock.settings().bpm() / 60.0);
            MidiTicks::from(tpb * beat as f32)
        }
    }

    fn advance_to_next_beat(clock: &mut Clock, sequencer: &mut MidiSequencer) {
        let next_beat = clock.beats().floor() + 1.0;
        while clock.beats() < next_beat {
            clock.tick();
            sequencer.tick(clock);
        }
    }

    // We're papering over the issue that MIDI events are firing a little late.
    // See Clock::next_slice_in_midi_ticks().
    fn advance_one_midi_tick(clock: &mut Clock, sequencer: &mut MidiSequencer) {
        let next_midi_tick = clock.midi_ticks() + 1;
        while clock.midi_ticks() < next_midi_tick {
            clock.tick();
            sequencer.tick(clock);
        }
    }

    #[test]
    fn test_sequencer() {
        let mut clock = Clock::new();
        let mut sequencer = MidiSequencer::new();

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

        let sink = rrc_downgrade(&device);
        sequencer.add_midi_sink(0, sink);

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
        assert_eq!(sequencer.tick_sequencer.debug_events().len(), 4);

        dbg!(&sequencer.tick_sequencer.debug_events());

        // Let the tick #0 event(s) fire.
        assert_eq!(clock.samples(), 0);
        assert_eq!(clock.midi_ticks(), 0);
        advance_one_midi_tick(&mut clock, &mut sequencer);
        dbg!(&device_1.borrow().messages);
        dbg!(&device_2.borrow().messages);
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
        dbg!(&device_1.borrow().messages);
        dbg!(&device_2.borrow().messages);
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
