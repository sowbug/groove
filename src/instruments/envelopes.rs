use super::oscillators::KahanSummation;
use crate::{
    clock::ClockTimeUnit,
    common::{F32ControlValue, Normal, OldMonoSample, TimeUnit},
    messages::EntityMessage,
    settings::patches::EnvelopeSettings,
    traits::{Controllable, HasUid, IsInstrument, Response, SourcesAudio, Updateable},
    Clock,
};
use groove_macros::{Control, Uid};
use more_asserts::{debug_assert_ge, debug_assert_le};
use nalgebra::{Matrix3, Matrix3x1};
use std::str::FromStr;
use std::{fmt::Debug, marker::PhantomData, ops::Range};
use strum_macros::{Display, EnumString, FromRepr};

/// The user-visible parts of an envelope generator, which provides an amplitude
/// 0.0..=1.0 that changes over time according to its internal parameters and
/// the note on/off trigger.
pub trait GeneratesEnvelope {
    /// Triggers the active part of the envelope.
    fn enqueue_attack(&mut self);

    /// Signals the end of the active part of the envelope.
    fn enqueue_release(&mut self);

    /// Gives the envelope generator time to do work. Must be called on every
    /// sample of the clock. It's not required for any GeneratesEnvelope to look
    /// for clock resets, which means that if the clock jumps around, correct
    /// behavior is not guaranteed.
    ///
    /// Returns the amplitude *after* processing any handle_* events, and
    /// *before* ticking to the next time slice.
    fn tick(&mut self, clock: &Clock) -> Normal;

    /// Whether the envelope generator has finished the active part of the
    /// envelope (or hasn't yet started it).
    ///
    /// It's generally OK to call this after tick(), unlike amplitude().
    fn is_idle(&self) -> bool;
}

#[derive(Clone, Copy, Debug, Default)]
enum SimpleEnvelopeState {
    #[default]
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Debug, Default)]
pub struct SimpleEnvelope {
    settings: EnvelopeSettings,
    sample_rate: f64,
    state: SimpleEnvelopeState,
    amplitude: KahanSummation<f64, f64>,
    delta: f64,

    amplitude_target: f64,
    time_target: TimeUnit,

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
impl GeneratesEnvelope for SimpleEnvelope {
    fn enqueue_attack(&mut self) {
        self.note_on_pending = true;
    }

    fn enqueue_release(&mut self) {
        self.note_off_pending = true;
    }

    fn tick(&mut self, clock: &Clock) -> Normal {
        let current_time = TimeUnit(clock.seconds() as f64);

        // 1. Handle queued events
        self.handle_pending(current_time);

        // 2. Calculate current amplitude
        let amplitude = match self.state {
            SimpleEnvelopeState::Attack => {
                self.transform_linear_to_convex(self.amplitude.current_sum())
            }
            SimpleEnvelopeState::Decay => {
                self.transform_linear_to_concave(self.amplitude.current_sum())
            }
            SimpleEnvelopeState::Release => {
                self.transform_linear_to_concave(self.amplitude.current_sum())
            }
            _ => self.amplitude.current_sum(),
        };

        // 3. Update for next time slice
        self.update_amplitude();
        self.handle_state(current_time);

        Normal::new(amplitude)
    }

    fn is_idle(&self) -> bool {
        matches!(self.state, SimpleEnvelopeState::Idle)
    }
}
impl SimpleEnvelope {
    fn handle_pending(&mut self, current_time: TimeUnit) {
        // We need to be careful when we've been asked to do a note-on and
        // note-off at the same time. Depending on whether we're active, we
        // handle this differently.
        if self.note_on_pending && self.note_off_pending {
            if self.is_idle() {
                self.set_state(SimpleEnvelopeState::Attack, current_time);
                self.set_state(SimpleEnvelopeState::Release, current_time);
            } else {
                self.set_state(SimpleEnvelopeState::Release, current_time);
                self.set_state(SimpleEnvelopeState::Attack, current_time);
            }
        } else if self.note_off_pending {
            self.set_state(SimpleEnvelopeState::Release, current_time);
        } else if self.note_on_pending {
            self.set_state(SimpleEnvelopeState::Attack, current_time);
        }
        self.note_off_pending = false;
        self.note_on_pending = false;
    }

    pub(crate) fn new_with(sample_rate: usize, envelope_settings: &EnvelopeSettings) -> Self {
        Self {
            settings: envelope_settings.clone(),
            sample_rate: sample_rate as f64,
            state: SimpleEnvelopeState::Idle,
            ..Default::default()
        }
    }

    fn update_amplitude(&mut self) {
        self.amplitude.add(self.delta);
    }

