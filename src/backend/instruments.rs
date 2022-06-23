use super::clock::Clock;
use super::devices::DeviceTrait;
use super::midi::{MidiMessage, MidiMessageType};
use crate::backend::midi;
use std::cell::RefCell;
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
            waveform: waveform,
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
            midi::MidiMessageType::NoteOff => {
                self.frequency = 0.;
            }
        }
    }

    fn get_audio_sample(&self) -> f32 {
        self.current_sample
    }
}

pub struct Sequencer {
    sinks: Vec<Rc<RefCell<dyn DeviceTrait>>>,
}

impl Sequencer {
    pub fn new() -> Sequencer {
        Sequencer { sinks: Vec::new() }
    }
}

impl DeviceTrait for Sequencer {
    fn sources_midi(&self) -> bool {
        true
    }

    fn tick(&mut self, clock: &Clock) {
        let note = if clock.real_clock < 0.25 {
            0
        } else if clock.real_clock < 0.50 {
            60
        } else if clock.real_clock < 0.75 {
            66
        } else {
            0
        };

        let message_type = match note {
            0 => MidiMessageType::NoteOff,
            _ => MidiMessageType::NoteOn,
        };
        let message = MidiMessage {
            status: message_type,
            channel: 0,
            data1: note,
            data2: 0,
        };
        for i in self.sinks.clone() {
            i.borrow_mut().handle_midi_message(&message);
        }
    }

    fn connect_midi_sink(&mut self, device: Rc<RefCell<dyn DeviceTrait>>) {
        self.sinks.push(device);
    }
}
