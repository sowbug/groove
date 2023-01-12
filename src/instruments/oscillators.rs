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

    // It's important for us to remember the "cursor" in the current waveform,
    // because the frequency can change over time, so recalculating the position
    // as if the current frequency were always the frequency leads to click,
    // pops, transients, and suckage.
    cycle_position: f64,

    delta: f64,
    delta_needs_update: bool,

    // Whether this oscillator's owner should sync other oscillators to this
    // one. IMPORTANT! Because we return the sample for the current state but
    // calculate the next sample, all in the same source_audio(), we're also
    // returning the should_sync value for the next clock tick. That's why this
    // field has the elaborate name. Be careful when you're using this, or else
    // you'll sync your synced oscillators one sample too early.
    should_sync_after_this_sample: bool,

    // If this is a synced oscillator, then whether we should reset our waveform
    // to the start.
    is_sync_pending: bool,
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
            cycle_position: 0.0,
            delta: 0.0,
            delta_needs_update: true,
            should_sync_after_this_sample: false,
            is_sync_pending: false,
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
        self.delta_needs_update = true;
    }

    pub(crate) fn set_fixed_frequency(&mut self, frequency: f32) {
        self.fixed_frequency = frequency as f64;
        self.delta_needs_update = true;
    }

    pub(crate) fn set_frequency_modulation(&mut self, frequency_modulation: f32) {
        self.frequency_modulation = frequency_modulation as f64;
        self.delta_needs_update = true;
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

    pub fn should_sync_after_this_sample(&self) -> bool {
        self.should_sync_after_this_sample
    }

    pub(crate) fn sync(&mut self) {
        self.is_sync_pending = true;
    }

    fn check_for_clock_reset(&mut self, clock: &Clock) {
        if clock.was_reset() {
            self.update_delta(clock);

            self.cycle_position = (self.delta * clock.samples() as f64).fract();
        }
    }

    fn update_delta(&mut self, clock: &Clock) {
        if self.delta_needs_update {
            self.delta = self.adjusted_frequency() / clock.sample_rate() as f64;
            self.delta_needs_update = false;
        }
    }

    fn calculate_cycle_position(&mut self, clock: &Clock) -> f64 {
        self.update_delta(clock);

        // Process any sync() calls since last tick. The point of sync() is to
        // restart the synced oscillator's cycle, so position zero is correct.
        //
        // Note that if the clock is reset, then synced oscillators will
        // momentarily have the wrong cycle_position, because in their own
        // check_for_clock_reset() they'll calculate a position, but then in
        // this method they'll detect that they're supposed to sync and will
        // reset to zero. This also means that for one cycle, the main
        // oscillator will have started at a synthetic starting point, but the
        // synced ones will have started at zero. I don't think this is
        // important.
        if self.is_sync_pending {
            self.is_sync_pending = false;
            self.cycle_position = 0.0;
        }

        // Add that to the previous position and mod 1.0.
        let next_cycle_position = (self.cycle_position + self.delta).fract();
        // assert_eq!(
        //     next_cycle_position,
        //     ((1 + clock.samples()) as f64 * self.delta).fract(),
        //     "failed at sample {}",
        //     clock.samples()
        // );

        // Should we signal to synced oscillators that it's time to sync?
        self.should_sync_after_this_sample = next_cycle_position < self.cycle_position;

        // On the first call to this method, clock and self.cycle_position are
        // assumed to be set to the start (zero). All the change calculations
        // are therefore for the *next* result. So we need to save the start
        // value, as that's what we'll report at the end of this method.
        let cycle_position = self.cycle_position;
        self.cycle_position = next_cycle_position;
        cycle_position
    }

    // https://en.wikipedia.org/wiki/Sine_wave
    // https://en.wikipedia.org/wiki/Square_wave
    // https://en.wikipedia.org/wiki/Triangle_wave
    // https://en.wikipedia.org/wiki/Sawtooth_wave
    // https://www.musicdsp.org/en/latest/Synthesis/216-fast-whitenoise-generator.html
    fn amplitude_for_position(&mut self, waveform: &WaveformType, cycle_position: f64) -> f64 {
        match waveform {
            WaveformType::None => 0.0,
            WaveformType::Sine => (cycle_position * 2.0 * PI).sin(),
            WaveformType::Square => (0.5 - cycle_position).signum(),
            WaveformType::PulseWidth(duty_cycle) => (*duty_cycle as f64 - cycle_position).signum(),
            WaveformType::Triangle => {
                4.0 * (cycle_position - (0.5 + cycle_position).floor()).abs() - 1.0
            }
            WaveformType::Sawtooth => 2.0 * (cycle_position - (0.5 + cycle_position).floor()),
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

        // we'll run one tick in case the oscillator happens to start at zero
        oscillator.source_audio(&clock);
        clock.tick();

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
            clock.tick();
            if f == 1.0 {
                n_pos += 1;
            } else if f == -1.0 {
                n_neg += 1;
            } else {
                panic!("square wave emitted strange amplitude: {f}");
            }
            if f != last_sample {
                transitions += 1;
                last_sample = f;
            }
        }
        assert_eq!(n_pos + n_neg, SAMPLE_RATE);
        assert_eq!(n_pos, n_neg);

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
        clock.tick();

        // Halfway between the first and second cycle, the wave should
        // transition from 1.0 to -1.0.
        //
        // We're fast-forwarding two different ways in this test. The first is
        // by just ticking the clock the desired number of times, so we're not
        // really fast-forwarding; we're just playing normally and ignoring the
        // results. The second is by testing that the oscillator responds
        // reasonably to clock.set_samples(). I haven't decided whether entities
        // need to pay close attention to clock.set_samples() other than not
        // exploding, so I might end up deleting that part of the test.
        for t in 1..SAMPLE_RATE / 4 - 2 {
            assert_eq!(t, clock.samples());
            oscillator.source_audio(&clock);
            clock.tick();
        }
        assert_eq!(oscillator.source_audio(&clock), 1.0);
        clock.tick();
        assert_eq!(oscillator.source_audio(&clock), 1.0);
        clock.tick();
        assert_eq!(oscillator.source_audio(&clock), -1.0);
        clock.tick();
        assert_eq!(oscillator.source_audio(&clock), -1.0);

        // Then should transition back to 1.0 at the first sample of the second
        // cycle.
        //
        // As noted above, we're using clock.set_samples() here.
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
        // Default
        assert_eq!(
            oscillator.adjusted_frequency(),
            MidiUtils::note_type_to_frequency(MidiNote::C4) as f64
        );

        // Explicitly zero (none)
        oscillator.set_frequency_modulation(0.0);
        assert_eq!(
            oscillator.adjusted_frequency(),
            MidiUtils::note_type_to_frequency(MidiNote::C4) as f64
        );

        // Max
        oscillator.set_frequency_modulation(1.0);
        assert_eq!(
            oscillator.adjusted_frequency(),
            MidiUtils::note_type_to_frequency(MidiNote::C5) as f64
        );

        // Min
        oscillator.set_frequency_modulation(-1.0);
        assert_eq!(
            oscillator.adjusted_frequency(),
            MidiUtils::note_type_to_frequency(MidiNote::C3) as f64
        );

        // Halfway between zero and max
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

        // On init, the oscillator should NOT flag that any init/reset work
        // needs to happen. We assume that synced oscillators can take care of
        // their own init.
        assert!(!oscillator.should_sync_after_this_sample());

        // Now run through and see that we're flagging cycle start at the right
        // time. Note the = in the for loop range; we're expecting a flag at the
        // zeroth sample of each cycle.
        const LAST_ITERATION_IN_LOOP: usize = TICKS_IN_CYCLE - 1;
        for tick in 0..=TICKS_IN_CYCLE {
            assert_eq!(tick, clock.samples());
            oscillator.source_audio(&clock);
            clock.tick();

            // I don't like the usability of should_sync_after_this_sample(),
            // because it requires an unnatural ordering of its handling. It
            // works for now, but I might want to rework it.
            let expected = match tick {
                0 => false,                     // zeroth sample of first cycle
                LAST_ITERATION_IN_LOOP => true, // zeroth sample of second cycle
                _ => false,
            };
            assert_eq!(
                oscillator.should_sync_after_this_sample(),
                expected,
                "expected {expected} at sample #{tick}"
            );
        }

        // Let's try again after rewinding the clock. It should recognize
        // something happened and restart the cycle. First we confirm that it
        // thinks it's midway through the cycle.
        oscillator.source_audio(&clock);
        assert!(!oscillator.should_sync_after_this_sample());

        // Then we actually change the clock. We'll pick something we know is
        // off-cycle. We don't treat this as a should-sync event, because we
        // assume that synced oscillators will also notice the clock change and
        // do the right thing. At worst, we'll be off for a single main
        // oscillator cycle. No normal audio performance will involve a clock
        // shift, so it's OK to have the wrong timbre for a tiny fraction of a
        // second.
        clock.set_samples(3);
        oscillator.source_audio(&clock);
        assert!(!oscillator.should_sync_after_this_sample());

        // Let's run through again, but this time go for a whole second, and
        // count the number of flags.
        clock.reset();
        let mut cycles = 0;
        for _ in 0..SAMPLE_RATE {
            oscillator.source_audio(&clock);
            clock.tick();
            if oscillator.should_sync_after_this_sample() {
                cycles += 1;
            }
        }
        assert_eq!(cycles, FREQUENCY as usize);
    }
}
