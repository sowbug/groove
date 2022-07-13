use std::f32::MAX;

use crate::common::{MidiMessage, MidiMessageType};

#[derive(Default, Clone, Copy)]
pub struct MiniEnvelopePreset {
    pub attack_seconds: f32,
    pub decay_seconds: f32,
    pub sustain_percentage: f32,
    pub release_seconds: f32,
}

#[derive(Debug)]
enum EnvelopeState {
    // TODO: hide this again once CelloSynth2 proto is complete
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

impl Default for EnvelopeState {
    fn default() -> Self {
        EnvelopeState::Idle
    }
}

#[derive(Default)]
pub struct MiniEnvelope {
    sample_rate: f32,

    attack_seconds: f32,
    decay_seconds: f32,
    sustain_percentage: f32,
    release_seconds: f32,

    state: EnvelopeState,
    amplitude: f32,
    delta: f32,
    target: f32,
    target_seconds: f32,
}

impl MiniEnvelope {
    pub fn new(sample_rate: u32, preset: &MiniEnvelopePreset) -> Self {
        Self {
            sample_rate: sample_rate as f32,
            attack_seconds: preset.attack_seconds,
            decay_seconds: preset.decay_seconds,
            sustain_percentage: preset.sustain_percentage,
            release_seconds: preset.release_seconds,
            target_seconds: MAX,
            ..Default::default()
        }
    }

    pub fn is_idle(&self) -> bool {
        matches!(self.state, EnvelopeState::Idle)
    }

    pub fn value(&self) -> f32 {
        self.amplitude
    }

    fn has_value_reached_target(&self, current_time_seconds: f32) -> bool {
        self.target_seconds <= current_time_seconds
        // || ulps_eq!(self.amplitude, self.target, max_ulps = 6)
        // || (self.delta > 0. && self.amplitude > self.target)
        // || (self.delta < 0. && self.amplitude < self.target)
    }

    fn update_transition_target(
        &mut self,
        time_seconds: f32,
        current_value: f32,
        target_value: f32,
        duration_seconds: f32,
    ) {
        if duration_seconds == 0. {
            self.delta = 0.;
            self.target_seconds = time_seconds;
            return;
        }
        // The floor() is necessary because a tick is the lowest level of
        // granularity. Any rate must be in terms of integer-sized time
        // slices.
        self.delta =
            (target_value - current_value) / ((duration_seconds * self.sample_rate).floor());
        self.target_seconds = time_seconds + duration_seconds;
    }

    fn switch_to_attack(&mut self, time_seconds: f32) {
        if self.attack_seconds == 0. {
            self.switch_to_decay(time_seconds);
        } else {
            self.state = EnvelopeState::Attack;
            self.amplitude = 0.;
            self.target = 1.;
            self.update_transition_target(
                time_seconds,
                self.amplitude,
                self.target,
                self.attack_seconds,
            );
        }
    }

    fn switch_to_decay(&mut self, time_seconds: f32) {
        if self.decay_seconds == 0. {
            self.switch_to_sustain(time_seconds);
        } else {
            self.state = EnvelopeState::Decay;
            self.amplitude = 1.;
            self.target = self.sustain_percentage;
            self.update_transition_target(
                time_seconds,
                self.amplitude,
                self.target,
                self.decay_seconds,
            );
        }
    }

    fn switch_to_sustain(&mut self, _time_seconds: f32) {
        self.state = EnvelopeState::Sustain;
        self.amplitude = self.sustain_percentage;
        self.target = self.sustain_percentage; // irrelevant
        self.delta = 0.;
    }

    fn switch_to_release(&mut self, time_seconds: f32) {
        if self.release_seconds == 0. {
            self.switch_to_idle(time_seconds);
        } else {
            self.state = EnvelopeState::Release;
            self.target = 0.;
            self.update_transition_target(
                time_seconds,
                self.amplitude,
                self.target,
                self.release_seconds,
            );
        }
    }

    fn switch_to_idle(&mut self, _time_seconds: f32) {
        self.state = EnvelopeState::Idle;
        self.amplitude = 0.;
        self.target = 0.; // irrelevant
        self.delta = 0.;
    }

    pub fn handle_midi_message(&mut self, message: &MidiMessage, time_seconds: f32) {
        match message.status {
            MidiMessageType::NoteOn => {
                self.switch_to_attack(time_seconds);
            }
            MidiMessageType::NoteOff => {
                if !matches!(self.state, EnvelopeState::Idle) {
                    self.switch_to_release(time_seconds);
                } else {
                    // Already in idle state. Shouldn't happen.
                }
            }
            MidiMessageType::ProgramChange => {}
        }
    }

    pub fn tick(&mut self, time_seconds: f32) {
        self.amplitude += self.delta;
        match self.state {
            EnvelopeState::Idle => {}
            EnvelopeState::Attack => {
                if self.has_value_reached_target(time_seconds) {
                    self.switch_to_decay(time_seconds);
                }
            }
            EnvelopeState::Decay => {
                if self.has_value_reached_target(time_seconds) {
                    self.switch_to_sustain(time_seconds);
                }
            }
            EnvelopeState::Sustain => {
                // Just wait
            }
            EnvelopeState::Release => {
                if self.has_value_reached_target(time_seconds) {
                    self.switch_to_idle(time_seconds);
                }
            }
            _ => {}
        }
    }
}

// TODO: this idea will work better if/when Envelope is a trait.
#[derive(Default)]
pub struct AlwaysLoudEnvelope {}

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

#[cfg(test)]
mod tests {
    use crate::primitives::clock::Clock;

