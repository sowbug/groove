use crate::{
    common::MonoSample,
    messages::EntityMessage,
    settings::patches::{LfoPreset, OscillatorSettings, WaveformType},
    traits::{HasUid, IsInstrument, Response, SourcesAudio, Updateable},
    Clock,
};
use groove_macros::Uid;
use std::f64::consts::PI;
use strum_macros::{Display, EnumString, FromRepr};

#[derive(Display, Debug, EnumString, FromRepr)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum OscillatorControlParams {
    // TODO: it's implied that this is 0.0f32..=1.0f32, which doesn't make a
    // whole lot of sense for something that should be in Hz and range
    // ~10f32..22050f32
    Frequency,
}

#[derive(Clone, Debug, Uid)]
pub struct Oscillator {
    uid: usize,

    waveform: WaveformType,

    /// Hertz. Any positive number. 440 = A4
    frequency: f64,

    /// if not zero, then ignores the `frequency` field and uses this one instead.
    fixed_frequency: f64,

    /// 1.0 is no change. 2.0 doubles the frequency. 0.5 halves it. Designed for pitch correction at construction time.
    frequency_tune: f64,

    /// [-1, 1] is typical range, with -1 halving the frequency, and 1 doubling it. Designed for LFO and frequent changes.
    frequency_modulation: f64,

    /// 0..1.0: volume
    mix: f64,

    noise_x1: u32,
    noise_x2: u32,

    /// An offset used to sync a secondary oscillator with a primary.
    phase_shift: usize,

    /// Whether a primary oscillator has begun a new period since the last source_audio()
    has_period_restarted: bool,
    last_cycle_position: f64,
}
impl IsInstrument for Oscillator {}
impl SourcesAudio for Oscillator {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        if clock.was_reset() {
            self.phase_shift = 0;
            self.last_cycle_position = 0.0;
        }
        let sample_count = (clock.samples() - self.phase_shift) as f64;
        let sample_rate = clock.sample_rate() as f64;
        let samples_at_frequency = sample_count * self.adjusted_frequency();
        let position = samples_at_frequency / sample_rate;
        let position_in_cycle = position.fract();
        self.has_period_restarted = position_in_cycle < self.last_cycle_position;
        self.last_cycle_position = position_in_cycle;

        let amplitude = self.mix
            * match self.waveform {
                WaveformType::None => 0.0,
                // https://en.wikipedia.org/wiki/Sine_wave
                WaveformType::Sine => (position_in_cycle * 2.0 * PI).sin(),
                // https://en.wikipedia.org/wiki/Square_wave
                //Waveform::Square => (phase_normalized * 2.0 * PI).sin().signum(),
                WaveformType::Square => {
                    if position_in_cycle < 0.5 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                WaveformType::PulseWidth(duty_cycle) => {
                    if position_in_cycle < duty_cycle as f64 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                // https://en.wikipedia.org/wiki/Triangle_wave
                WaveformType::Triangle => {
                    4.0 * (position_in_cycle - (0.5 + position_in_cycle).floor()).abs() - 1.0
                }
                // https://en.wikipedia.org/wiki/Sawtooth_wave
                WaveformType::Sawtooth => {
                    2.0 * (position_in_cycle - (0.5 + position_in_cycle).floor())
                }
                // https://www.musicdsp.org/en/latest/Synthesis/216-fast-whitenoise-generator.html
                WaveformType::Noise => {
                    // TODO: this is stateful, so random access will sound different from sequential, as will different sample rates.
                    // It also makes this method require mut. Is there a noise algorithm that can modulate on time_seconds? (It's a
                    // complicated question, potentially.)
                    self.noise_x1 ^= self.noise_x2;
                    let tmp =
                        2.0 * (self.noise_x2 as f64 - (u32::MAX as f64 / 2.0)) / u32::MAX as f64;
                    (self.noise_x2, _) = self.noise_x2.overflowing_add(self.noise_x1);
                    tmp
                }
                // TODO: figure out whether this was an either-or
                WaveformType::TriangleSine => {
                    4.0 * (position_in_cycle - (0.75 + position_in_cycle).floor() + 0.25).abs()
                        - 1.0
                }
            };
        amplitude as f32
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
            mix: 1.0,
            frequency: 440.0,
            fixed_frequency: 0.0,
            frequency_tune: 1.0,
            frequency_modulation: 0.0,
            noise_x1: 0x70f4f854,
            noise_x2: 0xe1e9f0a7,
            phase_shift: 0,
            has_period_restarted: true,
            last_cycle_position: 0.0,
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
            mix: preset.mix as f64,
            frequency_tune: preset.tune.into(),
            ..Default::default()
        }
    }

