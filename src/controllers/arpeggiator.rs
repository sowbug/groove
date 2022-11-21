use super::sequencers::BeatSequencer;
use crate::{
    clock::{Clock, PerfectTimeUnit},
    messages::GrooveMessage,
    midi::{MidiChannel, MidiMessage},
    traits::{HasUid, IsController, Updateable, Terminates},
};
use midly::num::u7;

#[derive(Debug, Default)]
pub struct Arpeggiator {
    uid: usize,
    midi_channel_out: MidiChannel,
    beat_sequencer: BeatSequencer<GrooveMessage>,

    is_device_playing: bool,
}
impl IsController for Arpeggiator {}
impl Updateable for Arpeggiator {
    type Message = GrooveMessage;

    fn update(
        &mut self,
        clock: &Clock,
        message: Self::Message,
    ) -> crate::traits::EvenNewerCommand<Self::Message> {
        match message {
            GrooveMessage::Tick => return self.beat_sequencer.update(clock, message),
            GrooveMessage::Midi(_channel, message) => {
                match message {
                    MidiMessage::NoteOff { key: _, vel: _ } => self.is_device_playing = false,
                    MidiMessage::NoteOn { key, vel } => {
                        self.rebuild_sequence(clock, key.as_int(), vel.as_int());
                        self.is_device_playing = true;
                        //                self.sequence_start_beats = clock.beats();
                    }
                    MidiMessage::Aftertouch { key: _, vel: _ } => todo!(),
                    MidiMessage::Controller {
                        controller: _,
                        value: _,
                    } => todo!(),
                    MidiMessage::ProgramChange { program: _ } => todo!(),
                    MidiMessage::ChannelAftertouch { vel: _ } => todo!(),
                    MidiMessage::PitchBend { bend: _ } => todo!(),
                }
                self.beat_sequencer.enable(self.is_device_playing);
            }
            _ => todo!(),
        }
        crate::traits::EvenNewerCommand::none()
    }
}
impl Terminates for Arpeggiator {
    fn is_finished(&self) -> bool {
        true
    }
}
impl HasUid for Arpeggiator {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}

impl Arpeggiator {
    pub fn new_with(midi_channel_out: MidiChannel) -> Self {
        Self {
            midi_channel_out,
            ..Default::default()
        }
    }

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
