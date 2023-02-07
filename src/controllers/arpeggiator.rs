use super::sequencers::BeatSequencer;
use crate::{
    clock::{Clock, PerfectTimeUnit},
    common::F32ControlValue,
    midi::{MidiChannel, MidiMessage},
    traits::{Controllable, HasUid, IsController, Response, Terminates, Updateable},
    EntityMessage,
};
use groove_macros::{Control, Uid};
use midly::num::u7;
use std::str::FromStr;
use strum_macros::{Display, EnumString, FromRepr};

#[derive(Control, Debug, Default, Uid)]
pub struct Arpeggiator {
    uid: usize,
    midi_channel_out: MidiChannel,
    beat_sequencer: BeatSequencer,

    // A poor-man's semaphore that allows note-off events to overlap with the
    // current note without causing it to shut off. Example is a legato
    // playing-style of the MIDI instrument that controls the arpeggiator. If we
    // turned on and off solely by the last note-on/off we received, then the
    // arpeggiator would frequently get clipped.
    note_semaphore: i16,
}
impl IsController for Arpeggiator {}
impl Updateable for Arpeggiator {
    fn update(&mut self, clock: &Clock, message: EntityMessage) -> Response<EntityMessage> {
        match message {
            EntityMessage::Tick => {
                return self.beat_sequencer.update(clock, message);
            }
            EntityMessage::Midi(_channel, message) => {
                match message {
                    MidiMessage::NoteOff { key: _, vel: _ } => {
                        self.note_semaphore -= 1;
                        if self.note_semaphore < 0 {
                            self.note_semaphore = 0;
                        }
                        self.beat_sequencer.enable(self.note_semaphore > 0);
                    }
                    MidiMessage::NoteOn { key, vel } => {
                        self.note_semaphore += 1;
                        self.rebuild_sequence(clock, key.as_int(), vel.as_int());
                        self.beat_sequencer.enable(true);
                        //                self.sequence_start_beats = clock.beats();

                        // TODO: this scratches the itch of needing to respond
                        // to a note-down with a note *during this slice*, but
                        // it also has an edge condition where we need to cancel
                        // a different note that was might have been supposed to
                        // be sent instead during this slice, or at least
                        // immediately shut it off. This seems to require a
                        // two-phase Tick handler (one to decide what we're
                        // going to send, and another to send it), and an
                        // internal memory of which notes we've asked the
                        // downstream to play. TODO TODO TODO
                        return self.beat_sequencer.update(clock, EntityMessage::Tick);
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
            }
            _ => todo!(),
        }
        Response::none()
    }
}
impl Terminates for Arpeggiator {
    fn is_finished(&self) -> bool {
        true
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
                vel: u7::from(0),
            },
        );
    }

    fn rebuild_sequence(&mut self, clock: &Clock, key: u8, vel: u8) {
        self.beat_sequencer.clear();

        // TODO: this is a good place to start pulling the f32 time thread --
        // remove that ".into()" and deal with it
        let start_beat = crate::clock::PerfectTimeUnit(clock.beats().into());
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

#[cfg(test)]
mod tests {
    use crate::{
        clock::PerfectTimeUnit,
        controllers::sequencers::BeatSequencer,
        entities::BoxedEntity,
        midi::MidiChannel,
        traits::{Internal, TestInstrument},
        Clock, GrooveMessage, Orchestrator,
    };

    use super::Arpeggiator;

    // Orchestrator sends a Tick message to everyone in an undefined order, and
    // routes the resulting messages to everyone in yet another undefined order.
    // This causes a problem. If we have a sequencer driving an arpeggiator, and
    // the two together are supposed to play a note at Time 0, then it's
    // possible that the events will happen as follows:
    //
    // Tick to Arp -> nothing emitted, because it's not playing Tick to
    // Sequencer -> emit Midi, delivered straight to Arp
    //
    // and that's pretty much it, because the event loop is done. Worse, the Arp
    // will never send the note-on MIDI message to its downstream instrument(s),
    // because by the time of its next Tick (when it calculates when to send
    // stuff), it's Time 1, but the note should have been sent at Time 0, so
    // that note-on is skipped.
    #[test]
    fn test_arpeggiator_sends_command_on_correct_time_slice() {
        let mut sequencer = Box::new(BeatSequencer::default());
        const MIDI_CHANNEL_SEQUENCER_TO_ARP: MidiChannel = 7;
        const MIDI_CHANNEL_ARP_TO_INSTRUMENT: MidiChannel = 8;
        let arpeggiator = Box::new(Arpeggiator::new_with(MIDI_CHANNEL_ARP_TO_INSTRUMENT));
        let instrument = Box::new(TestInstrument::default());
        let mut o = Orchestrator::default();

        sequencer.insert(
            PerfectTimeUnit(0.0),
            MIDI_CHANNEL_SEQUENCER_TO_ARP,
            midly::MidiMessage::NoteOn {
                key: 99.into(),
                vel: 88.into(),
            },
        );

        let arpeggiator_uid = o.add(None, BoxedEntity::Arpeggiator(arpeggiator));
        o.connect_midi_downstream(arpeggiator_uid, MIDI_CHANNEL_SEQUENCER_TO_ARP);
        let instrument_uid = o.add(None, BoxedEntity::TestInstrument(instrument));
        o.connect_midi_downstream(instrument_uid, MIDI_CHANNEL_ARP_TO_INSTRUMENT);
        let _sequencer_uid = o.add(None, BoxedEntity::BeatSequencer(sequencer));

        let clock = Clock::default();
        let command = o.update(&clock, GrooveMessage::Tick);
        if let Internal::Batch(messages) = command.0 {
            assert_eq!(messages.len(), 4);
            match messages[0] {
                GrooveMessage::MidiToExternal(channel, _message) => {
                    assert_eq!(channel, 7);
                }
                _ => panic!(),
            };
            match messages[1] {
                GrooveMessage::MidiToExternal(channel, _message) => {
                    assert_eq!(channel, 8);
                }
                _ => panic!(),
            };
            match messages[2] {
                GrooveMessage::AudioOutput(_) => {}
                _ => panic!(),
            };
            match messages[3] {
                GrooveMessage::OutputComplete => {}
                _ => panic!(),
            };
        } else {
            panic!("command wasn't Batch type");
        }
    }
}
