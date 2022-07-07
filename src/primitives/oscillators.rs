use std::f32::consts::PI;

use crate::backend::{
    clock::Clock,
    devices::DeviceTrait,
    midi::{self, MidiMessage},
};

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Waveform {
    Sine,
    Square,
    Triangle,
    Sawtooth,
}

impl Default for Waveform {
    fn default() -> Self {
        Waveform::Sine
    }
}

#[derive(Default, Debug)]
pub struct Oscillator {
    waveform: Waveform,
    current_sample: f32,
    frequency: f32,
}

// TODO: these oscillators are pure in a logical sense, but they alias badly in the real world
// of discrete sampling. Investigate replacing with smoothed waveforms.
impl Oscillator {
    pub fn new(waveform: Waveform) -> Oscillator {
        Oscillator {
            waveform,
            ..Default::default()
        }
    }
    pub fn get_frequency(&self) -> f32 {
        self.frequency
    }
    pub fn set_frequency(&mut self, frequency: f32) {
        self.frequency = frequency;
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
            let phase_normalized = self.frequency * (clock.seconds as f32);
            self.current_sample = match self.waveform {
                // https://en.wikipedia.org/wiki/Sine_wave
                Waveform::Sine => (phase_normalized * 2.0 * PI).sin(),
                // https://en.wikipedia.org/wiki/Square_wave
                Waveform::Square => (phase_normalized * 2.0 * PI).sin().signum(),
                // https://en.wikipedia.org/wiki/Triangle_wave
                Waveform::Triangle => {
                    4.0 * (phase_normalized - (0.75 + phase_normalized).floor() + 0.25).abs() - 1.0
                }
                // https://en.wikipedia.org/wiki/Sawtooth_wave
                Waveform::Sawtooth => 2.0 * (phase_normalized - (0.5 + phase_normalized).floor()),
            }
        } else {
            self.current_sample = 0.
        }
        true
    }
    fn handle_midi_message(&mut self, message: &MidiMessage, _clock: &Clock) {
        match message.status {
            midi::MidiMessageType::NoteOn => {
                self.frequency = message.to_frequency();
            }
            midi::MidiMessageType::NoteOff => {
                // TODO(miket): now that oscillators are in envelopes, they generally turn on but don't turn off.
                // these might not end up being full DeviceTrait devices, but rather owned/managed by synths.
                //self.frequency = 0.;
            }
        }
    }

    fn get_audio_sample(&self) -> f32 {
        self.current_sample
    }
}
