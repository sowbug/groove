use std::{cell::RefCell, rc::Rc};

use crate::backend::{
    clock::Clock,
    devices::DeviceTrait,
    midi::{MidiMessage, MidiMessageType},
};

use super::oscillators::Oscillator;

#[derive(Debug)]
pub enum EnvelopeState {  // TODO: hide this again once CelloSynth2 proto is complete
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
        self.child_device.borrow_mut().handle_midi_message(message, clock);
    }
}
