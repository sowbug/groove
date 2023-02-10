use super::oscillators::KahanSummation;
use crate::{
    clock::ClockTimeUnit,
    common::{Normal, SignalType, TimeUnit},
    settings::patches::EnvelopeSettings,
    traits::Ticks,
    Clock,
};
use more_asserts::{debug_assert_ge, debug_assert_le};
use nalgebra::{Matrix3, Matrix3x1};
use std::{fmt::Debug, ops::Range};

/// Describes the public parts of an envelope generator, which provides a
/// normalized amplitude (0.0..=1.0) that changes over time according to its
/// internal parameters, external triggers, and the progression of time.
pub trait GeneratesEnvelope: Send + Debug + Ticks {
    /// Triggers the active part of the envelope. "Enqueue" means that the
    /// attack event won't be processed until the next Ticks::tick().
    fn enqueue_attack(&mut self);

    /// Signals the end of the active part of the envelope. As with attack,
    /// release is processed at the next tick().
    fn enqueue_release(&mut self);

    /// Whether the envelope generator has finished the active part of the
    /// envelope (or hasn't yet started it). Like amplitude(), this value is
    /// valid for the current frame only after tick() is called.
    fn is_idle(&self) -> bool;

    /// Returns the current envelope amplitude(). This value is valid for the
    /// current frame once Ticks::tick() has been called for the current frame.
    fn amplitude(&self) -> Normal;

    fn batch_amplitude(&mut self, amplitudes: &mut [Normal]);
}

#[derive(Clone, Copy, Debug, Default)]
enum EnvelopeGeneratorState {
    #[default]
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Debug, Default)]
pub struct EnvelopeGenerator {
    settings: EnvelopeSettings,
    ticks: usize,
    time: TimeUnit,
    sample_rate: f64,
    state: EnvelopeGeneratorState,
    uncorrected_amplitude: KahanSummation<f64, f64>,
    corrected_amplitude: Normal,
    delta: f64,

    amplitude_target: f64,
    time_target: TimeUnit,

    // Whether the amplitude was set to an explicit value during this frame,
    // which means that the caller is expecting to get an amplitude of that
    // exact value, which means that we should return the PRE-update value
    // rather than the usual post-update value.
    amplitude_was_set: bool,

    // Polynomial coefficients for convex
    convex_a: f64,
    convex_b: f64,
    convex_c: f64,

    // Polynomial coefficients for concave
    concave_a: f64,
    concave_b: f64,
    concave_c: f64,

    note_on_pending: bool,
    note_off_pending: bool,
}
impl GeneratesEnvelope for EnvelopeGenerator {
    fn enqueue_attack(&mut self) {
        self.note_on_pending = true;
    }

    fn enqueue_release(&mut self) {
        self.note_off_pending = true;
    }

    fn amplitude(&self) -> Normal {
        self.corrected_amplitude
    }

    fn batch_amplitude(&mut self, amplitudes: &mut [Normal]) {
        // TODO: this is probably no more efficient than calling amplitude()
        // individually, but for now we're just getting the interface right.
        // Later we'll take advantage of it.
        for item in amplitudes {
            self.tick(1);
            *item = self.amplitude();
        }
    }

    fn is_idle(&self) -> bool {
        matches!(self.state, EnvelopeGeneratorState::Idle)
    }
}
impl Ticks for EnvelopeGenerator {
    fn reset(&mut self, sample_rate: usize) {
        self.sample_rate = sample_rate as f64;
        // TODO: reset stuff
    }