    use super::*;

    #[test]
    fn test_mini_envelope() {
        let mut clock = Clock::new_test();
        let mut envelope = MiniEnvelope::new(
            clock.sample_rate(),
            &MiniEnvelopePreset {
                attack_seconds: 0.1,
                decay_seconds: 0.2,
                sustain_percentage: 0.8,
                release_seconds: 0.3,
            },
        );

        let midi_on = MidiMessage {
            channel: 0,
            status: MidiMessageType::NoteOn,
            data1: 60,
            data2: 0,
        };
        let midi_off = MidiMessage {
            channel: 0,
            status: MidiMessageType::NoteOff,
            data1: 60,
            data2: 0,
        };
        assert_eq!(envelope.amplitude, 0.);

        let mut last_recognized_time_point = -1.;
        loop {
            envelope.tick(clock.seconds);
            if clock.seconds >= 0.0 && last_recognized_time_point < 0.0 {
                last_recognized_time_point = 0.0;
                assert!(matches!(envelope.state, EnvelopeState::Idle));
                envelope.handle_midi_message(&midi_on, clock.seconds);
            } else if matches!(envelope.state, EnvelopeState::Decay)
                && last_recognized_time_point < 0.1
            {
                last_recognized_time_point = 0.1;
                assert_eq!(envelope.amplitude, 1.0);
            } else if clock.seconds >= 0.1 + 0.2 && last_recognized_time_point < 0.1 + 0.2 {
                last_recognized_time_point = 0.1 + 0.2;
                assert_eq!(envelope.amplitude, 0.8);
                envelope.handle_midi_message(&midi_off, clock.seconds);
            } else if clock.seconds >= 0.1 + 0.2 + 0.3
                && last_recognized_time_point < 0.1 + 0.2 + 0.3
            {
                last_recognized_time_point = 0.1 + 0.2 + 0.3;
                assert_eq!(envelope.amplitude, 0.0);
                break;
            }
            clock.tick();
        }
    }

    #[test]
    fn test_envelope_eventually_ends() {
        let mut clock = Clock::new(44100, 4, 4, 128.0);
        let mut envelope = MiniEnvelope::new(
            clock.sample_rate(),
            &MiniEnvelopePreset {
                attack_seconds: 0.1,
                decay_seconds: 0.2,
                sustain_percentage: 0.8,
                release_seconds: 10.0,
            },
        );

        let midi_on = MidiMessage {
            channel: 0,
            status: MidiMessageType::NoteOn,
            data1: 60,
            data2: 0,
        };
        let midi_off = MidiMessage {
            channel: 0,
            status: MidiMessageType::NoteOff,
            data1: 60,
            data2: 0,
        };

        const SAMPLES_PER_SECOND: u32 = 44100;
        const TIME_ZERO: u32 = 0;
        const TIME_EXPECT_ATTACK: u32 = 0; // because we check after firing keydown in same spin of event loop
        const DURATION_ATTACK: u32 = 1 * SAMPLES_PER_SECOND / 10;
        const TIME_EXPECT_ATTACK_END: u32 = TIME_EXPECT_ATTACK + DURATION_ATTACK - 1;
        const TIME_EXPECT_DECAY: u32 = TIME_EXPECT_ATTACK + DURATION_ATTACK;
        const DURATION_DECAY: u32 = 2 * SAMPLES_PER_SECOND / 10;
        const TIME_EXPECT_DECAY_END: u32 = TIME_EXPECT_ATTACK + DURATION_DECAY - 1;
        const TIME_EXPECT_SUSTAIN: u32 = TIME_EXPECT_DECAY + DURATION_DECAY;
        const TIME_EXPECT_RELEASE: u32 = TIME_EXPECT_SUSTAIN;
        const DURATION_RELEASE: u32 = 10 * SAMPLES_PER_SECOND;
        const TIME_EXPECT_RELEASE_END: u32 = TIME_EXPECT_RELEASE + DURATION_RELEASE - 1;
        const TIME_EXPECT_IDLE: u32 = TIME_EXPECT_RELEASE + DURATION_RELEASE;
        loop {
            envelope.tick(clock.seconds);
            match clock.samples {
                TIME_ZERO => {
                    assert!(matches!(envelope.state, EnvelopeState::Idle));
                    envelope.handle_midi_message(&midi_on, clock.seconds);
                    assert!(matches!(envelope.state, EnvelopeState::Attack));
                }
                TIME_EXPECT_ATTACK_END => {
                    assert!(matches!(envelope.state, EnvelopeState::Attack));
                }
                TIME_EXPECT_DECAY => {
                    assert!(matches!(envelope.state, EnvelopeState::Decay));
                }
                TIME_EXPECT_DECAY_END => {
                    assert!(matches!(envelope.state, EnvelopeState::Decay));
                }
                TIME_EXPECT_SUSTAIN => {
                    assert!(matches!(envelope.state, EnvelopeState::Sustain));
                    envelope.handle_midi_message(&midi_off, clock.seconds);
                    assert!(matches!(envelope.state, EnvelopeState::Release));
                }
                TIME_EXPECT_RELEASE_END => {
                    assert!(matches!(envelope.state, EnvelopeState::Release));
                }
                TIME_EXPECT_IDLE => {
                    assert!(matches!(envelope.state, EnvelopeState::Idle));
                    break;
                }
                _ => {}
            };
            clock.tick();
        }
    }
}
