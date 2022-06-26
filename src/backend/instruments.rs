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
    Triangle,
    Sawtooth,
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
    fn tick(&mut self, clock: &Clock) -> bool {
        if self.frequency > 0. {
            let phase_normalized = (clock.sample_clock / clock.sample_rate * self.frequency) % 1.0;
            self.current_sample = match self.waveform {
                // https://en.wikipedia.org/wiki/Sine_wave
                // https://en.wikipedia.org/wiki/Square_wave
                // https://en.wikipedia.org/wiki/Triangle_wave
                // https://en.wikipedia.org/wiki/Sawtooth_wave
                Waveform::Sine => (phase_normalized * 2.0 * PI).sin(),
                Waveform::Square => (phase_normalized * 2.0 * PI).sin().signum(),
                Waveform::Triangle => {
                    4.0 * (phase_normalized - (0.75 + phase_normalized).floor() + 0.25).abs() - 1.0
                }

                Waveform::Sawtooth => 2.0 * (phase_normalized - (0.5 + phase_normalized).floor()),
            }
        } else {
            self.current_sample = 0.
        }
        true
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

pub struct TimeSignature {
    numerator: usize,
    denominator: usize,
}

impl TimeSignature {
    pub fn new(numerator: usize, denominator: usize) -> TimeSignature {
        TimeSignature {
            numerator,
            denominator,
        }
    }
}
pub struct Sequencer {
    time_signature: TimeSignature,
    midi_ticks_per_second: usize,
    sinks: Vec<Rc<RefCell<dyn DeviceTrait>>>,
    midi_messages: VecDeque<(usize, MidiMessage)>,
}

impl Sequencer {
    pub fn new() -> Sequencer {
        Sequencer {
            time_signature: TimeSignature::new(4, 4),
            midi_ticks_per_second: 0,
            sinks: Vec::new(),
            midi_messages: VecDeque::new(),
        }
    }

    pub fn set_time_signature(&mut self, numerator: usize, denominator: usize) {
        self.time_signature = TimeSignature::new(numerator, denominator);
    }

    pub fn set_midi_ticks_per_second(&mut self, tps: usize) {
        self.midi_ticks_per_second = tps;
    }

    pub fn add_message(&mut self, when: usize, message: MidiMessage) {
        self.midi_messages.push_back((when, message));
    }
    pub fn add_note_on(&mut self, when: usize, which: u8) {
        let midi_message = MidiMessage {
            status: MidiMessageType::NoteOn,
            channel: 0,
            data1: which,
            data2: 0,
        };
        self.midi_messages.push_back((when, midi_message));
    }
    pub fn add_note_off(&mut self, when: usize, which: u8) {
        let midi_message = MidiMessage {
            status: MidiMessageType::NoteOff,
            channel: 0,
            data1: which,
            data2: 0,
        };
        self.midi_messages.push_back((when, midi_message));
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
        let (when, midi_message) = self.midi_messages.pop_front().unwrap();

        // TODO(miket): I'm getting a bad feeling about the usize and f32 conversions.
        let elapsed_midi_ticks: usize =
            ((clock.real_clock * self.midi_ticks_per_second as f32) as u32) as usize;
        if elapsed_midi_ticks >= when {
            for i in self.sinks.clone() {
                i.borrow_mut().handle_midi_message(&midi_message);
            }
        } else {
            // TODO(miket): I had to always pop always and then sometimes re-push because
            // I can't figure out how to get around the borrow checker if I use just a front().
            self.midi_messages.push_front((when, midi_message));
        }
        false
    }

    fn connect_midi_sink(&mut self, device: Rc<RefCell<dyn DeviceTrait>>) {
        self.sinks.push(device);
    }
}