    fn handle_state(&mut self, current_time: TimeUnit) {
        match self.state {
            SimpleEnvelopeState::Idle => {
                // Nothing to do; we're waiting for a trigger
            }
            SimpleEnvelopeState::Attack => {
                if self.has_reached_target(current_time) {
                    self.set_state(SimpleEnvelopeState::Decay, current_time);
                }
            }
            SimpleEnvelopeState::Decay => {
                if self.has_reached_target(current_time) {
                    self.set_state(SimpleEnvelopeState::Sustain, current_time);
                }
            }
            SimpleEnvelopeState::Sustain => {
                // Nothing to do; we're waiting for a note-off event
            }
            SimpleEnvelopeState::Release => {
                if self.has_reached_target(current_time) {
                    self.set_state(SimpleEnvelopeState::Idle, current_time);
                }
            }
        }
    }

    fn has_reached_target(&mut self, current_time: TimeUnit) -> bool {
        let has_hit_target = if self.delta == 0.0 {
            // This is probably a degenerate case, but we don't want to be stuck
            // forever in the current state.
            true
        } else if self.time_target.0 != 0.0 && current_time >= self.time_target {
            // If we have a time target and we've hit it, then we're done even
            // if the amplitude isn't quite there yet.
            true
        } else {
            // Is the difference between the current value and the target
            // smaller than the delta? This is a fancy way of saying we're as
            // close as we're going to get without overshooting the next time.
            (self.amplitude.current_sum() - self.amplitude_target).abs() < self.delta.abs()
        };

        if has_hit_target {
            // Set to the exact amplitude target in case of precision errors.
            self.amplitude.set_sum(self.amplitude_target);
        }
        has_hit_target
    }