    fn tick(&mut self, tick_count: usize) {
        // TODO: same comment as above about not yet taking advantage of
        // batching
        for _ in 0..tick_count {
            self.ticks += 1;
            self.time = TimeUnit(self.ticks as f64 / self.sample_rate);

            self.handle_pending();
            let pre_update_amplitude = self.uncorrected_amplitude.current_sum();
            self.update_amplitude();
            self.handle_state();

            let linear_amplitude = if self.amplitude_was_set {
                self.amplitude_was_set = false;
                pre_update_amplitude
            } else {
                self.uncorrected_amplitude.current_sum()
            };
            self.corrected_amplitude = Normal::new(match self.state {
                EnvelopeGeneratorState::Attack => self.transform_linear_to_convex(linear_amplitude),
                EnvelopeGeneratorState::Decay | EnvelopeGeneratorState::Release => {
                    self.transform_linear_to_concave(linear_amplitude)
                }
                _ => linear_amplitude,
            });
        }
    }
}
impl EnvelopeGenerator {
    /// returns true if the amplitude was set to a new value.
    fn handle_pending(&mut self) {
        // We need to be careful when we've been asked to do a note-on and
        // note-off at the same time. Depending on whether we're active, we
        // handle this differently.
        if self.note_on_pending && self.note_off_pending {
            if self.is_idle() {
                self.set_state(EnvelopeGeneratorState::Attack);
                self.set_state(EnvelopeGeneratorState::Release);
            } else {
                self.set_state(EnvelopeGeneratorState::Release);
                self.set_state(EnvelopeGeneratorState::Attack);
            }
        } else if self.note_off_pending {
            self.set_state(EnvelopeGeneratorState::Release);
        } else if self.note_on_pending {
            self.set_state(EnvelopeGeneratorState::Attack);
        }
        self.note_off_pending = false;
        self.note_on_pending = false;
    }

    pub(crate) fn new_with(sample_rate: usize, envelope_settings: &EnvelopeSettings) -> Self {
        Self {
            settings: envelope_settings.clone(),
            sample_rate: sample_rate as f64,
            state: EnvelopeGeneratorState::Idle,
            ..Default::default()
        }
    }

    fn update_amplitude(&mut self) {
        self.uncorrected_amplitude.add(self.delta);
    }

    fn handle_state(&mut self) {
        match self.state {
            EnvelopeGeneratorState::Idle => {
                // Nothing to do; we're waiting for a trigger
            }
            EnvelopeGeneratorState::Attack => {
                if self.has_reached_target() {
                    self.set_state(EnvelopeGeneratorState::Decay);
                }
            }
            EnvelopeGeneratorState::Decay => {
                if self.has_reached_target() {
                    self.set_state(EnvelopeGeneratorState::Sustain);
                }
            }
            EnvelopeGeneratorState::Sustain => {
                // Nothing to do; we're waiting for a note-off event
            }
            EnvelopeGeneratorState::Release => {
                if self.has_reached_target() {
                    self.set_state(EnvelopeGeneratorState::Idle);
                }
            }
        }
    }

    fn has_reached_target(&mut self) -> bool {
        let has_hit_target = if self.delta == 0.0 {
            // This is probably a degenerate case, but we don't want to be stuck
            // forever in the current state.
            true
        } else if self.time_target.0 != 0.0 && self.time >= self.time_target {
            // If we have a time target and we've hit it, then we're done even
            // if the amplitude isn't quite there yet.
            true
        } else {
            // Is the difference between the current value and the target
            // smaller than the delta? This is a fancy way of saying we're as
            // close as we're going to get without overshooting the next time.
            (self.uncorrected_amplitude.current_sum() - self.amplitude_target).abs()
                < self.delta.abs()
        };

        if has_hit_target {
            // Set to the exact amplitude target in case of precision errors. We
            // don't want to set self.amplitude_was_set here because this is
            // happening after the update, so we'll already be returning the
            // amplitude snapshotted at the right time.
            self.uncorrected_amplitude.set_sum(self.amplitude_target);
        }
        has_hit_target
    }

