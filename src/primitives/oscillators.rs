use std::f32::consts::PI;

use crate::{
    common::{WaveformType, MonoSample},
    preset::{LfoPreset, OscillatorPreset},
};

use super::AudioSourceTrait;

#[derive(Debug, Clone)]
pub struct MiniOscillator {
    pub waveform: WaveformType,

    // Hertz. Any positive number. 440 = A4
    frequency: f32,

    // if not zero, then ignores the `frequency` field and this one instead.
    fixed_frequency: f32,

    // 1.0 is no change. 2.0 doubles the frequency. 0.5 halves it. Designed for pitch correction at construction time.
    frequency_tune: f32,

    // [-1, 1] is typical range, with -1 halving the frequency, and 1 doubling it. Designed for LFO and frequent changes.
    frequency_modulation: f32,

    noise_x1: u32,
    noise_x2: u32,
}

impl Default for MiniOscillator {
    fn default() -> Self {
        Self {
            waveform: WaveformType::None,
            frequency: 0.0,
            fixed_frequency: 0.0,
            frequency_tune: 1.0,
            frequency_modulation: 0.0,
            noise_x1: 0x70f4f854,
            noise_x2: 0xe1e9f0a7,
        }
    }
}

impl MiniOscillator {
    pub fn new(waveform: WaveformType) -> Self {
        Self {
            waveform,
            noise_x1: 0x70f4f854,
            noise_x2: 0xe1e9f0a7,
            ..Default::default()
        }
        // TODO: assert that if PWM, range is (0.0, 0.5). 0.0 is None, and 0.5 is Square.
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
            noise_x1: 0x70f4f854,
            noise_x2: 0xe1e9f0a7,
            ..Default::default()
        }
    }

    pub(crate) fn adjusted_frequency(&self) -> f32 {
        if self.fixed_frequency == 0.0 {
            self.frequency * (self.frequency_tune) * (2.0f32.powf(self.frequency_modulation))
        } else {
            self.fixed_frequency * (2.0f32.powf(self.frequency_modulation))
        }
    }

    pub(crate) fn set_frequency(&mut self, frequency: f32) {
        self.frequency = frequency;
    }

    pub(crate) fn set_fixed_frequency(&mut self, frequency: f32) {
        self.fixed_frequency = frequency;
    }

    pub(crate) fn set_frequency_modulation(&mut self, frequency_modulation: f32) {
        self.frequency_modulation = frequency_modulation;
    }
}

