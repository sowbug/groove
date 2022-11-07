use super::clock::Clock;
use crate::{
    clock::ClockTimeUnit,
    common::{rrc, rrc_downgrade, weak_new, MonoSample, Rrc, Ww},
    settings::patches::EnvelopeSettings,
    traits::{HasOverhead, Overhead, SourcesAudio},
};
use more_asserts::{debug_assert_ge, debug_assert_le};
use std::{fmt::Debug, ops::Range};

#[derive(Debug, Default)]
pub enum EnvelopeFunction {
    #[default]
    Linear,
    Logarithmic,
    Exponential,
}

#[derive(Debug, Default)]
pub struct EnvelopeStep {
    pub interval: Range<f32>,
    pub start_value: MonoSample,
    pub end_value: MonoSample,
    pub step_function: EnvelopeFunction,
}

impl EnvelopeStep {
    pub(crate) fn new_with_duration(
        start_time: f32,
        duration: f32,
        start_value: MonoSample,
        end_value: MonoSample,
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
        start_value: MonoSample,
        end_value: MonoSample,
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

#[derive(Debug, Default)]
pub struct SteppedEnvelope {
    overhead: Overhead,
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
            ..Default::default()
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

    pub(crate) fn value_for_step_at_time(&self, step: &EnvelopeStep, time: f32) -> MonoSample {
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
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        let time = self.time_for_unit(clock);
        let step = self.step_for_time(time);
        self.value_for_step_at_time(step, time)
    }
}
impl HasOverhead for SteppedEnvelope {
    fn overhead(&self) -> &Overhead {
        &self.overhead
    }

    fn overhead_mut(&mut self) -> &mut Overhead {
        &mut self.overhead
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

#[derive(Debug)]
pub struct AdsrEnvelope {
    pub(crate) me: Ww<Self>,
    overhead: Overhead,
    preset: EnvelopeSettings,

    envelope: SteppedEnvelope,

    note_on_time: f32,
    note_off_time: f32,
}

impl Default for AdsrEnvelope {
    fn default() -> Self {
        Self {
            me: weak_new(),
            overhead: Overhead::default(),
            preset: EnvelopeSettings::default(),
            envelope: SteppedEnvelope::default(),
            note_on_time: f32::MAX,
            note_off_time: f32::MAX,
        }
    }
}

impl AdsrEnvelope {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    pub(crate) fn new_wrapped_with(preset: &EnvelopeSettings) -> Rrc<Self> {
        let wrapped = rrc(Self::new_with(preset));
        wrapped.borrow_mut().me = rrc_downgrade(&wrapped);
        wrapped
    }

    pub(crate) fn is_idle(&self, clock: &Clock) -> bool {
        let current_time = self.envelope.time_for_unit(clock);
        let step = self.envelope.step_for_time(current_time);
        step.end_value == step.start_value && step.interval.end == f32::MAX
    }

    pub(crate) fn handle_note_event(&mut self, clock: &Clock, note_on: bool) {
        if note_on {
            self.note_on_time = self.envelope.time_for_unit(clock);
            self.note_off_time = f32::MAX;
            self.handle_state_change();
        } else {
            // We don't touch the note-on time because that's still important
            // to build the right envelope shape.
            self.note_off_time = self.envelope.time_for_unit(clock);
            self.handle_state_change();
        }
    }

    fn handle_state_change(&mut self) {
        if self.note_on_time == f32::MAX {
            // We're waiting for a keypress; we have neither key-down nor key-up.
            // InitialIdle is only state.
            self.envelope.steps[AdsrEnvelopeStepName::InitialIdle as usize] =
                EnvelopeStep::new_with_duration(0.0, f32::MAX, 0.0, 0.0, EnvelopeFunction::Linear);
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
            self.envelope.steps[AdsrEnvelopeStepName::FinalIdle as usize] =
                EnvelopeStep::new_with_duration(
                    ut + p.release,
                    f32::MAX,
                    0.0,
                    0.0,
                    EnvelopeFunction::Linear,
                );
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
                self.envelope.steps[AdsrEnvelopeStepName::FinalIdle as usize] =
                    EnvelopeStep::new_with_duration(
                        dt + p.attack + p.decay + p.release,
                        f32::MAX,
                        0.0,
                        0.0,
                        EnvelopeFunction::Linear,
                    );
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
                self.envelope.steps[AdsrEnvelopeStepName::FinalIdle as usize] =
                    EnvelopeStep::new_with_duration(
                        ut + p.decay + p.release,
                        f32::MAX,
                        0.0,
                        0.0,
                        EnvelopeFunction::Linear,
                    );
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
            preset: *preset,
            envelope: SteppedEnvelope::new_with(ClockTimeUnit::Seconds, vec),
            ..Default::default()
        }
    }
}
impl SourcesAudio for AdsrEnvelope {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        let time = self.envelope.time_for_unit(clock);
        let step = self.envelope.step_for_time(time);
        self.envelope.value_for_step_at_time(step, time)
    }
}
impl HasOverhead for AdsrEnvelope {
    fn overhead(&self) -> &Overhead {
        &self.overhead
    }

    fn overhead_mut(&mut self) -> &mut Overhead {
        &mut self.overhead
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::Clock;
    use assert_approx_eq::assert_approx_eq;
    use more_asserts::{assert_gt, assert_lt};

    mod adsr {
        use crate::envelopes::AdsrEnvelope;

        fn instance() -> AdsrEnvelope {
            return AdsrEnvelope::default();
        }

        include!("test_trait_makes_control_sink.rs");
    }

    mod stepped_envelope {
        use crate::envelopes::SteppedEnvelope;

        fn instance() -> SteppedEnvelope {
            return SteppedEnvelope::default();
        }

        include!("test_trait_sources_audio.rs");
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
        envelope.handle_note_event(&Clock::new(), true);
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
        envelope.handle_note_event(&Clock::new(), true);

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

        let mut envelope = SteppedEnvelope::new();
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

        let mut envelope = SteppedEnvelope::new();
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

        let mut envelope = SteppedEnvelope::new();
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
}
