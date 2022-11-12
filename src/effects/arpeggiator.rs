use midly::num::u7;

use crate::{
    clock::{Clock, PerfectTimeUnit},
    common::{rrc, rrc_downgrade, Rrc, Ww},
    control::BigMessage,
    midi::{sequencers::BeatSequencer, MidiChannel, MidiMessage},
    traits::{
        HasOverhead, IsMidiEffect, Overhead, SinksMidi, SourcesMidi, Terminates, WatchesClock,
    },
};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Arpeggiator {
    pub(crate) me: Ww<Self>,
    overhead: Overhead,
    midi_channel_in: MidiChannel,
    midi_channel_out: MidiChannel,
    beat_sequencer: BeatSequencer,

    is_device_playing: bool,
}

impl SinksMidi for Arpeggiator {
    fn midi_channel(&self) -> MidiChannel {
        self.midi_channel_in
    }

    fn handle_midi_for_channel(
        &mut self,
        clock: &Clock,
        _channel: &MidiChannel,
        message: &MidiMessage,
    ) {
        // TODO: we'll need clock to do cool things like schedule note change on
        // next bar... maybe
        #[allow(unused_variables)]
        match message {
            MidiMessage::NoteOff { key, vel } => self.is_device_playing = false,
            MidiMessage::NoteOn { key, vel } => {
                self.rebuild_sequence(clock, key.as_int(), vel.as_int());
                self.is_device_playing = true;
                //                self.sequence_start_beats = clock.beats();
            }
            MidiMessage::Aftertouch { key, vel } => todo!(),
            MidiMessage::Controller { controller, value } => todo!(),
            MidiMessage::ProgramChange { program } => todo!(),
            MidiMessage::ChannelAftertouch { vel } => todo!(),
            MidiMessage::PitchBend { bend } => todo!(),
        }
        self.beat_sequencer
            .overhead_mut()
            .set_enabled(self.is_device_playing);
    }

    fn set_midi_channel(&mut self, midi_channel: MidiChannel) {
        self.midi_channel_in = midi_channel;
    }
}

impl SourcesMidi for Arpeggiator {
    fn midi_sinks_mut(&mut self) -> &mut HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>> {
        self.beat_sequencer.midi_sinks_mut()
    }

    fn midi_sinks(&self) -> &HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>> {
        self.beat_sequencer.midi_sinks()
    }

    fn midi_output_channel(&self) -> MidiChannel {
        self.midi_channel_out
    }

    fn set_midi_output_channel(&mut self, midi_channel: MidiChannel) {
        self.midi_channel_out = midi_channel;
    }
}

impl WatchesClock for Arpeggiator {
    fn tick(&mut self, clock: &Clock) -> Vec<BigMessage> {
        self.beat_sequencer.tick(clock); // TODO: loop
        Vec::new()
    }
}

impl Terminates for Arpeggiator {
    fn is_finished(&self) -> bool {
        true
    }
}

impl IsMidiEffect for Arpeggiator {}

impl Arpeggiator {
    pub fn new_with(midi_channel_in: MidiChannel, midi_channel_out: MidiChannel) -> Self {
        Self {
            midi_channel_in,
            midi_channel_out,
            ..Default::default()
        }
    }

    pub fn new_wrapped_with(
        midi_channel_in: MidiChannel,
        midi_channel_out: MidiChannel,
    ) -> Rrc<Self> {
        let wrapped = rrc(Self::new_with(midi_channel_in, midi_channel_out));
        wrapped.borrow_mut().me = rrc_downgrade(&wrapped);
        wrapped
    }

    // TODO: placeholder for a bunch of knobs and dials
    pub(crate) fn nothing(&self) -> f32 {
        0.0
    }

    // this is a placeholder to get the trait requirements satisfied
    pub(crate) fn set_nothing(&mut self, _value: f32) {}

    fn insert_one_note(
        &mut self,
        when: PerfectTimeUnit,
        duration: PerfectTimeUnit,
        key: u8,
        vel: u8,
    ) {
        self.beat_sequencer.insert(
            when,
            self.midi_channel_out,
            MidiMessage::NoteOn {
                key: u7::from(key),
                vel: u7::from(vel),
            },
        );
        self.beat_sequencer.insert(
            when + duration,
            self.midi_channel_out,
            MidiMessage::NoteOff {
                key: u7::from(key),
                vel: u7::from(vel),
            },
        );
    }

    fn rebuild_sequence(&mut self, clock: &Clock, key: u8, vel: u8) {
        self.beat_sequencer.clear();

        let start_beat = crate::clock::PerfectTimeUnit(clock.beats());
        self.insert_one_note(
            start_beat + PerfectTimeUnit(0.25 * 0.0),
            PerfectTimeUnit(0.25),
            key,
            vel,
        );
        self.insert_one_note(
            start_beat + PerfectTimeUnit(0.25 * 1.0),
            PerfectTimeUnit(0.25),
            key + 2,
            vel,
        );
        self.insert_one_note(
            start_beat + PerfectTimeUnit(0.25 * 2.0),
            PerfectTimeUnit(0.25),
            key + 4,
            vel,
        );
        self.insert_one_note(
            start_beat + PerfectTimeUnit(0.25 * 3.0),
            PerfectTimeUnit(0.25),
            key + 5,
            vel,
        );
        self.insert_one_note(
            start_beat + PerfectTimeUnit(0.25 * 4.0),
            PerfectTimeUnit(0.25),
            key + 7,
            vel,
        );
        self.insert_one_note(
            start_beat + PerfectTimeUnit(0.25 * 5.0),
            PerfectTimeUnit(0.25),
            key + 9,
            vel,
        );
        self.insert_one_note(
            start_beat + PerfectTimeUnit(0.25 * 6.0),
            PerfectTimeUnit(0.25),
            key + 11,
            vel,
        );
    }
}

impl HasOverhead for Arpeggiator {
    fn overhead(&self) -> &Overhead {
        &self.overhead
    }

    fn overhead_mut(&mut self) -> &mut Overhead {
        &mut self.overhead
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instance() -> Arpeggiator {
        return Arpeggiator::default();
    }

    include!("../test_trait_is_midi_effect.rs");
}
