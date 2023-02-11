use crate::{
    common::{F32ControlValue, SignalType},
    settings::patches::{LfoPreset, OscillatorSettings, WaveformType},
    traits::{Controllable, Generates, Ticks},
    BipolarNormal, Normal,
};
use groove_macros::Control;
use more_asserts::debug_assert_lt;
use std::f64::consts::PI;
use std::fmt::Debug;
use std::str::FromStr;
use strum_macros::{Display, EnumString, FromRepr};

/// https://en.wikipedia.org/wiki/Kahan_summation_algorithm
///
/// Given a large number that you want to increase by small numbers, accumulates
/// fewer errors in the running sum than standard f32/f64.
#[derive(Clone, Debug, Default)]
pub(crate) struct KahanSummation<
    T: Copy
        + Default
        + std::ops::Add<Output = T>
        + std::ops::Sub<Output = T>
        + std::ops::Add<U, Output = T>
        + std::ops::Sub<U, Output = T>,
    U: Copy + Default + std::ops::Add<Output = U> + std::ops::Sub<Output = U>,
> {
    sum: T,
    compensation: U,
}
impl<
        T: Copy
            + Default
            + std::ops::Add<Output = T>
            + std::ops::Sub<Output = T>
            + std::ops::Add<U, Output = T>
            + std::ops::Sub<U, Output = T>,
        U: Copy + Default + std::ops::Add<Output = U> + std::ops::Sub<Output = U> + From<T>,
    > KahanSummation<T, U>
{
    pub(crate) fn add(&mut self, rhs: U) -> T {
        let y = rhs - self.compensation;
        let t = self.sum + y;
        self.compensation = U::from((t - self.sum) - y);
        self.sum = t;
        t
    }
    pub(crate) fn current_sum(&self) -> T {
        self.sum
    }
    pub(crate) fn set_sum(&mut self, sum: T) {
        self.sum = sum;
        self.reset_compensation();
    }
    pub(crate) fn reset_compensation(&mut self) {
        self.compensation = Default::default();
    }
}

#[derive(Clone, Control, Debug)]
pub struct Oscillator {
    waveform: WaveformType,

    /// Hertz. Any positive number. 440 = A4
    frequency: SignalType,

    /// if not zero, then ignores the `frequency` field and uses this one
    /// instead.
    fixed_frequency: SignalType,

    /// 1.0 is no change. 2.0 doubles the frequency. 0.5 halves it. Designed for
    /// pitch correction at construction time.
    frequency_tune: SignalType,

    /// [-1, 1] is typical range, with -1 halving the frequency, and 1 doubling
    /// it. Designed for LFO and frequent changes.
    frequency_modulation: SignalType,

    /// 0..1.0: volume
    mix: Normal,

    /// working variables to generate semi-deterministic noise.
    noise_x1: u32,
    noise_x2: u32,

    /// An internal copy of the current sample rate.
    sample_rate: usize,

    /// The internal clock. Advances once per tick().
    ticks: usize,

    signal: SignalType,

    // It's important for us to remember the "cursor" in the current waveform,
    // because the frequency can change over time, so recalculating the position
    // as if the current frequency were always the frequency leads to click,
    // pops, transients, and suckage.
    //
    // Needs Kahan summation algorithm to avoid accumulation of FP errors.
    cycle_position: KahanSummation<f64, f64>,

    delta: f64,
    delta_needs_update: bool,

    // Whether this oscillator's owner should sync other oscillators to this
    // one. Calculated during tick().
    should_sync: bool,

    // If this is a synced oscillator, then whether we should reset our waveform
    // to the start.
    is_sync_pending: bool,

    // Set on init and reset().
    is_reset_pending: bool,
}
impl Generates<SignalType> for Oscillator {
    fn value(&self) -> SignalType {
        self.signal
    }

