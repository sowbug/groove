use super::clock::Clock;
use super::devices::DeviceTrait;
use super::midi::{MidiMessage, MidiMessageType};
use crate::backend::midi;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::f32::consts::PI;
use std::rc::Rc;
pub enum Waveform {
    Sine,
    Square,
}

pub struct Oscillator {
    waveform: Waveform,
    current_sample: f32,
    frequency: f32,
}

impl Oscillator {
    pub fn new(waveform: Waveform) -> Oscillator {
        Oscillator {
            waveform,
            current_sample: 0.,
            frequency: 0.,
        }
    }
}
impl DeviceTrait for Oscillator {
    fn sinks_midi(&self) -> bool {
        true
    }
    fn sources_audio(&self) -> bool {
        true
    }
    fn tick(&mut self, clock: &Clock) {
        if self.frequency > 0. {
            self.current_sample = match self.waveform {
                Waveform::Sine => {
                    (clock.sample_clock / clock.sample_rate * self.frequency * 2.0 * PI).sin()
                }
                Waveform::Square => {
                    if ((clock.sample_clock / clock.sample_rate * self.frequency * 2.0 * PI).sin())
                        < 0.
                    {
                        -1.
                    } else {
                        1. // TODO(miket): this is lazy and wrong
                    }
                }
            }
        } else {
            self.current_sample = 0.
        }
    }
    fn handle_midi_message(&mut self, message: &MidiMessage) {
        match message.status {
            midi::MidiMessageType::NoteOn => {
                self.frequency = message.to_frequency();
            }
            midi::MidiMessageType::_NoteOff => {
                self.frequency = 0.;
            }
        }
    }

    fn get_audio_sample(&self) -> f32 {
        self.current_sample
    }
}

pub struct Note {
    which: u8,
    when: f32,
}
pub struct Sequencer {
    sinks: Vec<Rc<RefCell<dyn DeviceTrait>>>,
    note_events: VecDeque<Note>,
}

impl Sequencer {
    pub fn new() -> Sequencer {
        Sequencer {
            sinks: Vec::new(),
            note_events: VecDeque::new(),
        }
    }

    pub fn add_note_on(&mut self, which: u8, when: f32) {
        self.note_events.push_back(Note { which, when });
    }
    pub fn add_note_off(&mut self, which: u8, when: f32) {
        self.note_events.push_back(Note { which, when });
    }
}

impl DeviceTrait for Sequencer {
    fn sources_midi(&self) -> bool {
        true
    }

    fn tick(&mut self, clock: &Clock) {
        let note = self.note_events.pop_front().unwrap();
        if clock.real_clock >= note.when {
            let midi_message = MidiMessage {
                status: MidiMessageType::NoteOn,
                channel: 0,
                data1: note.which,
                data2: 0,
            };
            println!("I'm sending a note {} at {}", clock.real_clock, note.which);
            for i in self.sinks.clone() {
                i.borrow_mut().handle_midi_message(&midi_message);
            }
        } else {
            // TODO(miket): I had to always pop always and then sometimes re-push because
            // I can't figure out how to get around the borrow checker if I use just a front().
            self.note_events.push_front(note);
        }
    }

    fn connect_midi_sink(&mut self, device: Rc<RefCell<dyn DeviceTrait>>) {
        self.sinks.push(device);
    }
}