    pub fn new_lfo(lfo_preset: &LfoPreset) -> Self {
        Self {
            waveform: lfo_preset.waveform,
            frequency: lfo_preset.frequency as f64,
            ..Default::default()
        }
    }

    pub(crate) fn new_with_type_and_frequency(waveform: WaveformType, frequency: f32) -> Self {
        Self {
            waveform,
            frequency: frequency as f64,
            ..Default::default()
        }
    }

    fn adjusted_frequency(&self) -> f64 {
        let tmp = if self.fixed_frequency == 0.0 {
            self.frequency * (self.frequency_tune)
        } else {
            self.fixed_frequency
        };
        tmp * 2.0f64.powf(self.frequency_modulation)
    }

    pub(crate) fn set_frequency(&mut self, frequency: f32) {
        self.frequency = frequency as f64;
    }

    pub(crate) fn set_fixed_frequency(&mut self, frequency: f32) {
        self.fixed_frequency = frequency as f64;
    }

    pub(crate) fn set_frequency_modulation(&mut self, frequency_modulation: f32) {
        self.frequency_modulation = frequency_modulation as f64;
    }

    pub fn waveform(&self) -> WaveformType {
        self.waveform
    }

    pub fn set_waveform(&mut self, waveform: WaveformType) {
        self.waveform = waveform;
    }

    pub fn frequency_modulation(&self) -> f32 {
        self.frequency_modulation as f32
    }

    pub fn frequency(&self) -> f32 {
        self.frequency as f32
    }

    pub fn sync(&mut self, clock: &Clock) {
        self.has_period_restarted = true;
        self.phase_shift = clock.samples();
    }

    pub fn has_period_restarted(&self) -> bool {
        self.has_period_restarted
    }
}

#[cfg(test)]
mod tests {
    use more_asserts::assert_lt;

    use super::{Oscillator, WaveformType};
    use crate::{
        clock::Clock,
        controllers::orchestrator::tests::TestOrchestrator,
        midi::{MidiNote, MidiUtils},
        settings::patches::{OscillatorSettings, OscillatorTune},
        traits::SourcesAudio,
        utils::tests::samples_match_known_good_wav_file,
        EntityMessage, Paths, Timer,
    };

