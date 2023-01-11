use crate::{
    common::{F32ControlValue, MonoSample},
    messages::EntityMessage,
    settings::patches::{LfoPreset, OscillatorSettings, WaveformType},
    traits::{Controllable, HasUid, IsInstrument, SourcesAudio, Updateable},
    Clock,
};
use groove_macros::{Control, Uid};
use std::f64::consts::PI;
use std::str::FromStr;
use strum_macros::{Display, EnumString, FromRepr};

#[derive(Clone, Control, Debug, Uid)]
pub struct Oscillator {
    uid: usize,

    waveform: WaveformType,

    /// Hertz. Any positive number. 440 = A4
    frequency: f64,

    /// if not zero, then ignores the `frequency` field and uses this one
    /// instead.
    fixed_frequency: f64,

    /// 1.0 is no change. 2.0 doubles the frequency. 0.5 halves it. Designed for
    /// pitch correction at construction time.
    frequency_tune: f64,

    /// [-1, 1] is typical range, with -1 halving the frequency, and 1 doubling
    /// it. Designed for LFO and frequent changes.
    frequency_modulation: f64,

    /// 0..1.0: volume
    mix: f64,

    // working variables to generate semi-deterministic noise.
    noise_x1: u32,
    noise_x2: u32,

    // if this oscillator is synced, then it's the owner's job to set this
    // correctly before calling source_audio(). Otherwise it's calculated
    // internally.
    last_cycle_position: f64,
    // if this oscillator is not synced, then this is set true after a
    // source_audio() in which position_in_cycle's fractional component
    // overflows (e.g., from 0.99 to 1.01).
    has_cycle_restarted: bool,

