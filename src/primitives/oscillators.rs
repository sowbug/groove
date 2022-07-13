use std::f32::consts::PI;

use crate::preset::{LfoPreset, OscillatorPreset};

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Waveform {
    None,
    Sine,
    Square(f32),
    Triangle,
    Sawtooth,
    Noise,
}

impl Default for Waveform {
    fn default() -> Self {
        Waveform::Sine
    }
}

#[derive(Default, Debug)]
pub struct MiniOscillator {
    pub waveform: Waveform,
    frequency: f32,
    frequency_modulation: f32,
    frequency_tune: f32,

    noise_x1: u32,
    noise_x2: u32,
}

impl MiniOscillator {
    pub fn new(waveform: Waveform) -> Self {
        Self {
            waveform,
            noise_x1: 0x70f4f854,
            noise_x2: 0xe1e9f0a7,
            frequency_tune: 1.0,
            ..Default::default()
        }
    }

    pub fn new_from_preset(preset: &OscillatorPreset) -> Self {
        Self {
            waveform: preset.waveform,
            frequency_tune: preset.tune,
            noise_x1: 0x70f4f854,
            noise_x2: 0xe1e9f0a7,
            ..Default::default()
        }
    }

    pub fn new_lfo(lfo_preset: &LfoPreset) -> Self {
        Self {
            waveform: lfo_preset.waveform,
            frequency: lfo_preset.frequency,
            frequency_tune: 1.0,
            noise_x1: 0x70f4f854,
            noise_x2: 0xe1e9f0a7,
            ..Default::default()
        }
    }

    pub fn process(&mut self, time_seconds: f32) -> f32 {
        let phase_normalized =
            self.frequency * self.frequency_tune * (1.0 + self.frequency_modulation) * time_seconds;
        match self.waveform {
            Waveform::None => 0.0,
            // https://en.wikipedia.org/wiki/Sine_wave
            Waveform::Sine => (phase_normalized * 2.0 * PI).sin(),
            // https://en.wikipedia.org/wiki/Square_wave
            //Waveform::Square => (phase_normalized * 2.0 * PI).sin().signum(),
            Waveform::Square(duty_cycle) => {
                // TODO: make sure this is right. I eyeballed it when implementing PWM waves.
                (duty_cycle - (phase_normalized - phase_normalized.floor())).signum()
            }
            // https://en.wikipedia.org/wiki/Triangle_wave
            Waveform::Triangle => {
                4.0 * (phase_normalized - (0.75 + phase_normalized).floor() + 0.25).abs() - 1.0
            }
            // https://en.wikipedia.org/wiki/Sawtooth_wave
            Waveform::Sawtooth => 2.0 * (phase_normalized - (0.5 + phase_normalized).floor()),
            // https://www.musicdsp.org/en/latest/Synthesis/216-fast-whitenoise-generator.html
            Waveform::Noise => {
                // TODO: this is stateful, so random access will sound different from sequential, as will different sample rates.
                // It also makes this method require mut. Is there a noise algorithm that can modulate on time_seconds? (It's a
                // complicated question, potentially.)
                self.noise_x1 ^= self.noise_x2;
                let tmp = 2.0 * (self.noise_x2 as f32 - (u32::MAX as f32 / 2.0)) / u32::MAX as f32;
                (self.noise_x2, _) = self.noise_x2.overflowing_add(self.noise_x1);
                tmp
            }
        }
    }

    pub(crate) fn set_frequency(&mut self, frequency: f32) {
        self.frequency = frequency;
    }

    pub(crate) fn set_frequency_modulation(&mut self, frequency_modulation: f32) {
        self.frequency_modulation = frequency_modulation;
    }
}