    // For all the set_state_() methods, we assume that the prior state actually
    // happened, and that the amplitude is set to a reasonable value. This
    // matters, for example, if attack is zero and decay is non-zero. If we jump
    // straight from idle to decay, then decay is decaying from the idle
    // amplitude of zero, which is wrong.
    fn set_state(&mut self, new_state: EnvelopeGeneratorState) {
        match new_state {
            EnvelopeGeneratorState::Idle => {
                self.state = EnvelopeGeneratorState::Idle;
                self.uncorrected_amplitude = Default::default();
                self.delta = 0.0;
            }
            EnvelopeGeneratorState::Attack => {
                if self.settings.attack as f64 == TimeUnit::zero().0 {
                    self.set_explicit_amplitude(Normal::MAX);
                    self.set_state(EnvelopeGeneratorState::Decay);
                } else {
                    self.state = EnvelopeGeneratorState::Attack;
                    let target_amplitude = Normal::maximum().value();
                    self.set_target(
                        Normal::maximum(),
                        TimeUnit(self.settings.attack as f64),
                        false,
                        false,
                    );
                    let current_amplitude = self.uncorrected_amplitude.current_sum();

                    (self.convex_a, self.convex_b, self.convex_c) = Self::calculate_coefficients(
                        current_amplitude,
                        current_amplitude,
                        (target_amplitude - current_amplitude) / 2.0 + current_amplitude,
                        (target_amplitude - current_amplitude) / 1.5 + current_amplitude,
                        target_amplitude,
                        target_amplitude,
                    );
                }
            }
            EnvelopeGeneratorState::Decay => {
                if self.settings.decay as f64 == TimeUnit::zero().0 {
                    self.set_explicit_amplitude(self.settings.sustain as f64);
                    self.set_state(EnvelopeGeneratorState::Sustain);
                } else {
                    self.state = EnvelopeGeneratorState::Decay;
                    let target_amplitude = self.settings.sustain as f64;
                    self.set_target(
                        Normal::new(target_amplitude),
                        TimeUnit(self.settings.decay as f64),
                        true,
                        false,
                    );
                    let current_amplitude = self.uncorrected_amplitude.current_sum();
                    (self.concave_a, self.concave_b, self.concave_c) = Self::calculate_coefficients(
                        current_amplitude,
                        current_amplitude,
                        (current_amplitude - target_amplitude) / 2.0 + target_amplitude,
                        (current_amplitude - target_amplitude) / 3.0 + target_amplitude,
                        target_amplitude,
                        target_amplitude,
                    );
                }
            }
            EnvelopeGeneratorState::Sustain => {
                self.state = EnvelopeGeneratorState::Sustain;

                self.set_target(
                    Normal::new(self.settings.sustain as f64),
                    TimeUnit::infinite(),
                    false,
                    false,
                );
            }
            EnvelopeGeneratorState::Release => {
                if self.settings.release as f64 == TimeUnit::zero().0 {
                    self.set_explicit_amplitude(Normal::MAX);
                    self.set_state(EnvelopeGeneratorState::Idle);
                } else {
                    self.state = EnvelopeGeneratorState::Release;
                    let target_amplitude = 0.0;
                    self.set_target(
                        Normal::minimum(),
                        TimeUnit(self.settings.release as f64),
                        true,
                        false,
                    );
                    let current_amplitude = self.uncorrected_amplitude.current_sum();
                    (self.concave_a, self.concave_b, self.concave_c) = Self::calculate_coefficients(
                        current_amplitude,
                        current_amplitude,
                        (current_amplitude - target_amplitude) / 2.0 + target_amplitude,
                        (current_amplitude - target_amplitude) / 3.0 + target_amplitude,
                        target_amplitude,
                        target_amplitude,
                    );
                }
            }
        }
    }

    fn set_explicit_amplitude(&mut self, new_value: f64) {
        self.uncorrected_amplitude.set_sum(new_value);
        self.amplitude_was_set = true;
    }

    fn set_target(
        &mut self,
        target_amplitude: Normal,
        duration: TimeUnit,
        calculate_for_full_amplitude_range: bool,
        fast_reaction: bool,
    ) {
        self.amplitude_target = target_amplitude.into();
        if duration != TimeUnit::infinite() {
            let fast_reaction_extra_frame = if fast_reaction { 1.0 } else { 0.0 };
            let range = if calculate_for_full_amplitude_range {
                -1.0
            } else {
                self.amplitude_target - self.uncorrected_amplitude.current_sum()
            };
            self.time_target = self.time + duration;
            self.delta = if duration != TimeUnit::zero() {
                range / (duration.0 * self.sample_rate + fast_reaction_extra_frame) as f64
            } else {
                0.0
            };
            if fast_reaction {
                self.uncorrected_amplitude.add(self.delta);
            }
        } else {
            self.time_target = TimeUnit::infinite();
            self.delta = 0.0;
        }
    }

    fn calculate_coefficients(
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    ) -> (f64, f64, f64) {
        let m = Matrix3::new(
            1.0,
            x0,
            x0.powi(2),
            1.0,
            x1,
            x1.powi(2),
            1.0,
            x2,
            x2.powi(2),
        );
        let y = Matrix3x1::new(y0, y1, y2);
        let r = m.try_inverse();
        if let Some(r) = r {
            let abc = r * y;
            (abc[0], abc[1], abc[2])
        } else {
            (0.0, 0.0, 0.0)
        }
    }

