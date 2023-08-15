// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::{sequencers::Sequencer, SequencerParams};
use groove_core::{
    midi::{new_note_off, new_note_on, HandlesMidi, MidiChannel, MidiMessage, MidiMessagesFn},
    time::{MusicalTime, PerfectTimeUnit, SampleRate},
    traits::{Configurable, ControlEventsFn, Controls, Serializable},
    ParameterType,
};
use groove_proc_macros::{Control, IsController, Params, Uid};
use std::{ops::Range, option::Option};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

/// [Arpeggiator] creates [arpeggios](https://en.wikipedia.org/wiki/Arpeggio),
/// which "is a type of broken chord in which the notes that compose a chord are
/// individually and quickly sounded in a progressive rising or descending
/// order." You can also think of it as a hybrid MIDI instrument and MIDI
/// controller; you play it with MIDI, but instead of producing audio, it
/// produces more MIDI.
#[derive(Debug, Control, IsController, Params, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Arpeggiator {
    uid: groove_core::Uid,
    midi_channel_out: MidiChannel,
    sequencer: Sequencer,

    #[control]
    #[params]
    bpm: ParameterType,

    // A poor-man's semaphore that allows note-off events to overlap with the
    // current note without causing it to shut off. Example is a legato
    // playing-style of the MIDI instrument that controls the arpeggiator. If we
    // turned on and off solely by the last note-on/off we received, then the
    // arpeggiator would frequently get clipped.
    note_semaphore: i16,
}
impl Serializable for Arpeggiator {}
impl Configurable for Arpeggiator {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sequencer.update_sample_rate(sample_rate);
    }
}
impl Controls for Arpeggiator {
    fn update_time(&mut self, range: &Range<MusicalTime>) {
        self.sequencer.update_time(range);
    }

    fn work(&mut self, control_events_fn: &mut ControlEventsFn) {
        self.sequencer.work(control_events_fn)
    }

    fn is_finished(&self) -> bool {
        self.sequencer.is_finished()
    }

    fn play(&mut self) {
        self.sequencer.play();
    }

    fn stop(&mut self) {
        self.sequencer.stop();
    }

    fn skip_to_start(&mut self) {
        self.sequencer.skip_to_start();
    }

    fn set_loop(&mut self, range: &Range<PerfectTimeUnit>) {
        self.sequencer.set_loop(range);
    }

    fn clear_loop(&mut self) {
        self.sequencer.clear_loop();
    }

    fn set_loop_enabled(&mut self, is_enabled: bool) {
        self.sequencer.set_loop_enabled(is_enabled);
    }

    fn is_performing(&self) -> bool {
        self.sequencer.is_performing()
    }
}
impl HandlesMidi for Arpeggiator {
    fn handle_midi_message(
        &mut self,
        _channel: MidiChannel,
        message: MidiMessage,
        midi_messages_fn: &mut MidiMessagesFn,
    ) {
        match message {
            MidiMessage::NoteOff { key: _, vel: _ } => {
                self.note_semaphore -= 1;
                if self.note_semaphore < 0 {
                    self.note_semaphore = 0;
                }
                self.sequencer.enable(self.note_semaphore > 0);
            }
            MidiMessage::NoteOn { key, vel } => {
                self.note_semaphore += 1;
                self.rebuild_sequence(key.as_int(), vel.as_int());
                self.sequencer.enable(true);

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
                self.sequencer
                    .generate_midi_messages_for_current_frame(midi_messages_fn);
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
}

impl Arpeggiator {
    pub fn new_with(params: &ArpeggiatorParams, midi_channel_out: MidiChannel) -> Self {
        Self {
            uid: Default::default(),
            midi_channel_out,
            bpm: params.bpm,
            sequencer: Sequencer::new_with(&SequencerParams { bpm: params.bpm() }),
            note_semaphore: Default::default(),
        }
    }

    fn insert_one_note(&mut self, when: &MusicalTime, duration: &MusicalTime, key: u8, vel: u8) {
        self.sequencer
            .insert(when, self.midi_channel_out, new_note_on(key, vel));
        self.sequencer.insert(
            &(*when + *duration),
            self.midi_channel_out,
            new_note_off(key, 0),
        );
    }

    fn rebuild_sequence(&mut self, key: u8, vel: u8) {
        self.sequencer.clear();

        let start_beat = self.sequencer.cursor().clone();
        let duration = MusicalTime::new_with_parts(4); // TODO: we're ignoring time signature!
        let scale_notes = [0, 2, 4, 5, 7, 9, 11];
        for (index, offset) in scale_notes.iter().enumerate() {
            // TODO - more examples of needing wider range for smaller parts
            let when = start_beat + MusicalTime::new_with_parts(4 * index);
            self.insert_one_note(&when, &duration, key + offset, vel);
        }
    }

    #[cfg(feature = "iced-framework")]
    pub fn update(&mut self, message: ArpeggiatorMessage) {
        match message {
            ArpeggiatorMessage::Arpeggiator(s) => *self = Self::new_with(self.midi_channel_out, s),
            _ => self.derived_update(message),
        }
    }

    pub fn bpm(&self) -> f64 {
        self.bpm
    }

    pub fn set_bpm(&mut self, bpm: ParameterType) {
        self.bpm = bpm;
    }
}

#[cfg(feature = "egui-framework")]
mod gui {
    use super::Arpeggiator;
    use eframe::egui::{ComboBox, Ui};
    use groove_core::traits::gui::Shows;

    impl Shows for Arpeggiator {
        fn show(&mut self, ui: &mut Ui) {
            let alternatives = ["major", "minor"];
            let mut selected = 1;
            if ComboBox::from_label("Scale")
                .show_index(ui, &mut selected, alternatives.len(), |i| alternatives[i])
                .clicked()
            {}
        }
    }
}
