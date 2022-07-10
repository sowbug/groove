use std::{cell::RefCell, rc::Rc};

use crate::backend::{
    clock::Clock,
    devices::DeviceTrait,
    midi::{MidiMessage, MidiMessageType},
};

#[derive(Debug)]
pub enum EnvelopeState {
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

pub struct Envelope {
    pub child_device: Rc<RefCell<dyn DeviceTrait>>,
    amplitude: f32,
    amplitude_delta: f32,
    amplitude_target: f32,
    attack: f32,  // seconds
    decay: f32,   // seconds
    sustain: f32, // amplitude
    release: f32, // seconds

    state: EnvelopeState,
}

impl<'a> Envelope {
    pub fn new(
        child_device: Rc<RefCell<dyn DeviceTrait>>,
        attack: f32,
        decay: f32,
        sustain: f32,
        release: f32,
    ) -> Self {
        if !child_device.borrow().sources_audio() {
            panic!("Envelope created with non-audio-producing child device");
        }
        Self {
            child_device,
            amplitude: 0.,
            amplitude_delta: 0.,
            amplitude_target: 0.,
            attack,
            decay,
            sustain,
            release,
            state: EnvelopeState::Idle,
        }
    }

    fn update_amplitude_delta(&mut self, target: f32, state_duration: f32, clock: &Clock) {
        self.amplitude_target = target;
        if state_duration > 0. {
            self.amplitude_delta = (self.amplitude_target - self.amplitude)
                / (state_duration * clock.sample_rate() as f32);
        } else {
            self.amplitude_delta = self.amplitude_target - self.amplitude;
        }
    }

    fn change_to_attack_state(&mut self, clock: &Clock) {
        self.state = EnvelopeState::Attack;
        self.amplitude = 0.;
        self.update_amplitude_delta(1.0, self.attack, clock);
    }

    fn change_to_decay_state(&mut self, clock: &Clock) {
        self.state = EnvelopeState::Decay;
        self.amplitude = 1.;
        self.update_amplitude_delta(self.sustain, self.decay, clock);
    }

    fn change_to_sustain_state(&mut self, _clock: &Clock) {
        self.state = EnvelopeState::Sustain;
        self.amplitude = self.sustain;
        self.amplitude_target = self.sustain;
        self.amplitude_delta = 0.;
    }

    fn change_to_release_state(&mut self, clock: &Clock) {
        self.state = EnvelopeState::Release;
        self.update_amplitude_delta(0., self.release, clock);
    }

    fn change_to_idle_state(&mut self, _clock: &Clock) {
        self.state = EnvelopeState::Idle;
        self.amplitude = 0.;
        self.amplitude_delta = 0.;
    }

    fn has_amplitude_reached_target(&self) -> bool {
        (self.amplitude == self.amplitude_target)
            || (self.amplitude_delta < 0. && self.amplitude < self.amplitude_target)
            || (self.amplitude_delta > 0. && self.amplitude > self.amplitude_target)
    }

    pub fn is_active(&self) -> bool {
        !matches!(self.state, EnvelopeState::Idle)
    }
}

impl<'a> DeviceTrait for Envelope {
    fn sources_midi(&self) -> bool {
        true
    }

    fn tick(&mut self, clock: &Clock) -> bool {
        self.amplitude += self.amplitude_delta;
        if self.has_amplitude_reached_target() {
            match self.state {
                EnvelopeState::Idle => {
                    // Nothing to do but wait for note on
                }
                EnvelopeState::Attack => {
                    self.change_to_decay_state(clock);
                }
                EnvelopeState::Decay => {
                    self.change_to_sustain_state(clock);
                }
                EnvelopeState::Sustain => {
                    // Nothing to do but wait for note off
                }
                EnvelopeState::Release => {
                    self.change_to_idle_state(clock);
                }
            }
        }
        // TODO(miket): introduce notion of weak ref so that we can make sure nobody has two parents
        self.child_device.borrow_mut().tick(clock);

        matches!(self.state, EnvelopeState::Idle)
    }

    fn get_audio_sample(&self) -> f32 {
        self.child_device.borrow().get_audio_sample() * self.amplitude
    }

    fn handle_midi_message(&mut self, message: &MidiMessage, clock: &Clock) {
        match message.status {
            MidiMessageType::NoteOn => {
                self.change_to_attack_state(clock);
            }
            MidiMessageType::NoteOff => {
                self.change_to_release_state(clock);
            }
        }
        self.child_device
            .borrow_mut()
            .handle_midi_message(message, clock);
    }
}

#[derive(Default)]
pub struct MiniEnvelope {
    attack_seconds: f32,
    decay_seconds: f32,
    sustain_percentage: f32,
    release_seconds: f32,

    state: EnvelopeState,
    amplitude: f32,
    delta: f32,
    target: f32,
}