    is_sync_pending: bool,
    cycle_origin: usize,
}
impl IsInstrument for Oscillator {}
impl SourcesAudio for Oscillator {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        self.check_for_clock_reset(clock);
        let cycle_position = self.calculate_cycle_position(clock);
        let waveform_type = self.waveform;
        let amplitude = self.mix * self.amplitude_for_position(&waveform_type, cycle_position);
        amplitude as f32
    }
}
impl Updateable for Oscillator {
    type Message = EntityMessage;
}
impl Default for Oscillator {
    fn default() -> Self {
        Self {
            // See the _pola test. I kept running into non-bugs where I had a
            // default oscillator in a chain, and wasted time debugging why the
            // output was silent. The answer was that a default oscillator with
            // waveform None and frequency 0.0 is indeed silent.
            //
            // One view is that a default oscillator should be quiet. Another
            // view is that a quiet oscillator isn't doing its main job of
            // helping make sound. Principle of Least Astonishment prevails.
            uid: usize::default(),

            waveform: WaveformType::Sine,
            mix: 1.0,
            frequency: 440.0,
            fixed_frequency: 0.0,
            frequency_tune: 1.0,
            frequency_modulation: 0.0,
            noise_x1: 0x70f4f854,
            noise_x2: 0xe1e9f0a7,
            last_cycle_position: 0.0,
            has_cycle_restarted: true,
            is_sync_pending: false,
            cycle_origin: 0,
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
        // TODO: assert that if PWM, range is (0.0, 0.5). 0.0 is None, and 0.5
        // is Square.
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

    pub fn has_cycle_restarted(&self) -> bool {
        self.has_cycle_restarted
    }

    pub(crate) fn sync(&mut self) {
        self.is_sync_pending = true;
    }

    fn check_for_clock_reset(&mut self, clock: &Clock) {
        if clock.was_reset() {
            self.cycle_origin = 0;
            self.last_cycle_position = 0.0;
            self.has_cycle_restarted = true;
        } else {
            self.has_cycle_restarted = false;
        }
    }

    fn calculate_cycle_position(&mut self, clock: &Clock) -> f64 {
        if self.is_sync_pending {
            self.is_sync_pending = false;
            self.cycle_origin = clock.samples();
        }
        let position_in_time = (clock.samples() - self.cycle_origin) as f64
            * self.adjusted_frequency()
            / clock.sample_rate() as f64;
        let position_in_cycle = position_in_time.fract();
        if position_in_cycle < self.last_cycle_position {
            self.has_cycle_restarted = true;
        }
        self.last_cycle_position = position_in_cycle;
        position_in_cycle
    }

    fn amplitude_for_position(&mut self, waveform: &WaveformType, cycle_position: f64) -> f64 {
        match waveform {
            WaveformType::None => 0.0,
            // https://en.wikipedia.org/wiki/Sine_wave
            WaveformType::Sine => (cycle_position * 2.0 * PI).sin(),
            // https://en.wikipedia.org/wiki/Square_wave Waveform::Square =>
            //(phase_normalized * 2.0 * PI).sin().signum(),
            WaveformType::Square => {
                if cycle_position < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            WaveformType::PulseWidth(duty_cycle) => {
                if cycle_position < *duty_cycle as f64 {
                    1.0
                } else {
                    -1.0
                }
            }
            // https://en.wikipedia.org/wiki/Triangle_wave
            WaveformType::Triangle => {
                4.0 * (cycle_position - (0.5 + cycle_position).floor()).abs() - 1.0
            }
            // https://en.wikipedia.org/wiki/Sawtooth_wave
            WaveformType::Sawtooth => 2.0 * (cycle_position - (0.5 + cycle_position).floor()),
            // https://www.musicdsp.org/en/latest/Synthesis/216-fast-whitenoise-generator.html
            WaveformType::Noise => {
                // TODO: this is stateful, so random access will sound different
                // from sequential, as will different sample rates. It also
                // makes this method require mut. Is there a noise algorithm
                // that can modulate on time_seconds? (It's a complicated
                // question, potentially.)
                self.noise_x1 ^= self.noise_x2;
                let tmp = 2.0 * (self.noise_x2 as f64 - (u32::MAX as f64 / 2.0)) / u32::MAX as f64;
                (self.noise_x2, _) = self.noise_x2.overflowing_add(self.noise_x1);
                tmp
            }
            // TODO: figure out whether this was an either-or
            WaveformType::TriangleSine => {
                4.0 * (cycle_position - (0.75 + cycle_position).floor() + 0.25).abs() - 1.0
            }
        }
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
    fn oscillator_modulated() {
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

    #[test]
    fn oscillator_cycle_restarts_on_time() {
        let mut clock = Clock::default();
        let mut oscillator = Oscillator::default();
        const FREQUENCY: f32 = 2.0;
        const SAMPLE_RATE: usize = 44100;
        oscillator.set_frequency(FREQUENCY);

        // We're hardcoding 44.1 so we can use a const in the match below.
        const TICKS_IN_CYCLE: usize = SAMPLE_RATE / FREQUENCY as usize;

        // Before any work happens, the oscillator should flag that any
        // init/reset work needs to happen.
        assert!(oscillator.has_cycle_restarted());

        // Now run through and see that we're flagging cycle start at the right
        // time. Note the = in the for loop range; we're expecting a flag at the
        // zeroth sample of each cycle.
        for tick in 0..=TICKS_IN_CYCLE {
            oscillator.source_audio(&clock);
            clock.tick();

            let expected = match tick {
                0 => true,              // zeroth sample of first cycle
                TICKS_IN_CYCLE => true, // zeroth sample of second cycle
                _ => false,
            };
            assert_eq!(
                oscillator.has_cycle_restarted(),
                expected,
                "expected {expected} at sample #{tick}"
            );
        }

        // Let's try again after rewinding the clock. It should recognize
        // something happened and restart the cycle. First we confirm that it
        // thinks it's midway through the cycle.
        oscillator.source_audio(&clock);
        assert!(!oscillator.has_cycle_restarted());
        // Then we actually change the clock. We'll pick something we know is
        // off-cycle. There is a decision to make whether we consider this a
        // cycle restart or not. Until proven otherwise we're going to say it
        // is. The reason is that the whole concept of cycle-restarting is for
        // syncing a secondary oscillator, so we're allowing the secondary to
        // stay in sync even though it's a short cycle.
        clock.set_samples(3);
        oscillator.source_audio(&clock);
        assert!(oscillator.has_cycle_restarted());

        // Let's run through again, but this time go for a whole second, and
        // count the number of flags.
        clock.reset();
        let mut cycles = 0;
        for _ in 0..SAMPLE_RATE {
            oscillator.source_audio(&clock);
            clock.tick();
            if oscillator.has_cycle_restarted() {
                cycles += 1;
            }
        }
        assert_eq!(cycles, FREQUENCY as usize);
    }
}