    #[allow(unused_variables)]
    fn batch_values(&mut self, values: &mut [SignalType]) {
        todo!()
    }
}
impl Ticks for Oscillator {
    fn reset(&mut self, sample_rate: usize) {
        self.is_reset_pending = true;
        self.sample_rate = sample_rate;
    }

    fn tick(&mut self, tick_count: usize) {
        for _ in 0..tick_count {
            if self.is_reset_pending {
                self.ticks = 0; // TODO: this might not be the right thing to do

                self.update_delta();
                self.cycle_position
                    .set_sum((self.delta * self.ticks as f64).fract());
            } else {
                self.ticks += 1;
            }

            let cycle_position = self.calculate_cycle_position();
            let waveform = self.waveform;
            let amplitude_for_position = self.amplitude_for_position(&waveform, cycle_position);
            self.signal = BipolarNormal::from(self.mix.scale(amplitude_for_position)).value();

            // We need this to be at the end of tick() because any code running
            // during tick() might look at it.
            self.is_reset_pending = false;
        }
    }
}

impl Oscillator {
    pub fn new_with(sample_rate: usize) -> Self {
        Self {
            // See the _pola test. I kept running into non-bugs where I had a
            // default oscillator in a chain, and wasted time debugging why the
            // output was silent. The answer was that a default oscillator with
            // waveform None and frequency 0.0 is indeed silent.
            //
            // One view is that a default oscillator should be quiet. Another
            // view is that a quiet oscillator isn't doing its main job of
            // helping make sound. Principle of Least Astonishment prevails.
            waveform: WaveformType::Sine,

            mix: Normal::maximum(),
            frequency: 440.0,
            fixed_frequency: Default::default(),
            frequency_tune: 1.0,
            frequency_modulation: Default::default(),
            noise_x1: 0x70f4f854,
            noise_x2: 0xe1e9f0a7,
            sample_rate,
            ticks: Default::default(),
            signal: Default::default(),
            cycle_position: Default::default(),
            delta: Default::default(),
            delta_needs_update: true,
            should_sync: Default::default(),
            is_sync_pending: Default::default(),
            is_reset_pending: true,
        }
    }

    pub fn new_with_waveform(sample_rate: usize, waveform: WaveformType) -> Self {
        // TODO: assert that if PWM, range is (0.0, 0.5). 0.0 is None, and 0.5
        // is Square.
        let mut r = Self::new_with(sample_rate);
        r.waveform = waveform;
        r
    }

    pub(crate) fn new_with_type_and_frequency(
        sample_rate: usize,
        waveform: WaveformType,
        frequency: f32,
    ) -> Self {
        let mut r = Self::new_with(sample_rate);
        r.waveform = waveform;
        r.frequency = frequency as f64;
        r
    }

    pub fn new_from_preset(sample_rate: usize, preset: &OscillatorSettings) -> Self {
        let mut r = Self::new_with(sample_rate);
        r.waveform = preset.waveform;
        r.mix = Normal::from(preset.mix as f64);
        r.frequency_tune = preset.tune.into();
        r
    }

    pub fn new_lfo(sample_rate: usize, lfo_preset: &LfoPreset) -> Self {
        let mut r = Self::new_with(sample_rate);
        r.waveform = lfo_preset.waveform;
        r.frequency = lfo_preset.frequency as f64;
        r
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

    pub fn should_sync(&self) -> bool {
        self.should_sync
    }

    pub(crate) fn sync(&mut self) {
        self.is_sync_pending = true;
    }

    fn update_delta(&mut self) {
        if self.delta_needs_update {
            self.delta = self.adjusted_frequency() / self.sample_rate as f64;
            self.cycle_position.reset_compensation();
            self.delta_needs_update = false;
        }
    }

    fn calculate_cycle_position(&mut self) -> f64 {
        self.update_delta();

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
            self.cycle_position = Default::default();
        }

        // If we haven't just reset, add delta to the previous position and mod
        // 1.0.
        let next_cycle_position_unrounded = if self.is_reset_pending {
            0.0
        } else {
            self.cycle_position.add(self.delta)
        };

        self.should_sync = if self.is_reset_pending {
            // If we're in the first post-reset tick(), then we want other
            // oscillators to sync.
            true
        } else if next_cycle_position_unrounded > 0.999999999999 {
            // This special case is to deal with an FP precision issue that was
            // causing square waves to flip one sample too late in unit tests. We
            // take advantage of it to also record whether we should signal to
            // synced oscillators that it's time to sync.
            debug_assert_lt!(next_cycle_position_unrounded, 2.0);
            self.cycle_position.add(-1.0);
            true
        } else {
            false
        };

        self.cycle_position.current_sum()
    }

