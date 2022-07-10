use std::f32::consts::PI;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Waveform {
    None,
    Sine,
    Square,
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
    waveform: Waveform,
    duty_cycle: f32,
    frequency: f32,

    noise_x1: u32,
    noise_x2: u32,
}

impl MiniOscillator {
    pub fn new(waveform: Waveform, frequency: f32) -> Self {
        Self {
            waveform,
            duty_cycle: 0.5,
            ..Default::default()
        }
    }
    pub fn new_pwm_square(duty_cycle: f32, frequency: f32) -> Self {
        Self {
            waveform: Waveform::Square,
            duty_cycle,
            ..Default::default()
        }
    }
    pub fn new_noise() -> Self {
        Self {
            waveform: Waveform::Noise,
            noise_x1: 0x70f4f854,
            noise_x2: 0xe1e9f0a7,
            ..Default::default()
        }
    }

    pub fn process(&mut self, time_seconds: f32) -> f32 {
        let phase_normalized = self.frequency * time_seconds;
        match self.waveform {
            Waveform::None => 0.0,
            // https://en.wikipedia.org/wiki/Sine_wave
            Waveform::Sine => (phase_normalized * 2.0 * PI).sin(),
            // https://en.wikipedia.org/wiki/Square_wave
            //Waveform::Square => (phase_normalized * 2.0 * PI).sin().signum(),
            Waveform::Square => {
                // TODO: make sure this is right. I eyeballed it when implementing PWM waves.
                (self.duty_cycle - (phase_normalized - phase_normalized.floor())).signum()
            }
            // https://en.wikipedia.org/wiki/Triangle_wave
            Waveform::Triangle => {
                4.0 * (phase_normalized - (0.75 + phase_normalized).floor() + 0.25).abs() - 1.0
            }
            // https://en.wikipedia.org/wiki/Sawtooth_wave
            Waveform::Sawtooth => 2.0 * (phase_normalized - (0.5 + phase_normalized).floor()),
            // https://www.musicdsp.org/en/latest/Synthesis/216-fast-whitenoise-generator.html
            Waveform::Noise => {
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
}
