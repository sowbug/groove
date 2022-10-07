use std::fmt::Debug;

use more_asserts::{debug_assert_ge, debug_assert_le};

use crate::{
    common::MonoSample,
    midi::{MidiChannel, MidiMessage, MidiMessageType, MIDI_CHANNEL_RECEIVE_ALL},
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
    pub start_time: f32,
    pub end_time: f32,
    pub start_value: MonoSample,
    pub end_value: MonoSample,
}

impl EnvelopeStep {
    pub(crate) fn new_with_duration(
        start_time: f32,
        duration: f32,
        start_value: MonoSample,
        end_value: MonoSample,
    ) -> Self {
        Self {
            start_time,
            end_time: if duration == f32::MAX {
                duration
            } else {
                start_time + duration
            },
            start_value,
            end_value,
        }
    }

    pub(crate) fn new_with_end_time(
        start_time: f32,
        end_time: f32,
        start_value: MonoSample,
        end_value: MonoSample,
    ) -> Self {
        Self {
            start_time,
            end_time,
            start_value,
            end_value,
        }
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
    midi_channel: MidiChannel,
    preset: EnvelopePreset,
    time_unit: EnvelopeTimeUnit,
    steps: Vec<EnvelopeStep>,

    note_on_time: f32,
    note_off_time: f32,
}

impl Default for AdsrEnvelope {
    fn default() -> Self {
        Self {
            midi_channel: MIDI_CHANNEL_RECEIVE_ALL,
            preset: EnvelopePreset::default(),
            time_unit: EnvelopeTimeUnit::default(),
            steps: Vec::new(),
            note_on_time: f32::MAX,
            note_off_time: f32::MAX,
        }
    }
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
                    self.note_on_time = self.time_for_unit(clock);
                    self.note_off_time = f32::MAX;
                    self.handle_state_change();
                } else {
                    // We don't touch the note-on time because that's still important
                    // to build the right envelope shape.
                    self.note_off_time = self.time_for_unit(clock);
                    self.handle_state_change();
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
            MidiMessageType::NoteOn => self.handle_control(clock, &Primary { value: 1.0 }),
            MidiMessageType::NoteOff => self.handle_control(clock, &Primary { value: 0.0 }),
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
            debug_assert_le!(step.start_time, end_time); // Next step leaves no gaps (overlaps OK)
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

    fn handle_state_change(&mut self) {
        if self.note_on_time == f32::MAX {
            // We're waiting for a keypress; we have neither key-down nor key-up.
            // InitialIdle is only state.
            self.steps[AdsrEnvelopeStepName::InitialIdle as usize] =
                EnvelopeStep::new_with_duration(0.0, f32::MAX, 0.0, 0.0);
            self.debug_validate_steps();
            return;
        }

        // We have at least a key-down.
        let dt = self.note_on_time; // "down time" as in key-down time
        let p = &self.preset;

        self.steps[AdsrEnvelopeStepName::InitialIdle as usize] =
            EnvelopeStep::new_with_duration(0.0, dt, 0.0, 0.0);

        // No matter whether we have a key-up yet, we want Attack to behave as if it's
        // going to complete normally, starting at 0, targeting 1, at the expected rate.
        self.steps[AdsrEnvelopeStepName::Attack as usize] =
            EnvelopeStep::new_with_duration(dt, p.attack, 0.0, 1.0);

        if self.note_off_time == f32::MAX {
            // We don't have a key-up, so let's build an envelope that ends on sustain.
            self.steps[AdsrEnvelopeStepName::Decay as usize] =
                EnvelopeStep::new_with_duration(dt + p.attack, p.decay, 1.0, p.sustain);
            self.steps[AdsrEnvelopeStepName::Sustain as usize] = EnvelopeStep::new_with_duration(
                dt + p.attack + p.decay,
                f32::MAX,
                p.sustain,
                p.sustain,
            );
            self.debug_validate_steps();
            return;
        }

        // We do have a key-up. There are two cases: during Attack/Decay, or during Sustain.
        let ut = self.note_off_time;
        debug_assert_le!(dt, ut);

        let keydown_duration = ut - dt;
        let attack_decay_duration = p.attack + p.decay;
        if keydown_duration > attack_decay_duration {
            // normal case where key-up does not interrupt attack/decay.
            self.steps[AdsrEnvelopeStepName::Decay as usize] =
                EnvelopeStep::new_with_duration(dt + p.attack, p.decay, 1.0, p.sustain);
            self.steps[AdsrEnvelopeStepName::Sustain as usize] =
                EnvelopeStep::new_with_end_time(dt + p.attack + p.decay, ut, p.sustain, p.sustain);
            self.steps[AdsrEnvelopeStepName::Release as usize] =
                EnvelopeStep::new_with_duration(ut, p.release, p.sustain, 0.0);
            self.steps[AdsrEnvelopeStepName::FinalIdle as usize] =
                EnvelopeStep::new_with_duration(ut + p.release, f32::MAX, 0.0, 0.0);
        } else {
            // key-up happened during attack/decay.
            if keydown_duration >= p.attack {
                // Attack completed normally, and decay was midway. Let decay finish, skip sustain.
                self.steps[AdsrEnvelopeStepName::Decay as usize] =
                    EnvelopeStep::new_with_duration(dt + p.attack, p.decay, 1.0, p.sustain);
                self.steps[AdsrEnvelopeStepName::Sustain as usize] =
                    EnvelopeStep::new_with_duration(
                        dt + p.attack + p.decay,
                        0.0,
                        p.sustain,
                        p.sustain,
                    );
                self.steps[AdsrEnvelopeStepName::Release as usize] =
                    EnvelopeStep::new_with_duration(
                        dt + p.attack + p.decay,
                        p.release,
                        p.sustain,
                        0.0,
                    );
                self.steps[AdsrEnvelopeStepName::FinalIdle as usize] =
                    EnvelopeStep::new_with_duration(
                        dt + p.attack + p.decay + p.release,
                        f32::MAX,
                        0.0,
                        0.0,
                    );
            } else {
                // Attack was interrupted. Pick current amplitude as ceiling, skip rest of attack, and move to decay.
                // Since we're picking a new ceiling, we'll scale the sustain level along with it so that the
                // envelope shape doesn't get weird (example: attack is interrupted at amplitude 0.1, but sustain was
                // 0.8. If we let decay do its thing going from "ceiling" to sustain, then it would go *up* rather than
                // down).
                let intercept_value = self
                    .source_audio_for_step(ut, &self.steps[AdsrEnvelopeStepName::Attack as usize]);
                let scaled_sustain = p.sustain * intercept_value;
                self.steps[AdsrEnvelopeStepName::Decay as usize] =
                    EnvelopeStep::new_with_duration(ut, p.decay, intercept_value, scaled_sustain);
                self.steps[AdsrEnvelopeStepName::Sustain as usize] =
                    EnvelopeStep::new_with_duration(
                        ut + p.decay,
                        0.0,
                        scaled_sustain,
                        scaled_sustain,
                    );
                self.steps[AdsrEnvelopeStepName::Release as usize] =
                    EnvelopeStep::new_with_duration(ut + p.decay, p.release, scaled_sustain, 0.0);
                self.steps[AdsrEnvelopeStepName::FinalIdle as usize] =
                    EnvelopeStep::new_with_duration(ut + p.decay + p.release, f32::MAX, 0.0, 0.0);
            }
        }
        self.debug_validate_steps();
    }

    fn time_for_unit(&self, clock: &Clock) -> f32 {
        match self.time_unit {
            EnvelopeTimeUnit::Seconds => clock.seconds(),
            EnvelopeTimeUnit::Beats => clock.beats(),
            EnvelopeTimeUnit::Samples => todo!(),
        }
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

    fn source_audio_for_step(&self, time: f32, step: &EnvelopeStep) -> MonoSample {
        if step.start_time == step.end_time || step.start_value == step.end_value {
            return step.end_value;
        }
        let elapsed_time = time - step.start_time;
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

impl SourcesAudio for AdsrEnvelope {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        let time = match self.time_unit {
            EnvelopeTimeUnit::Seconds => clock.seconds(),
            EnvelopeTimeUnit::Beats => clock.beats(),
            EnvelopeTimeUnit::Samples => todo!(),
        };
        let step = self.current_step(time);
        self.source_audio_for_step(time, step)
    }
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;
    use more_asserts::{assert_gt, assert_lt};

    use crate::{clock::Clock, preset::EnvelopePreset};

    use super::*;

    #[test]
    fn test_envelope_mainline() {
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
        const NOTE_ON_TIMESTAMP: f32 = 0.5;
        envelope.handle_midi_for_channel(&Clock::debug_new_with_time(NOTE_ON_TIMESTAMP), &midi_on);

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
        let midi_off = MidiMessage::note_off_c4();
        const NOTE_OFF_TIMESTAMP: f32 = 2.0;
        envelope
            .handle_midi_for_channel(&Clock::debug_new_with_time(NOTE_OFF_TIMESTAMP), &midi_off);

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
        let ep = EnvelopePreset {
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
        let midi_on = MidiMessage::note_on_c4();
        envelope.handle_midi_for_channel(&Clock::new(), &midi_on);
        assert_eq!(
            envelope.source_audio(&Clock::debug_new_with_time(ep.attack)),
            1.0
        );

        // But it turns out we release the key before attack completes! Decay should
        // commence as of wherever the amplitude was at that point.
        let midi_off = MidiMessage::note_off_c4();
        let how_far_through_attack = 0.3f32;
        let attack_timestamp = ep.attack * how_far_through_attack;
        let amplitude_at_timestamp = (1.0 - 0.0) * how_far_through_attack;
        const EPSILONISH: f32 = 0.05;
        envelope.handle_midi_for_channel(&Clock::debug_new_with_time(attack_timestamp), &midi_off);
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
        let ep = EnvelopePreset {
            attack: 0.2,
            decay: 0.4,
            sustain: 0.8,
            release: 0.16,
        };
        let mut envelope = AdsrEnvelope::new_with(&ep);

        // Press a key at time zero to make arithmetic easier. Attack should be
        // complete at expected time.
        let midi_on = MidiMessage::note_on_c4();
        envelope.handle_midi_for_channel(&Clock::new(), &midi_on);

        // We release the key mid-decay. Release should
        // commence as of wherever the amplitude was at that point.
        let midi_off = MidiMessage::note_off_c4();
        let how_far_through_decay = 0.3f32;
        let decay_timestamp = ep.attack + ep.decay * how_far_through_decay;
        envelope.handle_midi_for_channel(&Clock::debug_new_with_time(decay_timestamp), &midi_off);

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
}