    fn transform_linear_to_convex(&self, linear_value: f64) -> f64 {
        self.convex_c * linear_value.powi(2) + self.convex_b * linear_value + self.convex_a
    }
    fn transform_linear_to_concave(&self, linear_value: f64) -> f64 {
        self.concave_c * linear_value.powi(2) + self.concave_b * linear_value + self.concave_a
    }
}

#[derive(Clone, Debug, Default)]
pub enum EnvelopeFunction {
    #[default]
    Linear,
    Logarithmic,
    Exponential,
}

#[derive(Clone, Debug, Default)]
pub struct EnvelopeStep {
    pub interval: Range<SignalType>,
    pub start_value: SignalType,
    pub end_value: SignalType,
    pub step_function: EnvelopeFunction,
}

#[derive(Clone, Debug, Default)]
pub struct SteppedEnvelope {
    time_unit: ClockTimeUnit,
    steps: Vec<EnvelopeStep>,
}
impl SteppedEnvelope {
    const EMPTY_STEP: EnvelopeStep = EnvelopeStep {
        interval: Range {
            start: 0.0,
            end: 0.0,
        },
        start_value: 0.0,
        end_value: 0.0,
        step_function: EnvelopeFunction::Linear,
    };

    pub(crate) fn new_with_time_unit(time_unit: ClockTimeUnit) -> Self {
        Self {
            time_unit,
            ..Default::default()
        }
    }

    pub(crate) fn push_step(&mut self, step: EnvelopeStep) {
        self.steps.push(step);
        self.debug_validate_steps();
    }

    fn steps(&self) -> &[EnvelopeStep] {
        &self.steps
    }

    pub(crate) fn step_for_time(&self, time: f64) -> &EnvelopeStep {
        let steps = self.steps();
        if steps.is_empty() {
            return &Self::EMPTY_STEP;
        }

        let mut candidate_step: &EnvelopeStep = steps.first().unwrap();
        for step in steps {
            if candidate_step.interval.end == f64::MAX {
                // Any step with max end_time is terminal.
                break;
            }
            debug_assert!(step.interval.start >= candidate_step.interval.start);
            debug_assert!(step.interval.end >= candidate_step.interval.start);

            if step.interval.start > time {
                // This step starts in the future. If all steps' start times
                // are in order, then we can't do better than what we have.
                break;
            }
            if step.interval.end < time {
                // This step already ended. It's invalid for this point in time.
                continue;
            }
            candidate_step = step;
        }
        candidate_step
    }

    pub(crate) fn value_for_step_at_time(&self, step: &EnvelopeStep, time: f64) -> SignalType {
        if step.interval.start == step.interval.end || step.start_value == step.end_value {
            return step.end_value;
        }
        let elapsed_time = time - step.interval.start;
        let total_interval_time = step.interval.end - step.interval.start;
        let percentage_complete = elapsed_time / total_interval_time;
        let total_interval_value_delta = step.end_value - step.start_value;

        let multiplier = if percentage_complete == 0.0 {
            0.0
        } else {
            match step.step_function {
                EnvelopeFunction::Linear => percentage_complete,
                EnvelopeFunction::Logarithmic => {
                    (percentage_complete.log(10000.0) * 2.0 + 1.0).clamp(0.0, 1.0)
                }
                EnvelopeFunction::Exponential => 100.0f64.powf(percentage_complete as f64) / 100.0,
            }
        };
        let mut value = step.start_value + total_interval_value_delta * multiplier;
        if (step.end_value > step.start_value && value > step.end_value)
            || (step.end_value < step.start_value && value < step.end_value)
        {
            value = step.end_value;
        }
        value
    }

    fn debug_validate_steps(&self) {
        debug_assert!(!self.steps.is_empty());
        debug_assert_eq!(self.steps.first().unwrap().interval.start, 0.0);
        // TODO: this should be optional depending on who's using it ..... debug_assert_eq!(self.steps.last().unwrap().interval.end, f32::MAX);
        let mut start_time = 0.0;
        let mut end_time = 0.0;
        let steps = self.steps();
        for step in steps {
            debug_assert_le!(step.interval.start, step.interval.end); // Next step has non-negative duration
            debug_assert_ge!(step.interval.start, start_time); // We're not moving backward in time
            debug_assert_le!(step.interval.start, end_time); // Next step leaves no gaps (overlaps OK)
            start_time = step.interval.start;
            end_time = step.interval.end;

            // We don't require subsequent steps to be valid, as long as
            // an earlier step covered the rest of the time range.
            if step.interval.end == f64::MAX {
                break;
            }
        }
        // TODO same debug_assert_eq!(end_time, f32::MAX);
    }

