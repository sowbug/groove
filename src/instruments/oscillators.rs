use crate::{
    common::MonoSample,
    messages::EntityMessage,
    settings::patches::{LfoPreset, OscillatorSettings, WaveformType},
    traits::{HasUid, IsInstrument, Response, SourcesAudio, Updateable},
    Clock,
};
use std::f32::consts::PI;
use strum_macros::{Display, EnumString, FromRepr};

#[derive(Display, Debug, EnumString, FromRepr)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum OscillatorControlParams {
    // TODO: it's implied that this is 0.0f32..=1.0f32, which doesn't make a
    // whole lot of sense for something that should be in Hz and range
    // ~10f32..22050f32
    Frequency,
}

#[derive(Clone, Debug)]
pub struct Oscillator {
    uid: usize,

    waveform: WaveformType,

    // Hertz. Any positive number. 440 = A4
    frequency: f32,

    // if not zero, then ignores the `frequency` field and uses this one instead.
    fixed_frequency: f32,

    // 1.0 is no change. 2.0 doubles the frequency. 0.5 halves it. Designed for pitch correction at construction time.
    frequency_tune: f32,

    // [-1, 1] is typical range, with -1 halving the frequency, and 1 doubling it. Designed for LFO and frequent changes.
    frequency_modulation: f32,

    noise_x1: u32,
    noise_x2: u32,
}
impl IsInstrument for Oscillator {}
impl SourcesAudio for Oscillator {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        let phase_normalized = (self.adjusted_frequency() * clock.seconds()) as MonoSample;
        match self.waveform {
            WaveformType::None => 0.0,
            // https://en.wikipedia.org/wiki/Sine_wave
            WaveformType::Sine => (phase_normalized * 2.0 * PI).sin(),
            // https://en.wikipedia.org/wiki/Square_wave
            //Waveform::Square => (phase_normalized * 2.0 * PI).sin().signum(),
            WaveformType::Square => (0.5 - (phase_normalized - phase_normalized.floor())).signum(),
            WaveformType::PulseWidth(duty_cycle) => (duty_cycle as MonoSample
                - (phase_normalized - phase_normalized.floor()))
            .signum() as MonoSample,
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
                let tmp = 2.0 * (self.noise_x2 as MonoSample - (u32::MAX as MonoSample / 2.0))
                    / u32::MAX as MonoSample;
                (self.noise_x2, _) = self.noise_x2.overflowing_add(self.noise_x1);
                tmp
            }
        }
    }
}
impl Updateable for Oscillator {
    type Message = EntityMessage;

    fn update(&mut self, _clock: &Clock, message: Self::Message) -> Response<Self::Message> {
        // Oscillators just oscillate. For now, at least, we'll leave any
        // control like MIDI to the owning instrument. Otherwise, we just emit
        // sound nonstop.
        if let Self::Message::UpdateF32(param_id, value) = message {
            if let Some(param) = OscillatorControlParams::from_repr(param_id) {
                match param {
                    OscillatorControlParams::Frequency => self.set_frequency(value),
                }
            }
        }
        Response::none()
    }
}
impl HasUid for Oscillator {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}

impl Default for Oscillator {
    fn default() -> Self {
        Self {
            // See the _pola test. I kept running into non-bugs where I had a
            // default oscillator in a chain, and wasted time debugging why the
            // output was silent. The answer was that a default oscillator with
            // waveform None and frequency 0.0 is indeed silent.
            //
            // One view is that a default oscillator should be quiet. Another view
            // is that a quiet oscillator isn't doing its main job of helping make
            // sound. Principle of Least Astonishment prevails.
            uid: usize::default(),

            waveform: WaveformType::Sine,
            frequency: 440.0,
            fixed_frequency: 0.0,
            frequency_tune: 1.0,
            frequency_modulation: 0.0,
            noise_x1: 0x70f4f854,
            noise_x2: 0xe1e9f0a7,
        }
    }
}

impl Oscillator {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn new_with(waveform: WaveformType) -> Self {
        Self {
            waveform,
            ..Default::default()
        }
        // TODO: assert that if PWM, range is (0.0, 0.5). 0.0 is None, and 0.5 is Square.
    }

    pub fn new_from_preset(preset: &OscillatorSettings) -> Self {
        Self {
            waveform: preset.waveform,
            frequency_tune: preset.tune,
            ..Default::default()
        }
    }

    pub fn new_lfo(lfo_preset: &LfoPreset) -> Self {
        Self {
            waveform: lfo_preset.waveform,
            frequency: lfo_preset.frequency,
            ..Default::default()
        }
    }