    // https://en.wikipedia.org/wiki/Sine_wave
    // https://en.wikipedia.org/wiki/Square_wave
    // https://en.wikipedia.org/wiki/Triangle_wave
    // https://en.wikipedia.org/wiki/Sawtooth_wave
    // https://www.musicdsp.org/en/latest/Synthesis/216-fast-whitenoise-generator.html
    //
    // Some of these have seemingly arbitrary phase-shift constants in their
    // formulas. The reason for them is to ensure that every waveform starts at
    // amplitude zero, which makes it a lot easier to avoid transients when a
    // waveform starts up. See Pirkle DSSPC++ p.133 for visualization.
    fn amplitude_for_position(&mut self, waveform: &WaveformType, cycle_position: f64) -> f64 {
        match waveform {
            WaveformType::None => 0.0,
            WaveformType::Sine => (cycle_position * 2.0 * PI).sin(),
            WaveformType::Square => -(cycle_position - 0.5).signum(),
            WaveformType::PulseWidth(duty_cycle) => -(cycle_position - *duty_cycle as f64).signum(),
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
            WaveformType::DebugZero => 0.0,
            WaveformType::DebugMax => 1.0,
            WaveformType::DebugMin => -1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Oscillator, WaveformType};
    use crate::{
        common::DEFAULT_SAMPLE_RATE,
        midi::{MidiNote, MidiUtils},
        settings::patches::{OscillatorSettings, OscillatorTune},
        traits::{tests::DebugTicks, Generates, Ticks},
        utils::tests::{render_signal_as_audio_source, samples_match_known_good_wav_file},
        Paths,
    };
    use more_asserts::assert_lt;

    impl DebugTicks for Oscillator {
        fn debug_tick_until(&mut self, tick_number: usize) {
            if self.ticks < tick_number {
                self.tick(tick_number - self.ticks);
            }
        }
    }

    fn create_oscillator(
        waveform: WaveformType,
        tune: OscillatorTune,
        note: MidiNote,
    ) -> Oscillator {
        let sample_rate = DEFAULT_SAMPLE_RATE;
        let mut oscillator = Oscillator::new_from_preset(
            sample_rate,
            &OscillatorSettings {
                waveform,
                tune,
                ..Default::default()
            },
        );
        oscillator.set_frequency(MidiUtils::note_type_to_frequency(note));
        oscillator
    }

    #[test]
    fn test_oscillator_pola() {
        let mut oscillator = Oscillator::new_with(DEFAULT_SAMPLE_RATE);

        // we'll run two ticks in case the oscillator happens to start at zero
        oscillator.tick(2);
        assert_ne!(
            0.0,
            oscillator.value(),
            "Default Oscillator should not be silent"
        );
    }

    // Make sure we're dealing with at least a pulse-width wave of amplitude
    // 1.0, which means that every value is either 1.0 or -1.0.
    #[test]
    fn test_square_wave_is_correct_amplitude() {
        const SAMPLE_RATE: usize = 63949; // Prime number
        const FREQUENCY: f32 = 499.0;
        let mut oscillator =
            Oscillator::new_with_type_and_frequency(SAMPLE_RATE, WaveformType::Square, FREQUENCY);

        // Below Nyquist limit
        assert_lt!(FREQUENCY, (SAMPLE_RATE / 2) as f32);

        for _ in 0..SAMPLE_RATE {
            oscillator.tick(1);
            let f = oscillator.value();
            assert_eq!(f, f.signum());
        }
    }

    #[test]
    fn test_square_wave_frequency_is_accurate() {
        // For this test, we want the sample rate and frequency to be nice even
        // numbers so that we don't have to deal with edge cases.
        const SAMPLE_RATE: usize = 65536;
        const FREQUENCY: f32 = 128.0;
        let mut oscillator =
            Oscillator::new_with_type_and_frequency(SAMPLE_RATE, WaveformType::Square, FREQUENCY);

        let mut n_pos = 0;
        let mut n_neg = 0;
        let mut last_sample = 1.0;
        let mut transitions = 0;
        for _ in 0..SAMPLE_RATE {
            oscillator.tick(1);
            let f = oscillator.value();
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
            Oscillator::new_with_type_and_frequency(SAMPLE_RATE, WaveformType::Square, FREQUENCY);

        oscillator.tick(1);
        assert_eq!(
            oscillator.value(),
            1.0,
            "the first sample of a square wave should be 1.0"
        );

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
        oscillator.tick(SAMPLE_RATE / 4 - 2);
        assert_eq!(oscillator.value(), 1.0);
        oscillator.tick(1);
        assert_eq!(oscillator.value(), 1.0);
        oscillator.tick(1);
        assert_eq!(oscillator.value(), -1.0);
        oscillator.tick(1);
        assert_eq!(oscillator.value(), -1.0);

        // Then should transition back to 1.0 at the first sample of the second
        // cycle.
        //
        // As noted above, we're using clock.set_samples() here.
        oscillator.debug_tick_until(SAMPLE_RATE / 2 - 2);
        assert_eq!(oscillator.value(), -1.0);
        oscillator.tick(1);
        assert_eq!(oscillator.value(), -1.0);
        oscillator.tick(1);
        assert_eq!(oscillator.value(), 1.0);
        oscillator.tick(1);
        assert_eq!(oscillator.value(), 1.0);
    }

    #[test]
    fn test_sine_wave_is_balanced() {
        const FREQUENCY: f32 = 1.0;
        let mut oscillator = Oscillator::new_with_type_and_frequency(
            DEFAULT_SAMPLE_RATE,
            WaveformType::Sine,
            FREQUENCY,
        );

        let mut n_pos = 0;
        let mut n_neg = 0;
        let mut n_zero = 0;
        for _ in 0..DEFAULT_SAMPLE_RATE {
            oscillator.tick(1);
            let f = oscillator.value();
            if f < -0.0000001 {
                n_neg += 1;
            } else if f > 0.0000001 {
                n_pos += 1;
            } else {
                n_zero += 1;
            }
        }
        assert_eq!(n_zero, 2);
        assert_eq!(n_pos, n_neg);
        assert_eq!(n_pos + n_neg + n_zero, DEFAULT_SAMPLE_RATE);
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
            let mut osc = Oscillator::new_with_type_and_frequency(
                DEFAULT_SAMPLE_RATE,
                WaveformType::Square,
                test_case.0,
            );
            let samples = render_signal_as_audio_source(&mut osc, 1);
            let mut filename = Paths::test_data_path();
            filename.push("audacity");
            filename.push("44100Hz-mono");
            filename.push(format!("square-{}.wav", test_case.1));

            assert!(
                samples_match_known_good_wav_file(samples, &filename, 0.001),
                "while testing square {}Hz",
                test_case.0
            );
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
            let mut osc = Oscillator::new_with_type_and_frequency(
                DEFAULT_SAMPLE_RATE,
                WaveformType::Sine,
                test_case.0,
            );
            let samples = render_signal_as_audio_source(&mut osc, 1);
            let mut filename = Paths::test_data_path();
            filename.push("audacity");
            filename.push("44100Hz-mono");
            filename.push(format!("sine-{}.wav", test_case.1));

            assert!(
                samples_match_known_good_wav_file(samples, &filename, 0.001),
                "while testing sine {}Hz",
                test_case.0
            );
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
            let mut osc = Oscillator::new_with_type_and_frequency(
                DEFAULT_SAMPLE_RATE,
                WaveformType::Sawtooth,
                test_case.0,
            );
            let samples = render_signal_as_audio_source(&mut osc, 1);
            let mut filename = Paths::test_data_path();
            filename.push("audacity");
            filename.push("44100Hz-mono");
            filename.push(format!("sawtooth-{}.wav", test_case.1));

            assert!(
                samples_match_known_good_wav_file(samples, &filename, 0.001),
                "while testing sawtooth {}Hz",
                test_case.0
            );
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
            let mut osc = Oscillator::new_with_type_and_frequency(
                DEFAULT_SAMPLE_RATE,
                WaveformType::Triangle,
                test_case.0,
            );
            let samples = render_signal_as_audio_source(&mut osc, 1);
            let mut filename = Paths::test_data_path();
            filename.push("audacity");
            filename.push("44100Hz-mono");
            filename.push(format!("triangle-{}.wav", test_case.1));

            assert!(
                samples_match_known_good_wav_file(samples, &filename, 0.01),
                "while testing triangle {}Hz",
                test_case.0
            );
        }
    }

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
        let mut oscillator = Oscillator::new_with(DEFAULT_SAMPLE_RATE);
        const FREQUENCY: f32 = 2.0;
        oscillator.set_frequency(FREQUENCY);

        const TICKS_IN_CYCLE: usize = DEFAULT_SAMPLE_RATE / FREQUENCY as usize;
        assert_eq!(TICKS_IN_CYCLE, 44100 / 2);

        // We assume that synced oscillators can take care of their own init.
        assert!(
            !oscillator.should_sync(),
            "On init, the oscillator should NOT flag that any init/reset work needs to happen."
        );

        // Now run through and see that we're flagging cycle start at the right
        // time. Note the = in the for loop range; we're expecting a flag at the
        // zeroth sample of each cycle.
        for tick in 0..=TICKS_IN_CYCLE {
            let expected = match tick {
                0 => true,              // zeroth sample of first cycle
                TICKS_IN_CYCLE => true, // zeroth sample of second cycle
                _ => false,
            };

            oscillator.tick(1);
            assert_eq!(
                oscillator.should_sync(),
                expected,
                "expected {expected} at sample #{tick}"
            );
        }

        // Let's try again after rewinding the clock. It should recognize
        // something happened and restart the cycle.
        oscillator.tick(1);
        assert!(
            !oscillator.should_sync(),
            "Oscillator shouldn't sync midway through cycle."
        );

        // Then we actually change the clock. We'll pick something we know is
        // off-cycle. We don't treat this as a should-sync event, because we
        // assume that synced oscillators will also notice the clock change and
        // do the right thing. At worst, we'll be off for a single main
        // oscillator cycle. No normal audio performance will involve a clock
        // shift, so it's OK to have the wrong timbre for a tiny fraction of a
        // second.
        oscillator.reset(DEFAULT_SAMPLE_RATE);
        oscillator.tick(1);
        assert!(
            oscillator.should_sync(),
            "After reset, oscillator should sync."
        );
        oscillator.tick(1);
        assert!(
            !oscillator.should_sync(),
            "Oscillator shouldn't sync twice when syncing after reset."
        );

        // Let's run through again, but this time go for a whole second, and
        // count the number of flags.
        oscillator.reset(DEFAULT_SAMPLE_RATE);
        let mut cycles = 0;
        for _ in 0..DEFAULT_SAMPLE_RATE {
            oscillator.tick(1);
            if oscillator.should_sync() {
                cycles += 1;
            }
        }
        assert_eq!(cycles, FREQUENCY as usize);
    }
}