impl AudioSourceTrait for MiniOscillator {
    fn process(&mut self, time_seconds: f32) -> MonoSample {
        let phase_normalized = (self.adjusted_frequency() * time_seconds) as MonoSample;
        match self.waveform {
            WaveformType::None => 0.0,
            // https://en.wikipedia.org/wiki/Sine_wave
            WaveformType::Sine => (phase_normalized * 2.0 * PI).sin(),
            // https://en.wikipedia.org/wiki/Square_wave
            //Waveform::Square => (phase_normalized * 2.0 * PI).sin().signum(),
            WaveformType::Square => (0.5 - (phase_normalized - phase_normalized.floor())).signum(),
            WaveformType::PulseWidth(duty_cycle) => {
                (duty_cycle as MonoSample - (phase_normalized - phase_normalized.floor())).signum() as MonoSample
            }
            // https://en.wikipedia.org/wiki/Triangle_wave
            WaveformType::Triangle => {
                4.0 * (phase_normalized - (0.75 + phase_normalized).floor() + 0.25).abs() - 1.0
            }
            // https://en.wikipedia.org/wiki/Sawtooth_wave
            WaveformType::Sawtooth => 2.0 * (phase_normalized - (0.5 + phase_normalized).floor()),
            // https://www.musicdsp.org/en/latest/Synthesis/216-fast-whitenoise-generator.html
            WaveformType::Noise => {
                // TODO: this is stateful, so random access will sound different from sequential, as will different sample rates.
                // It also makes this method require mut. Is there a noise algorithm that can modulate on time_seconds? (It's a
                // complicated question, potentially.)
                self.noise_x1 ^= self.noise_x2;
                let tmp = 2.0 * (self.noise_x2 as MonoSample - (u32::MAX as MonoSample / 2.0)) / u32::MAX as MonoSample;
                (self.noise_x2, _) = self.noise_x2.overflowing_add(self.noise_x1);
                tmp
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        common::{MidiMessage, MidiNote},
        preset::OscillatorPreset,
        primitives::tests::write_source_to_file,
    };

    use super::{MiniOscillator, WaveformType};

    fn create_oscillator(waveform: WaveformType, tune: f32, note: MidiNote) -> MiniOscillator {
        let mut oscillator = MiniOscillator::new_from_preset(&OscillatorPreset {
            waveform,
            tune,
            ..Default::default()
        });
        oscillator.set_frequency(MidiMessage::note_type_to_frequency(note));
        oscillator
    }

    #[test]
    fn test_oscillator_basic_waveforms() {
        let mut oscillator = create_oscillator(
            WaveformType::Sine,
            OscillatorPreset::NATURAL_TUNING,
            MidiNote::C4,
        );
        assert_eq!(
            oscillator.adjusted_frequency(),
            MidiMessage::note_type_to_frequency(MidiNote::C4)
        );
        write_source_to_file(&mut oscillator, "oscillator_sine_c3");

        let mut oscillator = create_oscillator(
            WaveformType::Square,
            OscillatorPreset::NATURAL_TUNING,
            MidiNote::C4,
        );
        write_source_to_file(&mut oscillator, "oscillator_square_c3");

        let mut oscillator = create_oscillator(
            WaveformType::PulseWidth(0.1),
            OscillatorPreset::NATURAL_TUNING,
            MidiNote::C4,
        );
        write_source_to_file(&mut oscillator, "oscillator_pulse_width_10_percent_c3");

        let mut oscillator = create_oscillator(
            WaveformType::Triangle,
            OscillatorPreset::NATURAL_TUNING,
            MidiNote::C4,
        );
        write_source_to_file(&mut oscillator, "oscillator_triangle_c3");

        let mut oscillator = create_oscillator(
            WaveformType::Sawtooth,
            OscillatorPreset::NATURAL_TUNING,
            MidiNote::C4,
        );
        write_source_to_file(&mut oscillator, "oscillator_sawtooth_c3");

        let mut oscillator = create_oscillator(
            WaveformType::Noise,
            OscillatorPreset::NATURAL_TUNING,
            MidiNote::None,
        );
        write_source_to_file(&mut oscillator, "oscillator_noise");

        let mut oscillator = create_oscillator(
            WaveformType::None,
            OscillatorPreset::NATURAL_TUNING,
            MidiNote::None,
        );
        write_source_to_file(&mut oscillator, "oscillator_none");
    }

    #[test]
    fn test_oscillator_tuned() {
        let mut oscillator = create_oscillator(
            WaveformType::Sine,
            OscillatorPreset::octaves(0.0),
            MidiNote::C4,
        );
        assert_eq!(
            oscillator.adjusted_frequency(),
            MidiMessage::note_type_to_frequency(MidiNote::C4)
        );
        write_source_to_file(&mut oscillator, "oscillator_sine_c4_plus_zero_octave");

        let mut oscillator = create_oscillator(
            WaveformType::Sine,
            OscillatorPreset::octaves(1.0),
            MidiNote::C4,
        );
        assert_eq!(
            oscillator.adjusted_frequency(),
            MidiMessage::note_type_to_frequency(MidiNote::C4) * 2.0
        );
        write_source_to_file(&mut oscillator, "oscillator_sine_c4_plus_1_octave");

        let mut oscillator = create_oscillator(
            WaveformType::Sine,
            OscillatorPreset::octaves(-1.0),
            MidiNote::C4,
        );
        assert_eq!(
            oscillator.adjusted_frequency(),
            MidiMessage::note_type_to_frequency(MidiNote::C4) / 2.0
        );
        write_source_to_file(&mut oscillator, "oscillator_sine_c4_minus_1_octave");

        let mut oscillator = create_oscillator(
            WaveformType::Sine,
            OscillatorPreset::semis_and_cents(12.0, 0.0),
            MidiNote::C4,
        );
        assert_eq!(
            oscillator.adjusted_frequency(),
            MidiMessage::note_type_to_frequency(MidiNote::C4) * 2.0
        );
        write_source_to_file(&mut oscillator, "oscillator_sine_c4_plus_12_semitone");

        let mut oscillator = create_oscillator(
            WaveformType::Sine,
            OscillatorPreset::semis_and_cents(0.0, -1200.0),
            MidiNote::C4,
        );
        assert_eq!(
            oscillator.adjusted_frequency(),
            MidiMessage::note_type_to_frequency(MidiNote::C4) / 2.0
        );
        write_source_to_file(&mut oscillator, "oscillator_sine_c4_minus_1200_cents");
    }

    #[test]
    fn test_oscillator_modulated() {
        let mut oscillator = create_oscillator(
            WaveformType::Sine,
            OscillatorPreset::octaves(0.0),
            MidiNote::C4,
        );
        assert_eq!(
            oscillator.adjusted_frequency(),
            MidiMessage::note_type_to_frequency(MidiNote::C4)
        );
        oscillator.set_frequency_modulation(0.0);
        assert_eq!(
            oscillator.adjusted_frequency(),
            MidiMessage::note_type_to_frequency(MidiNote::C4)
        );
        oscillator.set_frequency_modulation(1.0);
        assert_eq!(
            oscillator.adjusted_frequency(),
            MidiMessage::note_type_to_frequency(MidiNote::C4) * 2.0
        );
        oscillator.set_frequency_modulation(-1.0);
        assert_eq!(
            oscillator.adjusted_frequency(),
            MidiMessage::note_type_to_frequency(MidiNote::C4) / 2.0
        );
        oscillator.set_frequency_modulation(0.5);
        assert_eq!(
            oscillator.adjusted_frequency(),
            MidiMessage::note_type_to_frequency(MidiNote::C4) * 2.0f32.sqrt()
        );
    }
}