    fn create_oscillator(
        waveform: WaveformType,
        tune: OscillatorTune,
        note: MidiNote,
    ) -> Oscillator {
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

    // Make sure we're dealing with at least a pulse-width wave of amplitude
    // 1.0, which means that every value is either 1.0 or -1.0.
    #[test]
    fn test_square_wave_is_correct_amplitude() {
        const SAMPLE_RATE: usize = 63949; // Prime number
        const FREQUENCY: f32 = 499.0;
        let mut oscillator =
            Oscillator::new_with_type_and_frequency(WaveformType::Square, FREQUENCY);
        let mut clock = Clock::new_with_sample_rate(SAMPLE_RATE);

        // Below Nyquist limit
        assert_lt!(FREQUENCY, (SAMPLE_RATE / 2) as f32);

        for _ in 0..clock.sample_rate() {
            let f = oscillator.source_audio(&clock);
            assert_eq!(f, f.signum());
            clock.tick();
        }
    }

    #[test]
    fn test_square_wave_frequency_is_accurate() {
        // For this test, we want the sample rate and frequency to be nice even
        // numbers so that we don't have to deal with edge cases.
        const SAMPLE_RATE: usize = 65536;
        const FREQUENCY: f32 = 128.0;
        let mut oscillator =
            Oscillator::new_with_type_and_frequency(WaveformType::Square, FREQUENCY);
        let mut clock = Clock::new_with_sample_rate(SAMPLE_RATE);

        let mut n_pos = 0;
        let mut n_neg = 0;
        let mut last_sample = 1.0;
        let mut transitions = 0;
        for _ in 0..clock.sample_rate() {
            let f = oscillator.source_audio(&clock);
            if f > 0.0 {
                n_pos += 1;
            } else {
                n_neg += 1;
            }
            if f != last_sample {
                transitions += 1;
                last_sample = f;
            }
            clock.tick();
        }
        assert_eq!(n_pos, n_neg);
        assert_eq!(n_pos + n_neg, SAMPLE_RATE);

        // The -1 is because we stop at the end of the cycle, and the transition
        // back to 1.0 should be at the start of the next cycle.
        assert_eq!(transitions, FREQUENCY as i32 * 2 - 1);
    }

    #[test]
    fn test_square_wave_shape_is_accurate() {
        const SAMPLE_RATE: usize = 65536;
        const FREQUENCY: f32 = 2.0;
        let mut oscillator =
            Oscillator::new_with_type_and_frequency(WaveformType::Square, FREQUENCY);

        // The first sample should be 1.0.
        let mut clock = Clock::new_with_sample_rate(SAMPLE_RATE);
        assert_eq!(oscillator.source_audio(&clock), 1.0);

        // Halfway between the first and second cycle, the wave should
        // transition from 1.0 to -1.0.
        clock.set_samples(SAMPLE_RATE / 4 - 2);
        assert_eq!(oscillator.source_audio(&clock), 1.0);
        clock.tick();
        assert_eq!(oscillator.source_audio(&clock), 1.0);
        clock.tick();
        assert_eq!(oscillator.source_audio(&clock), -1.0);
        clock.tick();
        assert_eq!(oscillator.source_audio(&clock), -1.0);

        // Then should transition back to 1.0 at the first sample of the second
        // cycle.
        clock.set_samples(SAMPLE_RATE / 2 - 2);
        assert_eq!(oscillator.source_audio(&clock), -1.0);
        clock.tick();
        assert_eq!(oscillator.source_audio(&clock), -1.0);
        clock.tick();
        assert_eq!(oscillator.source_audio(&clock), 1.0);
        clock.tick();
        assert_eq!(oscillator.source_audio(&clock), 1.0);
    }

    #[test]
    fn test_sine_wave_is_balanced() {
        const SAMPLE_RATE: usize = 44100;
        const FREQUENCY: f32 = 1.0;
        let mut oscillator = Oscillator::new_with_type_and_frequency(WaveformType::Sine, FREQUENCY);
        let mut clock = Clock::new_with_sample_rate(SAMPLE_RATE);

        let mut n_pos = 0;
        let mut n_neg = 0;
        let mut n_zero = 0;
        for _ in 0..clock.sample_rate() {
            let f = oscillator.source_audio(&clock);
            if f < -0.0000001 {
                n_neg += 1;
            } else if f > 0.0000001 {
                n_pos += 1;
            } else {
                n_zero += 1;
            }
            clock.tick();
        }
        assert_eq!(n_zero, 2);
        assert_eq!(n_pos, n_neg);
        assert_eq!(n_pos + n_neg + n_zero, SAMPLE_RATE);
    }

    #[test]
    fn square_matches_known_good() {
        let test_cases = vec![
            (1.0, "1Hz"),
            (100.0, "100Hz"),
            (1000.0, "1000Hz"),
            (10000.0, "10000Hz"),
            (20000.0, "20000Hz"),
        ];
        for test_case in test_cases {
            let mut o = TestOrchestrator::default();
            let osc_uid = o.add(
                None,
                crate::BoxedEntity::Oscillator(Box::new(Oscillator::new_with_type_and_frequency(
                    WaveformType::Square,
                    test_case.0,
                ))),
            );
            assert!(o.patch_chain_to_main_mixer(&[osc_uid]).is_ok());
            let _ = o.add(
                None,
                crate::BoxedEntity::Timer(Box::new(Timer::<EntityMessage>::new_with(1.0))),
            );
            let mut clock = Clock::default();
            if let Ok(samples) = o.run(&mut clock) {
                let mut filename = Paths::test_data_path();
                filename.push("audacity");
                filename.push("44100Hz-mono");
                filename.push(format!("square-{}.wav", test_case.1));

                assert!(
                    samples_match_known_good_wav_file(samples, &filename, 0.001),
                    "while testing square {}Hz",
                    test_case.0
                );
            } else {
                panic!("run failed");
            }
        }
    }

    #[test]
    fn sine_matches_known_good() {
        let test_cases = vec![
            (1.0, "1Hz"),
            (100.0, "100Hz"),
            (1000.0, "1000Hz"),
            (10000.0, "10000Hz"),
            (20000.0, "20000Hz"),
        ];
        for test_case in test_cases {
            let mut o = TestOrchestrator::default();
            let osc_uid = o.add(
                None,
                crate::BoxedEntity::Oscillator(Box::new(Oscillator::new_with_type_and_frequency(
                    WaveformType::Sine,
                    test_case.0,
                ))),
            );
            assert!(o.patch_chain_to_main_mixer(&[osc_uid]).is_ok());
            let _ = o.add(
                None,
                crate::BoxedEntity::Timer(Box::new(Timer::<EntityMessage>::new_with(1.0))),
            );
            let mut clock = Clock::default();
            if let Ok(samples) = o.run(&mut clock) {
                let mut filename = Paths::test_data_path();
                filename.push("audacity");
                filename.push("44100Hz-mono");
                filename.push(format!("sine-{}.wav", test_case.1));

                assert!(
                    samples_match_known_good_wav_file(samples, &filename, 0.001),
                    "while testing sine {}Hz",
                    test_case.0
                );
            } else {
                panic!("run failed");
            }
        }
    }

    #[test]
    fn sawtooth_matches_known_good() {
        let test_cases = vec![
            (1.0, "1Hz"),
            (100.0, "100Hz"),
            (1000.0, "1000Hz"),
            (10000.0, "10000Hz"),
            (20000.0, "20000Hz"),
        ];
        for test_case in test_cases {
            let mut o = TestOrchestrator::default();
            let osc_uid = o.add(
                None,
                crate::BoxedEntity::Oscillator(Box::new(Oscillator::new_with_type_and_frequency(
                    WaveformType::Sawtooth,
                    test_case.0,
                ))),
            );
            assert!(o.patch_chain_to_main_mixer(&[osc_uid]).is_ok());
            let _ = o.add(
                None,
                crate::BoxedEntity::Timer(Box::new(Timer::<EntityMessage>::new_with(1.0))),
            );
            let mut clock = Clock::default();
            if let Ok(samples) = o.run(&mut clock) {
                let mut filename = Paths::test_data_path();
                filename.push("audacity");
                filename.push("44100Hz-mono");
                filename.push(format!("sawtooth-{}.wav", test_case.1));

                assert!(
                    samples_match_known_good_wav_file(samples, &filename, 0.001),
                    "while testing sawtooth {}Hz",
                    test_case.0
                );
            } else {
                panic!("run failed");
            }
        }
    }

    #[test]
    fn triangle_matches_known_good() {
        let test_cases = vec![
            (1.0, "1Hz"),
            (100.0, "100Hz"),
            (1000.0, "1000Hz"),
            (10000.0, "10000Hz"),
            (20000.0, "20000Hz"),
        ];
        for test_case in test_cases {
            let mut o = TestOrchestrator::default();
            let osc_uid = o.add(
                None,
                crate::BoxedEntity::Oscillator(Box::new(Oscillator::new_with_type_and_frequency(
                    WaveformType::Triangle,
                    test_case.0,
                ))),
            );
            assert!(o.patch_chain_to_main_mixer(&[osc_uid]).is_ok());
            let _ = o.add(
                None,
                crate::BoxedEntity::Timer(Box::new(Timer::<EntityMessage>::new_with(1.0))),
            );
            let mut clock = Clock::default();
            if let Ok(samples) = o.run(&mut clock) {
                let mut filename = Paths::test_data_path();
                filename.push("audacity");
                filename.push("44100Hz-mono");
                filename.push(format!("triangle-{}.wav", test_case.1));

                assert!(
                    samples_match_known_good_wav_file(samples, &filename, 0.01),
                    "while testing triangle {}Hz",
                    test_case.0
                );
            } else {
                panic!("run failed");
            }
        }
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
            OscillatorTune::Osc {
                octave: 0,
                semi: 0,
                cent: 0,
            },
            MidiNote::C4,
        );
        assert_eq!(
            oscillator.adjusted_frequency(),
            MidiUtils::note_type_to_frequency(MidiNote::C4) as f64
        );
        oscillator.set_frequency_modulation(0.0);
        assert_eq!(
            oscillator.adjusted_frequency(),
            MidiUtils::note_type_to_frequency(MidiNote::C4) as f64
        );
        oscillator.set_frequency_modulation(1.0);
        assert_eq!(
            oscillator.adjusted_frequency(),
            MidiUtils::note_type_to_frequency(MidiNote::C4) as f64 * 2.0
        );
        oscillator.set_frequency_modulation(-1.0);
        assert_eq!(
            oscillator.adjusted_frequency(),
            MidiUtils::note_type_to_frequency(MidiNote::C4) as f64 / 2.0
        );
        oscillator.set_frequency_modulation(0.5);
        assert_eq!(
            oscillator.adjusted_frequency(),
            MidiUtils::note_type_to_frequency(MidiNote::C4) as f64 * 2.0f64.sqrt()
        );
    }
}
