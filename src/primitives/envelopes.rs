use std::{f32::MAX, fmt::Debug};

use crate::{
    common::{MidiMessage, MidiMessageType},
    preset::EnvelopePreset,
};

use super::{
    clock::Clock,
    SinksControl,
    SinksControlParamType::{self, Primary, Secondary},
    SourcesAudio, WatchesClock,
};

#[derive(Debug, Default)]
enum EnvelopeState {
    #[default]
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Default, Debug)]
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
    pub const MAX: f32 = -1.0;

    pub fn new_with(sample_rate: usize, preset: &EnvelopePreset) -> Self {
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
        if duration_seconds == MiniEnvelope::MAX {
            self.delta = 0.;
            self.target_seconds = f32::MAX;
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
            MidiMessageType::NoteOn => self.handle_note_on(time_seconds),
            MidiMessageType::NoteOff => self.handle_note_off(time_seconds),
            MidiMessageType::ProgramChange => {}
        }
    }

    fn handle_note_on(&mut self, time_seconds: f32) {
        self.switch_to_attack(time_seconds);
    }

    fn handle_note_off(&mut self, time_seconds: f32) {
        if !matches!(self.state, EnvelopeState::Idle) {
            self.switch_to_release(time_seconds);
        } else {
            // Already in idle state. Shouldn't happen.
        }
    }
}

impl SourcesAudio for MiniEnvelope {
    fn source_audio(&mut self, _clock: &Clock) -> crate::common::MonoSample {
        self.amplitude
    }
}

impl SinksControl for MiniEnvelope {
    fn handle_control(&mut self, clock: &Clock, param: &SinksControlParamType) {
        match param {
            Primary { value } => {
                if *value == 1.0 {
                    self.handle_note_on(clock.seconds)
                } else {
                    self.handle_note_off(clock.seconds)
                }
            }
            #[allow(unused_variables)]
            Secondary { value } => todo!(),
        }
    }
}

impl WatchesClock for MiniEnvelope {
    fn tick(&mut self, clock: &Clock) -> bool {
        self.amplitude += self.delta;
        match self.state {
            EnvelopeState::Idle => {}
            EnvelopeState::Attack => {
                if self.has_value_reached_target(clock.seconds) {
                    self.switch_to_decay(clock.seconds);
                }
            }
            EnvelopeState::Decay => {
                if self.has_value_reached_target(clock.seconds) {
                    self.switch_to_sustain(clock.seconds);
                }
            }
            EnvelopeState::Sustain => {
                // Just wait
            }
            EnvelopeState::Release => {
                if self.has_value_reached_target(clock.seconds) {
                    self.switch_to_idle(clock.seconds);
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use crate::{preset::EnvelopePreset, primitives::clock::Clock};

    use super::*;

    // TODO: this idea will work better if/when Envelope is a trait.
    #[derive(Default)]
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
    #[allow(unused_assignments)]
    fn test_mini_envelope() {
        let mut clock = Clock::new_test();
        let mut envelope = MiniEnvelope::new_with(
            clock.settings().sample_rate(),
            &EnvelopePreset {
                attack_seconds: 0.1,
                decay_seconds: 0.2,
                sustain_percentage: 0.8,
                release_seconds: 0.3,
            },
        );

        let midi_on = MidiMessage::note_on_c4();
        let midi_off = MidiMessage::note_off_c4();
        assert_eq!(envelope.amplitude, 0.);

        let mut last_recognized_time_point = -1.;
        loop {
            envelope.tick(&clock);
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
        let mut clock = Clock::new();
        let mut envelope = MiniEnvelope::new_with(
            clock.settings().sample_rate(),
            &EnvelopePreset {
                attack_seconds: 0.1,
                decay_seconds: 0.2,
                sustain_percentage: 0.8,
                release_seconds: 10.0,
            },
        );

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
        loop {
            envelope.tick(&clock);
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