    pub(crate) fn new_with_type_and_frequency(waveform: WaveformType, frequency: f32) -> Self {
        Self {
            waveform,
            frequency,
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

    pub fn waveform(&self) -> WaveformType {
        self.waveform
    }

    pub fn set_waveform(&mut self, waveform: WaveformType) {
        self.waveform = waveform;
    }

    pub fn frequency_modulation(&self) -> f32 {
        self.frequency_modulation
    }

    pub fn frequency(&self) -> f32 {
        self.frequency
    }
}

#[cfg(test)]
mod tests {
    use super::{Oscillator, WaveformType};
    use crate::{
        clock::Clock,
        midi::{MidiNote, MidiUtils},
        settings::patches::OscillatorSettings,
        traits::SourcesAudio,
    };

    fn create_oscillator(waveform: WaveformType, tune: f32, note: MidiNote) -> Oscillator {
        let mut oscillator = Oscillator::new_from_preset(&OscillatorSettings {
            waveform,
            tune,
            ..Default::default()
        });
        oscillator.set_frequency(MidiUtils::note_type_to_frequency(note));
        oscillator
    }

    #[test]
    fn test_oscillator_pola() {
        let mut oscillator = Oscillator::default();
        let mut clock = Clock::default();
        clock.tick(); // in case the oscillator happens to start at zero
        assert_ne!(0.0, oscillator.source_audio(&clock));
    }

    // #[test]
    // fn test_oscillator_tuned() {
    //     let mut oscillator = create_oscillator(
    //         WaveformType::Sine,
    //         OscillatorSettings::octaves(0.0),
    //         MidiNote::C4,
    //     );
    //     assert_eq!(
    //         oscillator.adjusted_frequency(),
    //         MidiUtils::note_type_to_frequency(MidiNote::C4)
    //     );
    //     write_source_to_file(&mut oscillator, "oscillator_sine_c4_plus_zero_octave");

    //     let mut oscillator = create_oscillator(
    //         WaveformType::Sine,
    //         OscillatorSettings::octaves(1.0),
    //         MidiNote::C4,
    //     );
    //     assert_eq!(
    //         oscillator.adjusted_frequency(),
    //         MidiUtils::note_type_to_frequency(MidiNote::C4) * 2.0
    //     );
    //     write_source_to_file(&mut oscillator, "oscillator_sine_c4_plus_1_octave");

    //     let mut oscillator = create_oscillator(
    //         WaveformType::Sine,
    //         OscillatorSettings::octaves(-1.0),
    //         MidiNote::C4,
    //     );
    //     assert_eq!(
    //         oscillator.adjusted_frequency(),
    //         MidiUtils::note_type_to_frequency(MidiNote::C4) / 2.0
    //     );
    //     write_source_to_file(&mut oscillator, "oscillator_sine_c4_minus_1_octave");

    //     let mut oscillator = create_oscillator(
    //         WaveformType::Sine,
    //         OscillatorSettings::semis_and_cents(12.0, 0.0),
    //         MidiNote::C4,
    //     );
    //     assert_eq!(
    //         oscillator.adjusted_frequency(),
    //         MidiUtils::note_type_to_frequency(MidiNote::C4) * 2.0
    //     );
    //     write_source_to_file(&mut oscillator, "oscillator_sine_c4_plus_12_semitone");

    //     let mut oscillator = create_oscillator(
    //         WaveformType::Sine,
    //         OscillatorSettings::semis_and_cents(0.0, -1200.0),
    //         MidiNote::C4,
    //     );
    //     assert_eq!(
    //         oscillator.adjusted_frequency(),
    //         MidiUtils::note_type_to_frequency(MidiNote::C4) / 2.0
    //     );
    //     write_source_to_file(&mut oscillator, "oscillator_sine_c4_minus_1200_cents");
    // }

    #[test]
    fn test_oscillator_modulated() {
        let mut oscillator = create_oscillator(
            WaveformType::Sine,
            OscillatorSettings::octaves(0.0),
            MidiNote::C4,
        );
        assert_eq!(
            oscillator.adjusted_frequency(),
            MidiUtils::note_type_to_frequency(MidiNote::C4)
        );
        oscillator.set_frequency_modulation(0.0);
        assert_eq!(
            oscillator.adjusted_frequency(),
            MidiUtils::note_type_to_frequency(MidiNote::C4)
        );
        oscillator.set_frequency_modulation(1.0);
        assert_eq!(
            oscillator.adjusted_frequency(),
            MidiUtils::note_type_to_frequency(MidiNote::C4) * 2.0
        );
        oscillator.set_frequency_modulation(-1.0);
        assert_eq!(
            oscillator.adjusted_frequency(),
            MidiUtils::note_type_to_frequency(MidiNote::C4) / 2.0
        );
        oscillator.set_frequency_modulation(0.5);
        assert_eq!(
            oscillator.adjusted_frequency(),
            MidiUtils::note_type_to_frequency(MidiNote::C4) * 2.0f32.sqrt()
        );
    }
}
