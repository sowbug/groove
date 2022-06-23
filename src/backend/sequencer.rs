use crate::backend::clock::Clock;
use crate::backend::clock::ClockWatcherTrait;
use crate::backend::instruments::old_Oscillator;
use crate::backend::midi;
use crate::backend::midi::MIDIReceiverTrait;
use std::collections::VecDeque;

pub struct Note {
    when: f32,
    which: u8,
}

pub struct old_Sequencer {
    oscillator: old_Oscillator,
    notes: VecDeque<Note>,
}

impl old_Sequencer {
    pub fn new() -> old_Sequencer {
        old_Sequencer {
            oscillator: old_Oscillator::new(),
            notes: VecDeque::new(),
        }
    }
    pub fn add_note(&mut self, which: u8, when: f32, _duration: f32) {
        self.notes.push_back(Note {
            when: when,
            which: which,
        });
    }

    pub fn attach(&self, oscillator: old_Oscillator) {
       // self.oscillator = oscillator;
    }
}

impl ClockWatcherTrait for old_Sequencer {
    fn handle_time_slice(&mut self, clock: &Clock) -> bool {
        if !self.notes.is_empty() {
            let note = self.notes.pop_front().unwrap();
            if clock.real_clock >= note.when {
                let midi_message = midi::MIDIMessage {
                    status: midi::MIDIMessageType::NoteOn,
                    channel: 0,
                    data1: note.which,
                    data2: 0,
                };
                println!("I'm sending a note {} at {}", clock.real_clock, note.which);
                self.oscillator.handle_midi(midi_message);
            } else {
                // TODO(miket): I had to always pop always and then sometimes re-push because
                // I can't figure out how to get around the borrow checker if I use just a front().
                self.notes.push_front(note);
            }
        }

        true
    }
}