    // For all the set_state_() methods, we assume that the prior state actually
    // happened, and that the amplitude is set to a reasonable value. This
    // matters, for example, if attack is zero and decay is non-zero. If we jump
    // straight from idle to decay, then decay is decaying from the idle
    // amplitude of zero, which is wrong.
    fn set_state(&mut self, new_state: SimpleEnvelopeState, current_time: TimeUnit) {
        match new_state {
            SimpleEnvelopeState::Idle => {
                self.state = SimpleEnvelopeState::Idle;
                self.amplitude = Default::default();
                self.delta = 0.0;
            }
            SimpleEnvelopeState::Attack => {
                if self.settings.attack as f64 == TimeUnit::zero().0 {
                    self.amplitude.set_sum(Normal::MAX);
                    self.set_state(SimpleEnvelopeState::Decay, current_time);
                } else {
                    self.state = SimpleEnvelopeState::Attack;
                    let target_amplitude = Normal::maximum().value();
                    self.set_target(
                        current_time,
                        Normal::maximum(),
                        TimeUnit(self.settings.attack as f64),
                        false,
                        true,
                    );
                    let current_amplitude = self.amplitude.current_sum();

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
            SimpleEnvelopeState::Decay => {
                if self.settings.decay as f64 == TimeUnit::zero().0 {
                    self.amplitude.set_sum(self.settings.sustain as f64);
                    self.set_state(SimpleEnvelopeState::Sustain, current_time);
                } else {
                    self.state = SimpleEnvelopeState::Decay;
                    let target_amplitude = self.settings.sustain as f64;
                    self.set_target(
                        current_time,
                        Normal::new(target_amplitude),
                        TimeUnit(self.settings.decay as f64),
                        true,
                        false,
                    );
                    let current_amplitude = self.amplitude.current_sum();
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
            SimpleEnvelopeState::Sustain => {
                self.state = SimpleEnvelopeState::Sustain;

                self.set_target(
                    current_time,
                    Normal::new(self.settings.sustain as f64),
                    TimeUnit::infinite(),
                    false,
                    false,
                );
            }
            SimpleEnvelopeState::Release => {
                if self.settings.release as f64 == TimeUnit::zero().0 {
                    self.amplitude.set_sum(Normal::MAX);
                    self.set_state(SimpleEnvelopeState::Idle, current_time);
                } else {
                    self.state = SimpleEnvelopeState::Release;
                    let target_amplitude = 0.0;
                    self.set_target(
                        current_time,
                        Normal::minimum(),
                        TimeUnit(self.settings.release as f64),
                        true,
                        true,
                    );
                    let current_amplitude = self.amplitude.current_sum();
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

    fn set_target(
        &mut self,
        current_time: TimeUnit,
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
                self.amplitude_target - self.amplitude.current_sum()
            };
            self.time_target = current_time + duration;
            self.delta = if duration != TimeUnit::zero() {
                range / (duration.0 * self.sample_rate + fast_reaction_extra_frame) as f64
            } else {
                0.0
            };
            if fast_reaction {
                self.amplitude.add(self.delta);
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
            (abc[(0)], abc[(1)], abc[(2)])
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
    pub interval: Range<f32>,
    pub start_value: OldMonoSample,
    pub end_value: OldMonoSample,
    pub step_function: EnvelopeFunction,
}

impl EnvelopeStep {
    pub(crate) fn new_with_duration(
        start_time: f32,
        duration: f32,
        start_value: OldMonoSample,
        end_value: OldMonoSample,
        step_function: EnvelopeFunction,
    ) -> Self {
        Self {
            interval: Range {
                start: start_time,
                end: if duration == f32::MAX {
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

    pub(crate) fn new_with_end_time(
        interval: Range<f32>,
        start_value: OldMonoSample,
        end_value: OldMonoSample,
        step_function: EnvelopeFunction,
    ) -> Self {
        Self {
            interval,
            start_value,
            end_value,
            step_function,
        }
    }
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

    #[allow(dead_code)]
    pub(crate) fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    pub(crate) fn new_with_time_unit(time_unit: ClockTimeUnit) -> Self {
        Self {
            time_unit,
            ..Default::default()
        }
    }

    pub(crate) fn new_with(time_unit: ClockTimeUnit, vec: Vec<EnvelopeStep>) -> Self {
        let r = Self {
            time_unit,
            steps: vec,
        };
        r.debug_validate_steps();
        r
    }

    pub(crate) fn push_step(&mut self, step: EnvelopeStep) {
        self.steps.push(step);
        self.debug_validate_steps();
    }

    fn steps(&self) -> &[EnvelopeStep] {
        &self.steps
    }

    #[allow(dead_code)]
    fn time_unit(&self) -> &ClockTimeUnit {
        &self.time_unit
    }

    pub(crate) fn step_for_time(&self, time: f32) -> &EnvelopeStep {
        let steps = self.steps();
        if steps.is_empty() {
            return &Self::EMPTY_STEP;
        }

        let mut candidate_step: &EnvelopeStep = steps.first().unwrap();
        for step in steps {
            if candidate_step.interval.end == f32::MAX {
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

    pub(crate) fn value_for_step_at_time(&self, step: &EnvelopeStep, time: f32) -> OldMonoSample {
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
                EnvelopeFunction::Exponential => {
                    (100.0f64.powf(percentage_complete as f64) / 100.0) as f32
                }
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
            if step.interval.end == f32::MAX {
                break;
            }
        }
        // TODO same debug_assert_eq!(end_time, f32::MAX);
    }

    pub(crate) fn time_for_unit(&self, clock: &Clock) -> f32 {
        clock.time_for(&self.time_unit)
    }
}

impl SourcesAudio for SteppedEnvelope {
    fn source_audio(&mut self, clock: &Clock) -> OldMonoSample {
        let time = self.time_for_unit(clock);
        let step = self.step_for_time(time);
        self.value_for_step_at_time(step, time)
    }
}

#[derive(Debug, Default)]
enum AdsrEnvelopeStepName {
    #[default]
    InitialIdle,
    Attack,
    Decay,
    Sustain,
    Release,
    FinalIdle,
}

#[derive(Clone, Control, Debug, Uid)]
pub struct AdsrEnvelope {
    uid: usize,
    preset: EnvelopeSettings,

    #[controllable]
    note: PhantomData<u8>,

    envelope: SteppedEnvelope,
    note_on_time: f32,
    note_off_time: f32,
    end_work_time: f32,

    note_on_pending: bool,
    note_off_pending: bool,
    is_idle: bool,
}
impl IsInstrument for AdsrEnvelope {}
impl SourcesAudio for AdsrEnvelope {
    fn source_audio(&mut self, clock: &Clock) -> OldMonoSample {
        let time = self.envelope.time_for_unit(clock);
        let step = self.envelope.step_for_time(time);
        self.envelope.value_for_step_at_time(step, time)
    }
}
impl Updateable for AdsrEnvelope {
    type Message = EntityMessage;

    fn update(&mut self, clock: &Clock, message: Self::Message) -> Response<Self::Message> {
        match message {
            Self::Message::Midi(_, message) => match message {
                midly::MidiMessage::NoteOff { key: _, vel: _ } => {
                    self.handle_note_event(clock, false)
                }
                midly::MidiMessage::NoteOn { key: _, vel: _ } => {
                    self.handle_note_event(clock, true)
                }
                _ => {}
            },
            _ => todo!(),
        }
        Response::none()
    }
}
impl Default for AdsrEnvelope {
    fn default() -> Self {
        Self::new_with(&EnvelopeSettings::default())
    }
}
impl GeneratesEnvelope for AdsrEnvelope {
    fn enqueue_attack(&mut self) {
        self.note_on_pending = true;
    }

    fn enqueue_release(&mut self) {
        self.note_off_pending = true;
    }

    fn tick(&mut self, clock: &Clock) -> Normal {
        self.handle_pending(clock);
        let time = self.envelope.time_for_unit(clock);
        let step = self.envelope.step_for_time(time);
        self.is_idle = self.calculate_is_idle(clock);
        Normal::new(self.envelope.value_for_step_at_time(step, time) as f64)
    }

    fn is_idle(&self) -> bool {
        self.is_idle
    }
}

impl AdsrEnvelope {
    fn handle_pending(&mut self, clock: &Clock) {
        if self.note_on_pending {
            self.note_on_time = self.envelope.time_for_unit(clock);
            self.note_off_time = f32::MAX;
            self.handle_state_change();
            self.note_on_pending = false;
        }
        if self.note_off_pending {
            if self.note_on_time == f32::MAX {
                self.note_on_time = self.envelope.time_for_unit(clock);
            }
            self.note_off_time = self.envelope.time_for_unit(clock);
            self.handle_state_change();
            self.note_off_pending = false;
        }
    }

    fn calculate_is_idle(&self, clock: &Clock) -> bool {
        clock.seconds() < self.note_on_time || clock.seconds() >= self.end_work_time
    }

    pub(crate) fn handle_note_event(&mut self, clock: &Clock, note_on: bool) {
        if note_on {
            self.note_on_time = self.envelope.time_for_unit(clock);
            self.note_off_time = f32::MAX;
            self.handle_state_change();
        } else {
            // We don't touch the note-on time because that's still important to
            // build the right envelope shape, unless we got a note-off without
            // a prior note-on (which can happen), and in that case we'll fix it
            // up to now.
            if self.note_on_time == f32::MAX {
                self.note_on_time = self.envelope.time_for_unit(clock);
            }
            self.note_off_time = self.envelope.time_for_unit(clock);
            self.handle_state_change();
        }
    }

    // TODO: is this really used anywhere? If yes, then we either need to plumb
    // clock back through all the control infra, or else we need to figure out a
    // different way to communicate the control event to this special case,
    // e.g., storing away the note event and processing it at the next
    // source_audio(), when we will have a clock.
    pub(crate) fn set_control_note(&mut self, _value: F32ControlValue) {
        //        self.handle_note_event(clock, value.0 == 1.0);
    }

    fn handle_state_change(&mut self) {
        if self.note_on_time == f32::MAX {
            // We're waiting for a keypress; we have neither key-down nor key-up.
            // InitialIdle is only state.
            self.envelope.steps[AdsrEnvelopeStepName::InitialIdle as usize] =
                EnvelopeStep::new_with_duration(0.0, f32::MAX, 0.0, 0.0, EnvelopeFunction::Linear);
            self.end_work_time = f32::MAX;
            self.envelope.debug_validate_steps();
            return;
        }

        // We have at least a key-down.
        let dt = self.note_on_time; // "down time" as in key-down time
        let p = &self.preset;

        self.envelope.steps[AdsrEnvelopeStepName::InitialIdle as usize] =
            EnvelopeStep::new_with_duration(0.0, dt, 0.0, 0.0, EnvelopeFunction::Linear);

        // No matter whether we have a key-up yet, we want Attack to behave as if it's
        // going to complete normally, starting at 0, targeting 1, at the expected rate.
        self.envelope.steps[AdsrEnvelopeStepName::Attack as usize] =
            EnvelopeStep::new_with_duration(dt, p.attack, 0.0, 1.0, EnvelopeFunction::Linear);

        if self.note_off_time == f32::MAX {
            // We don't have a key-up, so let's build an envelope that ends on sustain.
            self.envelope.steps[AdsrEnvelopeStepName::Decay as usize] =
                EnvelopeStep::new_with_duration(
                    dt + p.attack,
                    p.decay,
                    1.0,
                    p.sustain,
                    EnvelopeFunction::Linear,
                );
            self.envelope.steps[AdsrEnvelopeStepName::Sustain as usize] =
                EnvelopeStep::new_with_duration(
                    dt + p.attack + p.decay,
                    f32::MAX,
                    p.sustain,
                    p.sustain,
                    EnvelopeFunction::Linear,
                );
            self.end_work_time = f32::MAX;
            self.envelope.debug_validate_steps();
            return;
        }

        // We do have a key-up. There are two cases: during Attack/Decay, or during Sustain.
        let ut = self.note_off_time;
        debug_assert_le!(dt, ut);

        let keydown_duration = ut - dt;
        let attack_decay_duration = p.attack + p.decay;
        if keydown_duration > attack_decay_duration {
            // normal case where key-up does not interrupt attack/decay.
            self.envelope.steps[AdsrEnvelopeStepName::Decay as usize] =
                EnvelopeStep::new_with_duration(
                    dt + p.attack,
                    p.decay,
                    1.0,
                    p.sustain,
                    EnvelopeFunction::Linear,
                );
            self.envelope.steps[AdsrEnvelopeStepName::Sustain as usize] =
                EnvelopeStep::new_with_end_time(
                    Range {
                        start: dt + p.attack + p.decay,
                        end: ut,
                    },
                    p.sustain,
                    p.sustain,
                    EnvelopeFunction::Linear,
                );
            self.envelope.steps[AdsrEnvelopeStepName::Release as usize] =
                EnvelopeStep::new_with_duration(
                    ut,
                    p.release,
                    p.sustain,
                    0.0,
                    EnvelopeFunction::Linear,
                );
            let final_idle_start_time = ut + p.release;
            self.envelope.steps[AdsrEnvelopeStepName::FinalIdle as usize] =
                EnvelopeStep::new_with_duration(
                    final_idle_start_time,
                    f32::MAX,
                    0.0,
                    0.0,
                    EnvelopeFunction::Linear,
                );
            self.end_work_time = final_idle_start_time;
        } else {
            // key-up happened during attack/decay.
            if keydown_duration >= p.attack {
                // Attack completed normally, and decay was midway. Let decay finish, skip sustain.
                self.envelope.steps[AdsrEnvelopeStepName::Decay as usize] =
                    EnvelopeStep::new_with_duration(
                        dt + p.attack,
                        p.decay,
                        1.0,
                        p.sustain,
                        EnvelopeFunction::Linear,
                    );
                self.envelope.steps[AdsrEnvelopeStepName::Sustain as usize] =
                    EnvelopeStep::new_with_duration(
                        dt + p.attack + p.decay,
                        0.0,
                        p.sustain,
                        p.sustain,
                        EnvelopeFunction::Linear,
                    );
                self.envelope.steps[AdsrEnvelopeStepName::Release as usize] =
                    EnvelopeStep::new_with_duration(
                        dt + p.attack + p.decay,
                        p.release,
                        p.sustain,
                        0.0,
                        EnvelopeFunction::Linear,
                    );
                let final_idle_start_time = dt + p.attack + p.decay + p.release;
                self.envelope.steps[AdsrEnvelopeStepName::FinalIdle as usize] =
                    EnvelopeStep::new_with_duration(
                        final_idle_start_time,
                        f32::MAX,
                        0.0,
                        0.0,
                        EnvelopeFunction::Linear,
                    );
                self.end_work_time = final_idle_start_time;
            } else {
                // Attack was interrupted. Pick current amplitude as ceiling, skip rest of attack, and move to decay.
                // Since we're picking a new ceiling, we'll scale the sustain level along with it so that the
                // envelope shape doesn't get weird (example: attack is interrupted at amplitude 0.1, but sustain was
                // 0.8. If we let decay do its thing going from "ceiling" to sustain, then it would go *up* rather than
                // down).
                let intercept_value = self.envelope.value_for_step_at_time(
                    &self.envelope.steps[AdsrEnvelopeStepName::Attack as usize],
                    ut,
                );
                let scaled_sustain = p.sustain * intercept_value;
                self.envelope.steps[AdsrEnvelopeStepName::Decay as usize] =
                    EnvelopeStep::new_with_duration(
                        ut,
                        p.decay,
                        intercept_value,
                        scaled_sustain,
                        EnvelopeFunction::Linear,
                    );
                self.envelope.steps[AdsrEnvelopeStepName::Sustain as usize] =
                    EnvelopeStep::new_with_duration(
                        ut + p.decay,
                        0.0,
                        scaled_sustain,
                        scaled_sustain,
                        EnvelopeFunction::Linear,
                    );
                self.envelope.steps[AdsrEnvelopeStepName::Release as usize] =
                    EnvelopeStep::new_with_duration(
                        ut + p.decay,
                        p.release,
                        scaled_sustain,
                        0.0,
                        EnvelopeFunction::Linear,
                    );
                let final_idle_start_time = ut + p.decay + p.release;
                self.envelope.steps[AdsrEnvelopeStepName::FinalIdle as usize] =
                    EnvelopeStep::new_with_duration(
                        final_idle_start_time,
                        f32::MAX,
                        0.0,
                        0.0,
                        EnvelopeFunction::Linear,
                    );
                self.end_work_time = final_idle_start_time;
            }
        }
        self.envelope.debug_validate_steps();
    }

    pub fn new_with(preset: &EnvelopeSettings) -> Self {
        let vec = vec![
            EnvelopeStep {
                // InitialIdle
                interval: Range {
                    start: 0.0,
                    end: f32::MAX,
                },
                start_value: 0.0,
                end_value: 0.0,
                step_function: EnvelopeFunction::Linear,
            },
            EnvelopeStep {
                // Attack
                interval: Range {
                    start: 0.0,
                    end: f32::MAX,
                },
                start_value: 0.0,
                end_value: 1.0,
                step_function: EnvelopeFunction::Linear,
            },
            EnvelopeStep {
                // Decay
                interval: Range {
                    start: 0.0,
                    end: f32::MAX,
                },
                start_value: 1.0,
                end_value: preset.sustain,
                step_function: EnvelopeFunction::Linear,
            },
            EnvelopeStep {
                // Sustain
                interval: Range {
                    start: 0.0,
                    end: f32::MAX,
                },
                start_value: preset.sustain,
                end_value: preset.sustain,
                step_function: EnvelopeFunction::Linear,
            },
            EnvelopeStep {
                // Release
                interval: Range {
                    start: 0.0,
                    end: f32::MAX,
                },
                start_value: preset.sustain,
                end_value: 0.0,
                step_function: EnvelopeFunction::Linear,
            },
            EnvelopeStep {
                // FinalIdle
                interval: Range {
                    start: 0.0,
                    end: f32::MAX,
                },
                start_value: 0.0,
                end_value: 0.0,
                step_function: EnvelopeFunction::Linear,
            },
        ];
        Self {
            uid: usize::default(),
            preset: *preset,
            note: Default::default(),
            envelope: SteppedEnvelope::new_with(ClockTimeUnit::Seconds, vec),
            note_on_time: f32::MAX,
            note_off_time: f32::MAX,
            end_work_time: f32::MAX,

            note_on_pending: false,
            note_off_pending: false,
            is_idle: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::Clock;
    use assert_approx_eq::assert_approx_eq;
    use float_cmp::approx_eq;
    use more_asserts::{assert_gt, assert_lt};

    impl SimpleEnvelope {
        fn debug_state(&self) -> &SimpleEnvelopeState {
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
            Normal::new(self.amplitude.current_sum())
        }
    }

    // temp for testing
    impl AdsrEnvelope {
        fn is_idle_for_time(&self, clock: &Clock) -> bool {
            self.calculate_is_idle(clock)
        }
    }

    #[test]
    fn test_envelope_mainline() {
        let ep = EnvelopeSettings {
            attack: 0.1,
            decay: 0.2,
            sustain: 0.8,
            release: 0.3,
        };
        let mut envelope = AdsrEnvelope::new_with(&ep);

        // Nobody has pressed a key, so it should be all silent.
        for t in 0..100 {
            let t_f32 = t as f32 / 10.0;
            let clock = Clock::debug_new_with_time(t_f32);
            assert_eq!(envelope.source_audio(&clock), 0.);
            assert!(envelope.is_idle_for_time(&clock));
        }

        // Now press a key. Make sure the sustaining part of the envelope is good.
        const NOTE_ON_TIMESTAMP: f32 = 0.5;
        envelope.handle_note_event(&Clock::debug_new_with_time(NOTE_ON_TIMESTAMP), true);

        assert_approx_eq!(envelope.source_audio(&Clock::debug_new_with_time(0.0)), 0.0);
        assert_approx_eq!(
            envelope.source_audio(&Clock::debug_new_with_time(NOTE_ON_TIMESTAMP + ep.attack)),
            1.0
        );
        assert_approx_eq!(
            envelope.source_audio(&Clock::debug_new_with_time(
                NOTE_ON_TIMESTAMP + ep.attack + ep.decay
            )),
            ep.sustain
        );
        assert_approx_eq!(
            envelope.source_audio(&Clock::debug_new_with_time(NOTE_ON_TIMESTAMP + 5.0)),
            ep.sustain
        );
        assert_approx_eq!(
            envelope.source_audio(&Clock::debug_new_with_time(NOTE_ON_TIMESTAMP + 10.0)),
            ep.sustain
        );
        assert!(envelope.is_idle_for_time(&Clock::debug_new_with_time(0.0)));
        assert!(!envelope.is_idle_for_time(&Clock::debug_new_with_time(NOTE_ON_TIMESTAMP)));
        assert!(!envelope.is_idle_for_time(&Clock::debug_new_with_time(
            NOTE_ON_TIMESTAMP + ep.attack + ep.decay
        )));
        assert!(!envelope.is_idle_for_time(&Clock::debug_new_with_time(NOTE_ON_TIMESTAMP + 10.0)));
        assert!(!envelope.is_idle_for_time(&Clock::debug_new_with_time(f32::MAX)));

        // Let the key go. Release should work.
        const NOTE_OFF_TIMESTAMP: f32 = 2.0;
        envelope.handle_note_event(&Clock::debug_new_with_time(NOTE_OFF_TIMESTAMP), false);

        assert_approx_eq!(envelope.source_audio(&Clock::debug_new_with_time(0.0)), 0.0);
        assert_approx_eq!(
            envelope.source_audio(&Clock::debug_new_with_time(NOTE_OFF_TIMESTAMP)),
            ep.sustain
        );
        assert_lt!(
            envelope.source_audio(&Clock::debug_new_with_time(NOTE_OFF_TIMESTAMP + 0.01)),
            ep.sustain
        );
        assert_approx_eq!(
            envelope.source_audio(&Clock::debug_new_with_time(
                NOTE_OFF_TIMESTAMP + ep.release / 2.0
            )),
            ep.sustain / 2.0
        );
        assert_approx_eq!(
            envelope.source_audio(&Clock::debug_new_with_time(NOTE_OFF_TIMESTAMP + ep.release)),
            0.0
        );
        assert_eq!(
            envelope.source_audio(&Clock::debug_new_with_time(
                NOTE_OFF_TIMESTAMP + ep.release + 0.1
            )),
            0.0
        );
        assert_eq!(
            envelope.source_audio(&Clock::debug_new_with_time(10.0)),
            0.0
        );

        assert!(envelope.is_idle_for_time(&Clock::debug_new_with_time(0.0)));
        assert!(!envelope.is_idle_for_time(&Clock::debug_new_with_time(NOTE_ON_TIMESTAMP)));
        assert!(!envelope.is_idle_for_time(&Clock::debug_new_with_time(NOTE_OFF_TIMESTAMP)));
        assert!(!envelope.is_idle_for_time(&Clock::debug_new_with_time(
            NOTE_OFF_TIMESTAMP + ep.release - 0.01
        )));
        assert!(
            envelope.is_idle_for_time(&Clock::debug_new_with_time(NOTE_OFF_TIMESTAMP + ep.release))
        );
    }

    #[test]
    fn test_envelope_interrupted_attack() {
        let ep = EnvelopeSettings {
            attack: 0.2,
            decay: 0.4,
            sustain: 0.8,
            release: 0.16,
        };
        let mut envelope = AdsrEnvelope::new_with(&ep);

        // Silence throughout (pick an arbitrary point of T0 + attack)
        assert_eq!(
            envelope.source_audio(&Clock::debug_new_with_time(ep.attack)),
            0.0
        );

        // Press a key at time zero to make arithmetic easier. Attack should be
        // complete at expected time.
        envelope.handle_note_event(&Clock::default(), true);
        assert_eq!(
            envelope.source_audio(&Clock::debug_new_with_time(ep.attack)),
            1.0
        );

        // But it turns out we release the key before attack completes! Decay should
        // commence as of wherever the amplitude was at that point.
        let how_far_through_attack = 0.3f32;
        let attack_timestamp = ep.attack * how_far_through_attack;
        let amplitude_at_timestamp = (1.0 - 0.0) * how_far_through_attack;
        const EPSILONISH: f32 = 0.05;
        envelope.handle_note_event(&Clock::debug_new_with_time(attack_timestamp), false);
        assert_approx_eq!(
            envelope.source_audio(&Clock::debug_new_with_time(attack_timestamp)),
            amplitude_at_timestamp
        );
        // Should be below local peak right before...
        assert_lt!(
            envelope.source_audio(&Clock::debug_new_with_time(attack_timestamp - EPSILONISH)),
            amplitude_at_timestamp
        );
        // and right after.
        assert_lt!(
            envelope.source_audio(&Clock::debug_new_with_time(attack_timestamp + EPSILONISH)),
            amplitude_at_timestamp
        );
        // and should decline through full expected release time to zero.
        assert_gt!(
            envelope.source_audio(&Clock::debug_new_with_time(
                attack_timestamp + ep.decay + ep.release - EPSILONISH
            )),
            0.0
        );
        assert_eq!(
            envelope.source_audio(&Clock::debug_new_with_time(
                attack_timestamp + ep.decay + ep.release
            )),
            0.0
        );
    }

    #[test]
    fn test_envelope_interrupted_decay() {
        let ep = EnvelopeSettings {
            attack: 0.2,
            decay: 0.4,
            sustain: 0.8,
            release: 0.16,
        };
        let mut envelope = AdsrEnvelope::new_with(&ep);

        // Press a key at time zero to make arithmetic easier. Attack should be
        // complete at expected time.
        envelope.handle_note_event(&Clock::default(), true);

        // We release the key mid-decay. Release should
        // commence as of wherever the amplitude was at that point.
        let how_far_through_decay = 0.3f32;
        let decay_timestamp = ep.attack + ep.decay * how_far_through_decay;
        envelope.handle_note_event(&Clock::debug_new_with_time(decay_timestamp), false);

        let amplitude_at_timestamp = 1.0 - (1.0 - ep.sustain) * how_far_through_decay;
        const EPSILONISH: f32 = 0.05;
        assert_approx_eq!(
            envelope.source_audio(&Clock::debug_new_with_time(decay_timestamp)),
            amplitude_at_timestamp
        );
        // Should be above right before...
        assert_gt!(
            envelope.source_audio(&Clock::debug_new_with_time(decay_timestamp - EPSILONISH)),
            amplitude_at_timestamp
        );
        // and below right after.
        assert_lt!(
            envelope.source_audio(&Clock::debug_new_with_time(decay_timestamp + EPSILONISH)),
            amplitude_at_timestamp
        );

        // and should decline through release time to zero.
        let end_of_envelope_timestamp = ep.attack + ep.decay + ep.release;
        assert_gt!(
            envelope.source_audio(&Clock::debug_new_with_time(
                end_of_envelope_timestamp - EPSILONISH
            )),
            0.0
        );
        assert_eq!(
            envelope.source_audio(&Clock::debug_new_with_time(end_of_envelope_timestamp)),
            0.0
        );
    }

    #[test]
    fn test_envelope_step_functions() {
        const START_TIME: f32 = 3.14159;
        const DURATION: f32 = 2.71828;
        const START_VALUE: f32 = 1.0;
        const END_VALUE: f32 = 1.0 + 10.0;

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
        let envelope = SimpleEnvelope::new_with(clock.sample_rate(), &envelope_settings);
        (envelope_settings, clock, envelope)
    }

    #[test]
    fn generates_envelope_trait_idle() {
        let (_envelope_settings, mut clock, mut e) = get_ge_trait_stuff();

        assert!(e.is_idle(), "Envelope should be idle on creation.");

        let amplitude = e.tick(&clock);
        clock.tick();
        assert!(e.is_idle(), "Untriggered envelope should remain idle.");
        assert_eq!(
            amplitude.value(),
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
        let mut amplitude;
        loop {
            amplitude = envelope.tick(&clock);
            let should_continue = clock.seconds() < time_marker;
            clock.tick();
            if !should_continue {
                break;
            }
            test(amplitude.value());
        }
        amplitude
    }

    #[test]
    fn generates_envelope_trait_instant_trigger_response() {
        let (_envelope_settings, mut clock, mut e) = get_ge_trait_stuff();

        e.enqueue_attack();
        let mut amplitude = e.tick(&clock);
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
            amplitude = e.tick(&clock);
            clock.tick();
        }
        assert_gt!(
            amplitude.value(),
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
        let mut envelope = SimpleEnvelope::new_with(clock.sample_rate(), &envelope_settings);

        envelope.enqueue_attack();
        envelope.tick(&clock);
        let mut time_marker = clock.seconds() + envelope_settings.attack;
        assert!(
            matches!(envelope.debug_state(), SimpleEnvelopeState::Attack),
            "Expected SimpleEnvelopeState::Attack after trigger, but got {:?} instead",
            envelope.debug_state()
        );
        clock.tick();

        let amplitude = run_until(&mut envelope, &mut clock, time_marker, |_amplitude| {});
        assert!(matches!(envelope.debug_state(), SimpleEnvelopeState::Decay));
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
            SimpleEnvelopeState::Sustain
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
        let mut envelope = SimpleEnvelope::new_with(clock.sample_rate(), &envelope_settings);

        envelope.enqueue_attack();
        envelope.tick(&clock);
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
        let mut envelope = SimpleEnvelope::new_with(clock.sample_rate(), &envelope_settings);

        let amplitude = envelope.tick(&clock);
        clock.tick();

        assert_eq!(
            amplitude,
            Normal::minimum(),
            "Amplitude should start at zero"
        );

        // See https://floating-point-gui.de/errors/comparison/ for standard
        // warning about comparing floats and looking for epsilons.
        envelope.enqueue_attack();
        let amplitude = envelope.tick(&clock);
        let mut time_marker = clock.seconds();
        clock.tick();
        assert!(
            approx_eq!(f64, amplitude.value(), Normal::maximum().value(), ulps = 8),
            "Amplitude should reach peak upon trigger"
        );

        let amplitude = envelope.tick(&clock);
        clock.tick();
        assert_lt!(
            amplitude,
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
        let _amplitude = envelope.tick(&clock);
        clock.tick();

        // And hit it again.
        envelope.enqueue_attack();
        let amplitude = envelope.tick(&clock);
        let mut time_marker = clock.seconds();
        clock.tick();
        assert!(
            approx_eq!(f64, amplitude.value(), Normal::maximum().value(), ulps = 8),
            "Amplitude should reach peak upon second trigger"
        );

        // Then release again.
        envelope.enqueue_release();

        // Check that we keep decreasing amplitude to zero, not to sustain.
        time_marker += envelope_settings.release;
        let mut last_amplitude = amplitude.value();
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
        let mut envelope = SimpleEnvelope::new_with(clock.sample_rate(), &envelope_settings);

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

    #[ignore]
    #[test]
    fn compare_old_and_new() {
        // These settings are copied from Welsh Piano's filter envelope, which
        // is where I noticed some unwanted behavior.
        let envelope_settings = EnvelopeSettings {
            attack: 0.0,
            decay: 5.22,
            sustain: 0.25,
            release: 0.5,
        };
        let mut clock = Clock::default();
        let mut old_envelope = AdsrEnvelope::new_with(&envelope_settings);
        let mut new_envelope = SimpleEnvelope::new_with(clock.sample_rate(), &envelope_settings);

        old_envelope.enqueue_attack();
        new_envelope.enqueue_attack();

        let time_marker = clock.seconds() + 10.0;
        let when_to_release = time_marker + 1.0;
        let mut has_released = false;
        loop {
            if clock.seconds() >= when_to_release && !has_released {
                has_released = true;
                old_envelope.enqueue_release();
                new_envelope.enqueue_release();
            }
            let old_amplitude = old_envelope.tick(&clock);
            let new_amplitude = new_envelope.tick(&clock);
            let should_continue = clock.seconds() < time_marker;
            assert_approx_eq!(old_amplitude.value(), new_amplitude.value(), 0.001);
            clock.tick();
            if !should_continue {
                break;
            }
        }
    }

    // https://docs.google.com/spreadsheets/d/1DSkut7rLG04Qx_zOy3cfI7PMRoGJVr9eaP5sDrFfppQ/edit#gid=0
    #[test]
    fn coeff() {
        let (a, b, c) = SimpleEnvelope::calculate_coefficients(0.0, 1.0, 0.5, 0.25, 1.0, 0.0);
        assert_eq!(a, 1.0);
        assert_eq!(b, -2.0);
        assert_eq!(c, 1.0);
    }
}
