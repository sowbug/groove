use crate::backend::clock::Clock;
use crate::backend::midi;
use crate::backend::midi::MIDIReceiverTrait;
use std::f32::consts::PI;

use super::devices::DeviceTrait;
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
    fn tick(&mut self, time: f32) {
        if self.frequency > 0. {
            self.current_sample = match self.waveform {
                Waveform::Sine => (time * self.frequency * 2.0 * PI).sin(),
                Waveform::Square => {
                    if ((time * self.frequency * 2.0 * PI).sin()) < 0. {
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
    fn handle_midi_message(&mut self, note: u8) {
        self.frequency = match note {
            0 => 0.,
            _ => 2.0_f32.powf((note as f32 - 69.0) / 12.0) * 440.0,
        };    }
    fn get_audio_sample(&self) -> f32 {
        self.current_sample
    }
}

pub struct old_Oscillator {
    frequency: f32,
}

impl old_Oscillator {
    pub fn new() -> old_Oscillator {
        old_Oscillator { frequency: 0. }
    }

    pub fn get_sample(&self, clock: &Clock) -> f32 {
        (clock.sample_clock * self.frequency * 2.0 * std::f32::consts::PI / clock.sample_rate).sin()
    }
}

impl MIDIReceiverTrait for old_Oscillator {
    fn handle_midi(&mut self, midi_message: midi::MIDIMessage) -> bool {
        match midi_message.status {
            midi::MIDIMessageType::NoteOn => {
                self.frequency = midi_message.to_frequency();
                ()
            }
            midi::MIDIMessageType::NoteOff => {
                println!("note off");
                self.frequency = 0.;
                ()
            }
        }
        true
    }
}
