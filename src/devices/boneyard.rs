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

//https://stackoverflow.com/questions/27831944/how-do-i-store-a-closure-in-a-struct-in-rust
#[derive(Default)]
pub struct Lfo {
    frequency: f32,
    current_value: f32,
    target: Option<Box<dyn FnMut(f32) -> ()>>,
}

impl Lfo {

    fn tick(&mut self, time_seconds: f32) -> bool {
        let phase_normalized = self.frequency * time_seconds;
        self.current_value = 2.0 * (phase_normalized - (0.5 + phase_normalized).floor());
        match &mut self.target {
            Some(tfn) => (tfn)(self.current_value),
            None => {}
        }
        true
    }

    fn get_audio_sample(&self) -> f32 {
        self.current_value
    }
    pub fn new(frequency: f32) -> Self {
        Self {
            frequency,
            current_value: 0.,
            target: Option::None,
        }
    }
    pub fn connect_automation_sink(&mut self, target: impl FnMut(f32) -> () + 'static) {
        self.target = Option::Some(Box::new(target));
    }
}

// TODO: is this just extra stuff hung off Oscillator?

#[cfg(test)]
mod tests {

    use std::{cell::RefCell, rc::Rc};

    use more_asserts::assert_gt;

    use crate::{
        primitives::{self, oscillators::Oscillator, clock::Clock},
    };

    use super::*;

    impl Lfo {
        fn new_test_1hz() -> Self {
            Self::new(1.)
        }
    }

    #[test]
    fn test_lfo_shape() {
        let mut clock = Clock::new_test();
        let mut lfo_1hz = Lfo::new_test_1hz();

        assert_eq!(lfo_1hz.frequency, 1.);

        lfo_1hz.tick(&clock);
        assert_eq!(lfo_1hz.get_audio_sample(), 0.);

        // test that sawtooth's first half is positive
        loop {
            clock.tick();
            lfo_1hz.tick(&clock);
            dbg!(clock.seconds);
            dbg!(lfo_1hz.get_audio_sample());
            if clock.seconds >= 0.5 {
                break;
            }
            assert!(lfo_1hz.get_audio_sample() > 0.);
        }
        assert_eq!(clock.samples, Clock::TEST_SAMPLE_RATE / 2);
        assert_eq!(lfo_1hz.get_audio_sample(), -1.);

        // test that sawtooth's second half is negative
        loop {
            clock.tick();
            lfo_1hz.tick(&clock);
            dbg!(clock.seconds);
            dbg!(lfo_1hz.get_audio_sample());
            if clock.seconds >= 1. {
                break;
            }
            assert!(lfo_1hz.get_audio_sample() < 0.);
        }
        assert_eq!(clock.samples, Clock::TEST_SAMPLE_RATE);
        assert_eq!(lfo_1hz.get_audio_sample(), 0.);
    }

    #[test]
    fn test_automation() {
        let mut clock = Clock::new_test();

        let oscillator = Rc::new(RefCell::new(Oscillator::new(
            primitives::oscillators::Waveform::Sine,
        )));
        oscillator.borrow_mut().set_frequency(440.);

        let mut lfo = Lfo::new_test_1hz();
        let o2 = oscillator.clone();
        let thefn = move |value: f32| -> () {
            let frequency = o2.borrow().get_frequency();
            let mut o = o2.borrow_mut();
            o.set_frequency(frequency + frequency * value * 0.05);
        };
        lfo.connect_automation_sink(thefn);

        oscillator.borrow_mut().tick(&clock);
        lfo.tick(&clock);
        assert_eq!(oscillator.borrow_mut().get_frequency(), 440.);

        clock.tick();
        oscillator.borrow_mut().tick(&clock);
        lfo.tick(&clock);
        assert_gt!(oscillator.borrow_mut().get_frequency(), 440.);
    }
}

#[derive(Default, Debug)]
pub struct Oscillator_ {
    waveform: Waveform,
    current_sample: f32,
    frequency: f32,

    noise_x1: u32,
    noise_x2: u32,
}

// TODO: these oscillators are pure in a logical sense, but they alias badly in the real world
// of discrete sampling. Investigate replacing with smoothed waveforms.
impl Oscillator_ {
    pub fn new(waveform: Waveform) -> Self {
        Self {
            waveform,
            noise_x1: 0x70f4f854,
            noise_x2: 0xe1e9f0a7,
            ..Default::default()
        }
    }
    pub fn get_frequency(&self) -> f32 {
        self.frequency
    }
    pub fn set_frequency(&mut self, frequency: f32) {
        self.frequency = frequency;
    }
}

impl DeviceTrait for Oscillator_ {
    fn sinks_midi(&self) -> bool {
        true
    }
    fn sources_audio(&self) -> bool {
        true
    }
    fn tick(&mut self, clock: &Clock) -> bool {
        let phase_normalized = self.frequency * (clock.seconds as f32);
        self.current_sample = match self.waveform {
            // https://en.wikipedia.org/wiki/Sine_wave
            Waveform::Sine => (phase_normalized * 2.0 * PI).sin(),
            // https://en.wikipedia.org/wiki/Square_wave
            Waveform::Square => (phase_normalized * 2.0 * PI).sin().signum(),
            // https://en.wikipedia.org/wiki/Triangle_wave
            Waveform::Triangle => {
                4.0 * (phase_normalized - (0.75 + phase_normalized).floor() + 0.25).abs() - 1.0
            }
            // https://en.wikipedia.org/wiki/Sawtooth_wave
            Waveform::Sawtooth => 2.0 * (phase_normalized - (0.5 + phase_normalized).floor()),
            // https://www.musicdsp.org/en/latest/Synthesis/216-fast-whitenoise-generator.html
            Waveform::Noise => {
                self.noise_x1 ^= self.noise_x2;
                let tmp = 2.0 * (self.noise_x2 as f32 - (u32::MAX as f32 / 2.0)) / u32::MAX as f32;
                (self.noise_x2, _) = self.noise_x2.overflowing_add(self.noise_x1);
                tmp
            }
        };
        true
    }
    fn handle_midi_message(&mut self, message: &MidiMessage, _clock: &Clock) {
        match message.status {
            midi::MidiMessageType::NoteOn => {
                self.frequency = message.to_frequency();
            }
            midi::MidiMessageType::NoteOff => {
                // TODO(miket): now that oscillators are in envelopes, they generally turn on but don't turn off.
                // these might not end up being full DeviceTrait devices, but rather owned/managed by synths.
                //self.frequency = 0.;
            }
        }
    }

    fn get_audio_sample(&self) -> f32 {
        self.current_sample
    }
}