impl MiniEnvelope {
    pub fn new(
        attack_seconds: f32,
        decay_seconds: f32,
        sustain_percentage: f32,
        release_seconds: f32,
    ) -> Self {
        Self {
            attack_seconds,
            decay_seconds,
            sustain_percentage,
            release_seconds,
            ..Default::default()
        }
    }

    pub fn is_idle(&self) -> bool {
        matches!(self.state, EnvelopeState::Idle)
    }

    pub fn value(&self) -> f32 {
        self.amplitude
    }

    fn has_value_reached_target(&self) -> bool {
        self.amplitude == self.target
            || (self.delta > 0. && self.amplitude > self.target)
            || (self.delta < 0. && self.amplitude < self.target)
    }

    fn delta_for_transition(current: f32, target: f32, seconds: f32, ticks_per_second: f32) -> f32 {
        if seconds == 0. {
            return 0.;
        }
        // The floor() is necessary because a tick is the lowest level of
        // granularity. Any rate must be in terms of integer-sized time
        // slices.
        (target - current) / ((seconds * ticks_per_second).floor())
    }

    fn switch_to_attack(&mut self, clock: &Clock) {
        if self.attack_seconds == 0. {
            self.switch_to_decay(clock);
        } else {
            self.state = EnvelopeState::Attack;
            self.amplitude = 0.;
            self.target = 1.;
            self.delta = MiniEnvelope::delta_for_transition(
                self.amplitude,
                self.target,
                self.attack_seconds,
                clock.sample_rate() as f32,
            );
        }
    }

    fn switch_to_decay(&mut self, clock: &Clock) {
        if self.decay_seconds == 0. {
            self.switch_to_sustain(clock);
        } else {
            self.state = EnvelopeState::Decay;
            self.amplitude = 1.;
            self.target = self.sustain_percentage;
            self.delta = MiniEnvelope::delta_for_transition(
                self.amplitude,
                self.target,
                self.decay_seconds,
                clock.sample_rate() as f32,
            );
        }
    }

    fn switch_to_sustain(&mut self, clock: &Clock) {
        self.state = EnvelopeState::Sustain;
        self.amplitude = self.sustain_percentage;
        self.target = self.sustain_percentage; // irrelevant
        self.delta = 0.;
    }

    fn switch_to_release(&mut self, clock: &Clock) {
        if self.release_seconds == 0. {
            self.switch_to_idle(clock);
        } else {
            self.state = EnvelopeState::Release;
            self.target = 0.;
            self.delta = MiniEnvelope::delta_for_transition(
                self.amplitude,
                self.target,
                self.release_seconds,
                clock.sample_rate() as f32,
            );
        }
    }

    fn switch_to_idle(&mut self, clock: &Clock) {
        self.state = EnvelopeState::Idle;
        self.amplitude = 0.;
        self.target = 0.; // irrelevant
        self.delta = 0.;
    }

    pub fn handle_midi_message(&mut self, message: &MidiMessage, clock: &Clock) {
        match message.status {
            MidiMessageType::NoteOn => {
                self.switch_to_attack(clock);
            }
            MidiMessageType::NoteOff => {
                if !matches!(self.state, EnvelopeState::Idle) {
                    self.switch_to_release(clock);
                } else {
                    // Already in idle state. Shouldn't happen.
                }
            }
        }
    }

    pub fn tick(&mut self, clock: &Clock) {
        self.amplitude += self.delta;
        match self.state {
            EnvelopeState::Idle => {}
            EnvelopeState::Attack => {
                if self.has_value_reached_target() {
                    self.switch_to_decay(clock);
                }
            }
            EnvelopeState::Decay => {
                if self.has_value_reached_target() {
                    self.switch_to_sustain(clock);
                }
            }
            EnvelopeState::Sustain => {
                // Just wait
            }
            EnvelopeState::Release => {
                if self.has_value_reached_target() {
                    self.switch_to_idle(clock);
                }
            }
            _ => {}
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mini_envelope() {
        let mut envelope = MiniEnvelope::new(0.1, 0.2, 0.8, 0.3);
        let mut clock = Clock::new_test();

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
            envelope.tick(&clock);
            if clock.seconds >= 0.0 && last_recognized_time_point < 0.0 {
                last_recognized_time_point = 0.0;
                assert!(matches!(envelope.state, EnvelopeState::Idle));
                envelope.handle_midi_message(&midi_on, &clock);
            } else if matches!(envelope.state, EnvelopeState::Decay)
                && last_recognized_time_point < 0.1
            {
                last_recognized_time_point = 0.1;
                assert_eq!(envelope.amplitude, 1.0);
            } else if clock.seconds >= 0.1 + 0.2 && last_recognized_time_point < 0.1 + 0.2 {
                last_recognized_time_point = 0.1 + 0.2;
                assert_eq!(envelope.amplitude, 0.8);
                envelope.handle_midi_message(&midi_off, &clock);
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
}