    pub(crate) fn time_for_unit(&self, clock: &Clock) -> f64 {
        clock.time_for(&self.time_unit) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::Clock;
    use assert_approx_eq::assert_approx_eq;
    use float_cmp::approx_eq;
    use more_asserts::{assert_gt, assert_lt};

    impl EnvelopeStep {
        pub(crate) fn new_with_duration(
            start_time: f64,
            duration: f64,
            start_value: SignalType,
            end_value: SignalType,
            step_function: EnvelopeFunction,
        ) -> Self {
            Self {
                interval: Range {
                    start: start_time,
                    end: if duration == f64::MAX {
                        duration
                    } else {
                        start_time + duration
                    },
                },
                start_value,
                end_value,
                step_function,
            }
        }
    }

    impl EnvelopeGenerator {
        fn debug_state(&self) -> &EnvelopeGeneratorState {
            &self.state
        }

        /// The current value of the envelope generator. Note that this value is
        /// often not the one you want if you really care about getting the
        /// amplitude at specific interesting time points in the envelope's
        /// lifecycle. If you call it before the current time slice's tick(), then
        /// you get the value before any pending events (which is probably bad), and
        /// if you call it after the tick(), then you get the value for the *next*
        /// time slice (which is probably bad). It's better to use the value
        /// returned by tick(), which is in between pending events but after
        /// updating for the time slice.
        fn debug_amplitude(&self) -> Normal {
            Normal::new(self.uncorrected_amplitude.current_sum())
        }
    }

    #[test]
    fn test_envelope_step_functions() {
        const START_TIME: f64 = 3.14159;
        const DURATION: f64 = 2.71828;
        const START_VALUE: SignalType = 1.0;
        const END_VALUE: SignalType = 1.0 + 10.0;

        let mut envelope = SteppedEnvelope::default();
        // This envelope is here just to offset the one we're testing,
        // to catch bugs where we assumed the start time was 0.0.
        envelope.push_step(EnvelopeStep::new_with_duration(
            0.0,
            START_TIME,
            0.0,
            0.0,
            EnvelopeFunction::Linear,
        ));
        envelope.push_step(EnvelopeStep::new_with_duration(
            START_TIME,
            DURATION,
            START_VALUE,
            END_VALUE,
            EnvelopeFunction::Linear,
        ));

        // We're lazy and ask for the step only once because we know there's only one.
        let step = envelope.step_for_time(START_TIME);
        assert_eq!(
            envelope.value_for_step_at_time(step, START_TIME),
            START_VALUE
        );
        assert_eq!(
            envelope.value_for_step_at_time(step, START_TIME + DURATION / 2.0),
            1.0 + 10.0 / 2.0
        );
        assert_eq!(
            envelope.value_for_step_at_time(step, START_TIME + DURATION),
            END_VALUE
        );

        let mut envelope = SteppedEnvelope::default();
        envelope.push_step(EnvelopeStep::new_with_duration(
            0.0,
            START_TIME,
            0.0,
            0.0,
            EnvelopeFunction::Linear,
        ));
        envelope.push_step(EnvelopeStep::new_with_duration(
            START_TIME,
            DURATION,
            START_VALUE,
            END_VALUE,
            EnvelopeFunction::Logarithmic,
        ));

        let step = envelope.step_for_time(START_TIME);
        assert_eq!(
            envelope.value_for_step_at_time(step, START_TIME),
            START_VALUE
        ); // special case log(0) == 0.0
        assert_approx_eq!(
            envelope.value_for_step_at_time(step, START_TIME + DURATION / 2.0),
            1.0 + 8.49485
        ); // log(0.5, 10000) corrected for (0.0..=1.0)
        assert_eq!(
            envelope.value_for_step_at_time(step, START_TIME + DURATION),
            END_VALUE
        );

        let mut envelope = SteppedEnvelope::default();
        envelope.push_step(EnvelopeStep::new_with_duration(
            0.0,
            START_TIME,
            0.0,
            0.0,
            EnvelopeFunction::Linear,
        ));
        envelope.push_step(EnvelopeStep::new_with_duration(
            START_TIME,
            DURATION,
            START_VALUE,
            END_VALUE,
            EnvelopeFunction::Exponential,
        ));

        let step = envelope.step_for_time(START_TIME);
        assert_eq!(
            envelope.value_for_step_at_time(step, START_TIME),
            START_VALUE
        );
        assert_eq!(
            envelope.value_for_step_at_time(step, START_TIME + DURATION / 2.0),
            1.0 + 10.0 * 0.1
        );
        assert_eq!(
            envelope.value_for_step_at_time(step, START_TIME + DURATION),
            END_VALUE
        );
    }

    // Where possible, we'll erase the envelope type and work only with the
    // GeneratesEnvelope trait, so that we can confirm that the trait alone is
    // useful.
    fn get_ge_trait_stuff() -> (EnvelopeSettings, Clock, impl GeneratesEnvelope) {
        let envelope_settings = EnvelopeSettings {
            attack: 0.1,
            decay: 0.2,
            sustain: 0.8,
            release: 0.3,
        };
        let clock = Clock::default();
        let envelope = EnvelopeGenerator::new_with(clock.sample_rate(), &envelope_settings);
        (envelope_settings, clock, envelope)
    }

    #[test]
    fn generates_envelope_trait_idle() {
        let (_envelope_settings, mut clock, mut e) = get_ge_trait_stuff();

        assert!(e.is_idle(), "Envelope should be idle on creation.");

        e.tick(1);
        clock.tick();
        assert!(e.is_idle(), "Untriggered envelope should remain idle.");
        assert_eq!(
            e.amplitude().value(),
            0.0,
            "Untriggered envelope should remain amplitude zero."
        );
    }

    fn run_until<F>(
        envelope: &mut impl GeneratesEnvelope,
        clock: &mut Clock,
        time_marker: f32,
        mut test: F,
    ) -> Normal
    where
        F: FnMut(f64),
    {
        let mut amplitude: Normal = Normal::new(0.0);
        loop {
            envelope.tick(1);
            let should_continue = clock.seconds() < time_marker;
            clock.tick();
            if !should_continue {
                break;
            }
            amplitude = envelope.amplitude();
            test(amplitude.value());
        }
        amplitude
    }

    #[test]
    fn generates_envelope_trait_instant_trigger_response() {
        let (_envelope_settings, mut clock, mut e) = get_ge_trait_stuff();

        e.enqueue_attack();
        e.tick(1);
        clock.tick();
        assert!(
            !e.is_idle(),
            "Envelope should be active immediately upon trigger"
        );

        // We apply a small fudge factor to account for the fact that the MMA
        // convex transform rounds to zero pretty aggressively, so attacks take
        // a bit of time before they are apparent. I'm not sure whether this is
        // a good thing; it objectively makes attack laggy (in this case 16
        // samples late!).
        for _ in 0..17 {
            e.tick(1);
            clock.tick();
        }
        assert_gt!(
            e.amplitude().value(),
            0.0,
            "Envelope amplitude should increase immediately upon trigger"
        );
    }

    #[test]
    fn generates_envelope_trait_attack_decay_duration() {
        let envelope_settings = EnvelopeSettings {
            attack: 0.1,
            decay: 0.2,
            sustain: 0.8,
            release: 0.3,
        };
        // An even sample rate means we can easily calculate how much time was spent in each state.
        let mut clock = Clock::new_with_sample_rate(100);
        let mut envelope = EnvelopeGenerator::new_with(clock.sample_rate(), &envelope_settings);

        envelope.enqueue_attack();
        envelope.tick(1);
        let mut time_marker = clock.seconds() + envelope_settings.attack;
        assert!(
            matches!(envelope.debug_state(), EnvelopeGeneratorState::Attack),
            "Expected SimpleEnvelopeState::Attack after trigger, but got {:?} instead",
            envelope.debug_state()
        );
        clock.tick();

        let amplitude = run_until(&mut envelope, &mut clock, time_marker, |_amplitude| {});
        assert!(matches!(
            envelope.debug_state(),
            EnvelopeGeneratorState::Decay
        ));
        assert_eq!(
            amplitude.value(),
            1.0,
            "Amplitude should reach maximum after attack."
        );

        time_marker += envelope_settings.decay;
        let amplitude = run_until(&mut envelope, &mut clock, time_marker, |_amplitude| {});
        assert_eq!(
            amplitude.value(),
            envelope_settings.sustain as f64,
            "Amplitude should reach sustain level after decay."
        );
        assert!(matches!(
            envelope.debug_state(),
            EnvelopeGeneratorState::Sustain
        ));
    }

    #[test]
    fn generates_envelope_trait_sustain_duration_then_release() {
        let envelope_settings = EnvelopeSettings {
            attack: 0.1,
            decay: 0.2,
            sustain: 0.8,
            release: 0.3,
        };
        let mut clock = Clock::default();
        let mut envelope = EnvelopeGenerator::new_with(clock.sample_rate(), &envelope_settings);

        envelope.enqueue_attack();
        envelope.tick(1);
        let mut time_marker =
            clock.seconds() + envelope_settings.attack + envelope_settings.expected_decay_time();
        clock.tick();

        // Skip past attack/decay.
        run_until(&mut envelope, &mut clock, time_marker, |_amplitude| {});

        let sustain = envelope_settings.sustain as f64;
        time_marker += 0.5;
        let amplitude = run_until(&mut envelope, &mut clock, time_marker, |amplitude| {
            assert_eq!(
                amplitude, sustain,
                "Amplitude should remain at sustain level while note is still triggered"
            );
        })
        .value();

        envelope.enqueue_release();
        time_marker += envelope_settings.expected_release_time(amplitude);
        let mut last_amplitude = amplitude;
        let amplitude = run_until(&mut envelope, &mut clock, time_marker, |inner_amplitude| {
            assert_lt!(
                inner_amplitude,
                last_amplitude,
                "Amplitude should begin decreasing as soon as note off."
            );
            last_amplitude = inner_amplitude;
        });

        // These assertions are checking the next frame's state, which is right
        // because we want to test what happens after the release ends.
        assert!(
            envelope.is_idle(),
            "Envelope should be idle when release ends, but it wasn't (amplitude is {})",
            amplitude.value()
        );
        assert_eq!(
            envelope.debug_amplitude().value(),
            0.0,
            "Amplitude should be zero when release ends"
        );
    }

    #[test]
    fn simple_envelope_interrupted_decay_with_second_attack() {
        // These settings are copied from Welsh Piano's filter envelope, which
        // is where I noticed some unwanted behavior.
        let envelope_settings = EnvelopeSettings {
            attack: 0.0,
            decay: 5.22,
            sustain: 0.25,
            release: 0.5,
        };
        let mut clock = Clock::default();
        let mut envelope = EnvelopeGenerator::new_with(clock.sample_rate(), &envelope_settings);

        envelope.tick(1);
        clock.tick();

        assert_eq!(
            envelope.amplitude(),
            Normal::minimum(),
            "Amplitude should start at zero"
        );

        // See https://floating-point-gui.de/errors/comparison/ for standard
        // warning about comparing floats and looking for epsilons.
        envelope.enqueue_attack();
        envelope.tick(1);
        let mut time_marker = clock.seconds();
        clock.tick();
        assert!(
            approx_eq!(
                f64,
                envelope.amplitude().value(),
                Normal::maximum().value(),
                ulps = 8
            ),
            "Amplitude should reach peak upon trigger, but instead of {} we got {}",
            Normal::maximum().value(),
            envelope.amplitude().value(),
        );
        envelope.tick(1);
        clock.tick();
        assert_lt!(
            envelope.amplitude(),
            Normal::maximum(),
            "Zero-attack amplitude should begin decreasing immediately after peak"
        );

        // Jump to halfway through decay.
        time_marker += envelope_settings.attack + envelope_settings.decay / 2.0;
        let amplitude = run_until(&mut envelope, &mut clock, time_marker, |_amplitude| {});
        assert_lt!(
            amplitude,
            Normal::maximum(),
            "Amplitude should have decayed halfway through decay"
        );

        // Release the trigger.
        envelope.enqueue_release();
        let _amplitude = envelope.tick(1);
        clock.tick();

        // And hit it again.
        envelope.enqueue_attack();
        envelope.tick(1);
        let mut time_marker = clock.seconds();
        clock.tick();
        assert!(
            approx_eq!(
                f64,
                envelope.amplitude().value(),
                Normal::maximum().value(),
                ulps = 8
            ),
            "Amplitude should reach peak upon second trigger"
        );

        // Then release again.
        envelope.enqueue_release();

        // Check that we keep decreasing amplitude to zero, not to sustain.
        time_marker += envelope_settings.release;
        let mut last_amplitude = envelope.amplitude().value();
        let _amplitude = run_until(&mut envelope, &mut clock, time_marker, |inner_amplitude| {
            assert_lt!(
                inner_amplitude,
                last_amplitude,
                "Amplitude should continue decreasing after note off"
            );
            last_amplitude = inner_amplitude;
        });

        // These assertions are checking the next frame's state, which is right
        // because we want to test what happens after the release ends.
        assert!(
            envelope.is_idle(),
            "Envelope should be idle when release ends"
        );
        assert_eq!(
            envelope.debug_amplitude().value(),
            0.0,
            "Amplitude should be zero when release ends"
        );
    }

    // Per Pirkle, DSSPC++, p.87-88, decay and release times determine the
    // *slope* but not necessarily the *duration* of those phases of the
    // envelope. The slope assumes the specified time across a full 1.0-to-0.0
    // range. This means that the actual decay and release times for a given
    // envelope can be shorter than its parameters might suggest.
    #[test]
    fn generates_envelope_trait_decay_and_release_based_on_full_amplitude_range() {
        let envelope_settings = EnvelopeSettings {
            attack: 0.0,
            decay: 0.8,
            sustain: 0.5,
            release: 0.4,
        };
        let mut clock = Clock::default();
        let mut envelope = EnvelopeGenerator::new_with(clock.sample_rate(), &envelope_settings);

        // Decay after note-on should be shorter than the decay value.
        envelope.enqueue_attack();
        let mut time_marker = clock.seconds() + envelope_settings.expected_decay_time();
        let amplitude = run_until(&mut envelope, &mut clock, time_marker, |_amplitude| {}).value();
        assert_eq!(amplitude as f32,
            envelope_settings.sustain,
            "Expected to see sustain level {} at time {} (which is {:.1}% of decay time {}, based on full 1.0..=0.0 amplitude range)",
            envelope_settings.sustain,
            time_marker,
            envelope_settings.decay,
            100.0 * (1.0 - envelope_settings.sustain)
        );

        // Release after note-off should also be shorter than the release value.
        envelope.enqueue_release();
        let expected_release_time = envelope_settings.expected_release_time(amplitude);
        time_marker += expected_release_time;
        let amplitude = run_until(&mut envelope, &mut clock, time_marker, |inner_amplitude| {
            assert_gt!(
                inner_amplitude,
                0.0,
                "We should not reach idle before time {}, but we did.",
                &expected_release_time,
            )
        });
        let portion_of_full_amplitude_range = envelope_settings.sustain;
        assert!(
            envelope.is_idle(),
            "Expected release to end after time {}, which is {:.1}% of release time {}. Amplitude is {}",
            expected_release_time,
            100.0 * portion_of_full_amplitude_range,
            envelope_settings.release,
            amplitude.value()
        );
    }

    // https://docs.google.com/spreadsheets/d/1DSkut7rLG04Qx_zOy3cfI7PMRoGJVr9eaP5sDrFfppQ/edit#gid=0
    #[test]
    fn coeff() {
        let (a, b, c) = EnvelopeGenerator::calculate_coefficients(0.0, 1.0, 0.5, 0.25, 1.0, 0.0);
        assert_eq!(a, 1.0);
        assert_eq!(b, -2.0);
        assert_eq!(c, 1.0);
    }

    #[test]
    fn envelope_amplitude_batching() {
        let sample_rate = Clock::DEFAULT_SAMPLE_RATE;
        let envelope_settings = EnvelopeSettings {
            attack: 0.1,
            decay: 0.2,
            sustain: 0.5,
            release: 0.3,
        };
        let mut e = EnvelopeGenerator::new_with(sample_rate, &envelope_settings);

        // Initialize the buffer with a nonsense value so we know it got
        // overwritten by the method we're about to call.
        //
        // TODO: that buffer size should be pulled from somewhere centralized.
        let mut amplitudes: [Normal; 64] = [Normal::from(0.888); 64];

        // The envelope starts out in the idle state, and we haven't triggered
        // it.
        e.batch_amplitude(&mut amplitudes);
        amplitudes.iter().for_each(|i| {
            assert_eq!(
                i.value(),
                Normal::MIN,
                "Each value in untriggered EG's buffer should be set to silence"
            );
        });

        // Now trigger the envelope and see what happened.
        e.enqueue_attack();
        e.batch_amplitude(&mut amplitudes);
        assert!(
            amplitudes.iter().any(|i| { i.value() != Normal::MIN }),
            "Once triggered, the EG should generate non-silent values"
        );
    }
}
