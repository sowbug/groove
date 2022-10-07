use std::fmt::Debug;

use more_asserts::{debug_assert_ge, debug_assert_le};

use crate::{
    common::MonoSample,
    midi::{MidiChannel, MidiMessage, MidiMessageType},
    preset::EnvelopePreset,
    traits::{
        ShapesEnvelope, SinksControl, SinksControlParam, SinksControlParam::Primary,
        SinksControlParam::Secondary, SinksMidi, SourcesAudio,
    },
};

use super::clock::Clock;

#[derive(Debug, Default)]
pub enum EnvelopeTimeUnit {
    #[default]
    Seconds,
    Beats,
    Samples,
}

#[derive(Debug, Default)]
pub struct EnvelopeStep {
    pub start_value: MonoSample,
    pub end_value: MonoSample,
    pub start_time: f32,
    pub end_time: f32,
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

#[derive(Debug, Default)]
pub struct AdsrEnvelope {
    midi_channel: MidiChannel,
    preset: EnvelopePreset,
    time_unit: EnvelopeTimeUnit,
    steps: Vec<EnvelopeStep>,
}

impl ShapesEnvelope for AdsrEnvelope {
    fn steps(&self) -> &[EnvelopeStep] {
        &self.steps
    }

    fn time_unit(&self) -> &EnvelopeTimeUnit {
        &self.time_unit
    }
}

impl SinksControl for AdsrEnvelope {
    fn handle_control(&mut self, clock: &Clock, param: &SinksControlParam) {
        match param {
            Primary { value } => {
                if *value == 1.0 {
                    self.handle_state_change(true, clock);
                } else {
                    self.handle_state_change(false, clock);
                }
            }
            #[allow(unused_variables)]
            Secondary { value } => todo!(),
        }
    }
}

impl SinksMidi for AdsrEnvelope {
    fn midi_channel(&self) -> MidiChannel {
        self.midi_channel
    }
    fn set_midi_channel(&mut self, midi_channel: MidiChannel) {
        self.midi_channel = midi_channel;
    }
    fn handle_midi_for_channel(&mut self, clock: &Clock, message: &MidiMessage) {
        match message.status {
            MidiMessageType::NoteOn => self.handle_state_change(true, clock),
            MidiMessageType::NoteOff => self.handle_state_change(false, clock),
            MidiMessageType::ProgramChange => {}
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

    fn debug_validate_steps(&self) {
        debug_assert!(!self.steps.is_empty());
        debug_assert_eq!(self.steps.first().unwrap().start_time, 0.0);
        debug_assert_eq!(self.steps.last().unwrap().end_time, f32::MAX);
        let mut start_time = 0.0;
        let mut end_time = 0.0;
        let steps = self.steps();
        for step in steps {
            debug_assert_le!(step.start_time, step.end_time); // Next step has non-negative duration
            debug_assert_ge!(step.start_time, start_time); // We're not moving backward in time
            debug_assert_eq!(step.start_time, end_time); // Next step leaves no gaps
            start_time = step.start_time;
            end_time = step.end_time;

            // We don't require subsequent steps to be valid, as long as
            // an earlier step covered the rest of the time range.
            if step.end_time == f32::MAX {
                break;
            }
        }
        debug_assert_eq!(end_time, f32::MAX);
    }

    fn set_step_time_interval(
        &mut self,
        step: AdsrEnvelopeStepName,
        start_time: f32,
        duration: f32,
    ) {
        self.steps[step as usize].start_time = start_time;

        self.steps[step as usize].end_time = if duration < f32::MAX {
            start_time + duration
        } else {
            f32::MAX
        };
    }

    fn clamp_step_start(
        &mut self,
        step: AdsrEnvelopeStepName,
        current_time: f32,
        current_value: Option<f32>,
    ) {
        let mut step = &mut self.steps[step as usize];
        if step.start_time >= current_time {
            if current_value.is_some() {
                step.start_value = current_value.unwrap();
            }
            step.start_time = current_time;
        }
    }

    fn clamp_step_end(
        &mut self,
        step: AdsrEnvelopeStepName,
        current_time: f32,
        current_value: Option<f32>,
    ) {
        let mut step = &mut self.steps[step as usize];
        if step.end_time >= current_time {
            if current_value.is_some() {
                step.end_value = current_value.unwrap();
            }
            step.end_time = current_time;
        }
    }

    fn handle_state_change(&mut self, is_note_on: bool, clock: &Clock) {
        let current_time = match self.time_unit {
            EnvelopeTimeUnit::Seconds => clock.seconds(),
            EnvelopeTimeUnit::Beats => clock.beats(),
            EnvelopeTimeUnit::Samples => todo!(),
        };
        if is_note_on {
            // The !note_on case will mutate the Attack and Decay steps, so we restore them here.
            self.steps[AdsrEnvelopeStepName::Attack as usize] = EnvelopeStep {
                start_value: 0.0,
                end_value: 1.0,
                start_time: 0.0,
                end_time: f32::MAX,
            };
            self.steps[AdsrEnvelopeStepName::Decay as usize] = EnvelopeStep {
                start_value: 1.0,
                end_value: self.preset.sustain,
                start_time: 0.0,
                end_time: f32::MAX,
            };

            self.set_step_time_interval(AdsrEnvelopeStepName::InitialIdle, 0.0, current_time);
            self.set_step_time_interval(
                AdsrEnvelopeStepName::Attack,
                current_time,
                self.preset.attack,
            );
            self.set_step_time_interval(
                AdsrEnvelopeStepName::Decay,
                current_time + self.preset.attack,
                self.preset.decay,
            );
            self.set_step_time_interval(
                AdsrEnvelopeStepName::Sustain,
                current_time + self.preset.attack + self.preset.decay,
                f32::MAX,
            );
            self.set_step_time_interval(AdsrEnvelopeStepName::Release, f32::MAX, f32::MAX);
            self.set_step_time_interval(AdsrEnvelopeStepName::FinalIdle, f32::MAX, f32::MAX);
        } else {
            let current_value = self.source_audio(clock);

            // We assume that the off happens after the on, and rely on the initial steps being
            // correct.
            self.clamp_step_end(
                AdsrEnvelopeStepName::Attack,
                current_time,
                Some(current_value),
            );
            self.clamp_step_start(
                AdsrEnvelopeStepName::Decay,
                current_time,
                Some(current_value),
            );
            self.clamp_step_end(AdsrEnvelopeStepName::Decay, current_time, None);
            self.clamp_step_start(AdsrEnvelopeStepName::Sustain, current_time, None);
            self.clamp_step_end(AdsrEnvelopeStepName::Sustain, current_time, None);
            self.set_step_time_interval(
                AdsrEnvelopeStepName::Release,
                current_time,
                self.preset.release,
            );
            self.set_step_time_interval(
                AdsrEnvelopeStepName::FinalIdle,
                current_time + self.preset.release,
                f32::MAX,
            );
        }
        self.debug_validate_steps();
    }

    pub fn new_with(preset: &EnvelopePreset) -> Self {
        let r = Self {
            preset: *preset,
            steps: vec![
                EnvelopeStep {
                    // InitialIdle
                    start_value: 0.0,
                    end_value: 0.0,
                    start_time: 0.0,
                    end_time: f32::MAX,
                },
                EnvelopeStep {
                    // Attack
                    start_value: 0.0,
                    end_value: 1.0,
                    start_time: 0.0,
                    end_time: f32::MAX,
                },
                EnvelopeStep {
                    // Decay
                    start_value: 1.0,
                    end_value: preset.sustain,
                    start_time: 0.0,
                    end_time: f32::MAX,
                },
                EnvelopeStep {
                    // Sustain
                    start_value: preset.sustain,
                    end_value: preset.sustain,
                    start_time: 0.0,
                    end_time: f32::MAX,
                },
                EnvelopeStep {
                    // Release
                    start_value: preset.sustain,
                    end_value: 0.0,
                    start_time: 0.0,
                    end_time: f32::MAX,
                },
                EnvelopeStep {
                    // FinalIdle
                    start_value: 0.0,
                    end_value: 0.0,
                    start_time: 0.0,
                    end_time: f32::MAX,
                },
            ],
            ..Default::default()
        };
        r.debug_validate_steps();
        r
    }
}

impl SourcesAudio for AdsrEnvelope {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        let current_time = match self.time_unit {
            EnvelopeTimeUnit::Seconds => clock.seconds(),
            EnvelopeTimeUnit::Beats => clock.beats(),
            EnvelopeTimeUnit::Samples => todo!(),
        };
        let step = self.current_step(current_time);

        if step.start_time == step.end_time || step.start_value == step.end_value {
            return step.end_value;
        }
        let elapsed_time = current_time - step.start_time;
        let total_interval_time = step.end_time - step.start_time;
        let percentage_complete = elapsed_time / total_interval_time;
        let total_interval_value_delta = step.end_value - step.start_value;
        let mut value = step.start_value + total_interval_value_delta * percentage_complete;
        if (step.end_value > step.start_value && value > step.end_value)
            || (step.end_value < step.start_value && value < step.end_value)
        {
            value = step.end_value;
        }
        value
    }
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;
    use more_asserts::assert_lt;

    use crate::{clock::Clock, preset::EnvelopePreset};

    use super::*;

    // TODO: this idea will work better if/when Envelope is a trait.
    #[derive(Debug, Default)]
    pub struct AlwaysLoudEnvelope {}

    #[allow(dead_code)]
    impl AlwaysLoudEnvelope {
        pub fn new() -> Self {
            Self {}
        }

        pub fn is_idle(&self) -> bool {
            false
        }

        pub fn tick(&self, _time_seconds: f32) {}

        pub fn handle_midi_message(&mut self, _message: &MidiMessage, _time_seconds: f32) {}

        pub fn value(&self) -> f32 {
            1.
        }
    }

    #[test]
    fn test_envelope() {
        let ep = EnvelopePreset {
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
        let midi_on = MidiMessage::note_on_c4();
        const NOTE_ON_TIME_SECONDS: f32 = 0.5;
        envelope
            .handle_midi_for_channel(&Clock::debug_new_with_time(NOTE_ON_TIME_SECONDS), &midi_on);

        assert_approx_eq!(envelope.source_audio(&Clock::debug_new_with_time(0.0)), 0.0);
        assert_approx_eq!(
            envelope.source_audio(&Clock::debug_new_with_time(
                NOTE_ON_TIME_SECONDS + ep.attack
            )),
            1.0
        );
        assert_approx_eq!(
            envelope.source_audio(&Clock::debug_new_with_time(
                NOTE_ON_TIME_SECONDS + ep.attack + ep.decay
            )),
            ep.sustain
        );
        assert_approx_eq!(
            envelope.source_audio(&Clock::debug_new_with_time(NOTE_ON_TIME_SECONDS + 5.0)),
            ep.sustain
        );
        assert_approx_eq!(
            envelope.source_audio(&Clock::debug_new_with_time(NOTE_ON_TIME_SECONDS + 10.0)),
            ep.sustain
        );

        // Let the key go. Release should work.
        let midi_off = MidiMessage::note_off_c4();
        const RELEASE_TIME_SECONDS: f32 = 2.0;
        envelope
            .handle_midi_for_channel(&Clock::debug_new_with_time(RELEASE_TIME_SECONDS), &midi_off);

        assert_approx_eq!(envelope.source_audio(&Clock::debug_new_with_time(0.0)), 0.0);
        assert_approx_eq!(
            envelope.source_audio(&Clock::debug_new_with_time(RELEASE_TIME_SECONDS)),
            ep.sustain
        );
        assert_lt!(
            envelope.source_audio(&Clock::debug_new_with_time(RELEASE_TIME_SECONDS + 0.01)),
            ep.sustain
        );
        assert_approx_eq!(
            envelope.source_audio(&Clock::debug_new_with_time(
                RELEASE_TIME_SECONDS + ep.release / 2.0
            )),
            ep.sustain / 2.0
        );
        assert_approx_eq!(
            envelope.source_audio(&Clock::debug_new_with_time(
                RELEASE_TIME_SECONDS + ep.release
            )),
            0.0
        );
        assert_eq!(
            envelope.source_audio(&Clock::debug_new_with_time(
                RELEASE_TIME_SECONDS + ep.release + 0.1
            )),
            0.0
        );
        assert_eq!(
            envelope.source_audio(&Clock::debug_new_with_time(10.0)),
            0.0
        );
    }

    //    #[test]
    #[allow(dead_code)]
    #[allow(unused_variables)]
    #[allow(unused_mut)]
    fn test_envelope_eventually_ends() {
        let mut clock = Clock::new();
        let mut envelope = AdsrEnvelope::new_with(&EnvelopePreset {
            attack: 0.1,
            decay: 0.2,
            sustain: 0.8,
            release: 10.0,
        });

        let midi_on = MidiMessage::note_on_c4();
        let midi_off = MidiMessage::note_off_c4();

        const SAMPLES_PER_SECOND: usize = 44100;
        const TIME_ZERO: usize = 0;
        const TIME_EXPECT_ATTACK: usize = 0; // because we check after firing keydown in same spin of event loop
        const DURATION_ATTACK: usize = 1 * SAMPLES_PER_SECOND / 10;
        const TIME_EXPECT_ATTACK_END: usize = TIME_EXPECT_ATTACK + DURATION_ATTACK - 1;
        const TIME_EXPECT_DECAY: usize = TIME_EXPECT_ATTACK + DURATION_ATTACK;
        const DURATION_DECAY: usize = 2 * SAMPLES_PER_SECOND / 10;
        const TIME_EXPECT_DECAY_END: usize = TIME_EXPECT_ATTACK + DURATION_DECAY - 1;
        const TIME_EXPECT_SUSTAIN: usize = TIME_EXPECT_DECAY + DURATION_DECAY;
        const TIME_EXPECT_RELEASE: usize = TIME_EXPECT_SUSTAIN;
        const DURATION_RELEASE: usize = 10 * SAMPLES_PER_SECOND;
        const TIME_EXPECT_RELEASE_END: usize = TIME_EXPECT_RELEASE + DURATION_RELEASE - 1;
        const TIME_EXPECT_IDLE: usize = TIME_EXPECT_RELEASE + DURATION_RELEASE;
        // loop {
        //     envelope.tick(&clock);
        //     match clock.samples() {
        //         TIME_ZERO => {
        //             assert!(matches!(envelope.state, EnvelopeState::Idle));
        //             envelope.handle_midi_message(&midi_on, clock.seconds);
        //             assert!(matches!(envelope.state, EnvelopeState::Attack));
        //         }
        //         TIME_EXPECT_ATTACK_END => {
        //             assert!(matches!(envelope.state, EnvelopeState::Attack));
        //         }
        //         TIME_EXPECT_DECAY => {
        //             assert!(matches!(envelope.state, EnvelopeState::Decay));
        //         }
        //         TIME_EXPECT_DECAY_END => {
        //             assert!(matches!(envelope.state, EnvelopeState::Decay));
        //         }
        //         TIME_EXPECT_SUSTAIN => {
        //             assert!(matches!(envelope.state, EnvelopeState::Sustain));
        //             envelope.handle_midi_message(&midi_off, clock.seconds);
        //             assert!(matches!(envelope.state, EnvelopeState::Release));
        //         }
        //         TIME_EXPECT_RELEASE_END => {
        //             assert!(matches!(envelope.state, EnvelopeState::Release));
        //         }
        //         TIME_EXPECT_IDLE => {
        //             assert!(matches!(envelope.state, EnvelopeState::Idle));
        //             break;
        //         }
        //         _ => {}
        //     };
        //     clock.tick();
        // }
    }
}
