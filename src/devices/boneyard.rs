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

pub fn new_cello(sample_rate: u32) -> Self {
    Self::new(
        sample_rate,
        SimpleSynthPreset {
            oscillator_1_preset: OscillatorPreset {
                waveform: Waveform::Square(0.1),
                tune: 1.0,
                mix: 1.0,
            },
            oscillator_2_preset: OscillatorPreset {
                waveform: Waveform::Square(0.5),
                tune: 1.0,
                mix: 1.0,
            },
            amp_envelope_preset: MiniEnvelopePreset {
                attack_seconds: 0.06,
                decay_seconds: 0.0,
                sustain_percentage: 1.0,
                release_seconds: 0.3,
            },
            lfo_preset: LfoPreset {
                routing: LfoRouting::Amplitude,
                waveform: Waveform::Sine,
                frequency: 7.5,
                depth: 0.05,
            },
            filter_24db_type: MiniFilterType::FourthOrderLowPass(300.),
            filter_12db_type: MiniFilterType::SecondOrderLowPass(40., 0.),
            filter_24db_weight: 0.9,
            filter_12db_weight: 0.1,
            filter_envelope_preset: MiniEnvelopePreset {
                attack_seconds: 0.0,
                decay_seconds: 3.29,
                sustain_percentage: 0.78,
                release_seconds: 0.0,
            },
            filter_envelope_weight: 0.9,
        },
    )
}

pub fn new_angels(sample_rate: u32) -> Self {
    Self::new(
        sample_rate,
        SimpleSynthPreset {
            oscillator_1_preset: OscillatorPreset {
                waveform: Waveform::Sawtooth,
                ..Default::default()
            },
            oscillator_2_preset: OscillatorPreset {
                waveform: Waveform::None,
                ..Default::default()
            },
            amp_envelope_preset: MiniEnvelopePreset {
                attack_seconds: 0.32,
                decay_seconds: 0.0,
                sustain_percentage: 1.0,
                release_seconds: 0.93,
            },
            lfo_preset: LfoPreset {
                routing: LfoRouting::None,
                waveform: Waveform::Triangle,
                frequency: 2.4,
                depth: 0.0000119, // TODO 20 cents
            },
            filter_24db_type: MiniFilterType::FourthOrderLowPass(900.), // TODO: map Q to %
            filter_12db_type: MiniFilterType::SecondOrderLowPass(900., 1.0),
            filter_24db_weight: 0.85,
            filter_12db_weight: 0.25,
            filter_envelope_preset: MiniEnvelopePreset {
                attack_seconds: 0.,
                decay_seconds: 0.,
                sustain_percentage: 0.,
                release_seconds: 0.,
            },
            filter_envelope_weight: 0.0,
        },
    )
}

// TODO: this is an automation thing.
// maybe LFOs and envelopes shouldn't have audio output, but only value outputs.
// Then they don't have to get into the business of understanding the rest of DeviceTraits,
// and can be reused for more things.
//
// (this was in CelloSynth)
// From Welsh's Synthesizer Cookbook, page 53
//
// Osc1: PW 10%, mix 100%
// Osc2: Square, mix 100%, track on, sync off
// noise off
// LFO: route -> amplitude, sine, 7.5hz/moderate, depth 5%
// glide off unison off voices multi
// LP filter
//   24db cutoff 40hz 10%, resonance 0%, envelope 90%
//   12db cutoff 40hz 10%
//   ADSR 0s, 3.29s, 78%, max
// Amp envelope
//   ADSR 0.06s, max, 100%, 0.30s
//
// alternate: osc 1 sawtooth

pub struct SimpleSynth {
    voices: Vec<Voice>,
    note_to_voice: HashMap<u8, usize>,
    channel: u32,
}

impl SimpleSynth {
    pub fn new(waveform: Waveform, channel: u32) -> Self {
        const VOICE_COUNT: usize = 32;
        let mut synth = Self {
            voices: Vec::new(),
            note_to_voice: HashMap::<u8, usize>::new(),
            channel,
        };
        for _ in 0..VOICE_COUNT {
            synth.voices.push(Voice::new(waveform));
        }
        synth
    }
    fn next_available_voice(&self) -> usize {
        for i in 0..self.voices.len() {
            if !self.voices[i].is_active() {
                return i;
            }
        }
        // TODO: voice stealing
        0
    }

    pub fn temp_set_oscillator_frequency(&mut self, value: f32) {
        //self.voices[0].envelope.child_device.borrow_mut().set_frequency(value);
    }
}

impl DeviceTrait for SimpleSynth {
    fn sources_audio(&self) -> bool {
        true
    }
    fn sinks_midi(&self) -> bool {
        true
    }
    fn handle_midi_message(&mut self, message: &MidiMessage, clock: &Clock) {
        if message.channel as u32 != self.channel {
            // TODO: temp, eventually put responsibility on sender to filter
            return;
        }
        match message.status {
            MidiMessageType::NoteOn => {
                let index = self.next_available_voice();
                self.voices[index].handle_midi_message(message, clock);
                self.note_to_voice.insert(message.data1, index);
            }
            MidiMessageType::NoteOff => {
                let note = message.data1;
                let index: usize = *self.note_to_voice.get(&note).unwrap();
                self.voices[index].handle_midi_message(message, clock);
                self.note_to_voice.remove(&note);
            }
            MidiMessageType::ProgramChange => {
                panic!("asdfsdf");
            }
        }
    }
    fn tick(&mut self, clock: &Clock) -> bool {
        let mut is_everyone_done = true;
        for voice in self.voices.iter_mut() {
            is_everyone_done = voice.tick(clock) && is_everyone_done;
        }
        is_everyone_done
    }
    fn get_audio_sample(&self) -> f32 {
        let mut total_sample = 0.;
        for voice in self.voices.iter() {
            if voice.is_active() {
                total_sample += voice.get_audio_sample();
            }
        }
        // See https://www.kvraudio.com/forum/viewtopic.php?t=529789 for one discussion of
        // how to handle polyphonic note mixing (TLDR: just sum them and deal with > 1.0 in
        // a later limiter). If we do nothing then we get hard clipping for free (see
        // https://manual.audacityteam.org/man/limiter.html for terminology).
        total_sample
    }
}


#[derive(Default)]
pub struct CelloSynth2 {
    is_playing: bool,
    current_value: f32,

    osc_1: MiniOscillator,
    osc_2: MiniOscillator,
    osc_1_mix: f32,
    osc_2_mix: f32,
    amp_envelope: MiniEnvelope,

    lfo: MiniOscillator,
    lfo_routing: LfoRouting,
    lfo_depth: f32,

    filter: MiniFilter,
    filter_weight: f32,
    filter_envelope: MiniEnvelope,
    filter_envelope_weight: f32,
}

impl CelloSynth2 {
    pub fn new_calibration(sample_rate: u32) -> Self {
        Self::new(
            sample_rate,
            MiniSynthPreset {
                oscillator_1_preset: OscillatorPreset {
                    waveform: Waveform::Sawtooth,
                    tune: 1.0,
                    mix: 1.0,
                },
                oscillator_2_preset: OscillatorPreset {
                    waveform: Waveform::None,
                    tune: 4.0, // Two octaves
                    mix: 1.0,
                },
                amp_envelope_preset: MiniEnvelopePreset {
                    attack_seconds: 0.00,
                    decay_seconds: 0.0,
                    sustain_percentage: 1.0,
                    release_seconds: 0.0,
                },
                lfo_preset: LfoPreset {
                    routing: LfoRouting::Amplitude,
                    waveform: Waveform::Square(0.5),
                    frequency: 5.0,
                    depth: 1.0,
                    ..Default::default()
                },
                filter_24db_type: MiniFilterType::FourthOrderLowPass(440.),
                filter_12db_type: MiniFilterType::SecondOrderLowPass(440., 0.),
                filter_24db_weight: 0.1,
                filter_12db_weight: 0.0,
                filter_envelope_preset: MiniEnvelopePreset {
                    attack_seconds: 0.0,
                    decay_seconds: 0.0,
                    sustain_percentage: 1.0,
                    release_seconds: 0.0,
                },
                filter_envelope_weight: 1.0,
            },
        )
    }

    pub fn new(sample_rate: u32, preset: MiniSynthPreset) -> Self {
        Self {
            osc_1: MiniOscillator::new_from_preset(&preset.oscillator_1_preset),
            osc_2: MiniOscillator::new_from_preset(&preset.oscillator_2_preset),
            osc_1_mix: preset.oscillator_1_preset.mix,
            osc_2_mix: preset.oscillator_2_preset.mix,
            amp_envelope: MiniEnvelope::new(sample_rate, &preset.amp_envelope_preset),

            lfo: MiniOscillator::new_lfo(&preset.lfo_preset),
            lfo_routing: preset.lfo_preset.routing,
            lfo_depth: preset.lfo_preset.depth,

            filter: MiniFilter::new(44100, preset.filter_24db_type),
            filter_weight: preset.filter_24db_weight,
            filter_envelope: MiniEnvelope::new(sample_rate, &preset.filter_envelope_preset),
            filter_envelope_weight: preset.filter_envelope_weight,

            ..Default::default()
        }
    }
}

impl DeviceTrait for CelloSynth2 {
    fn sources_audio(&self) -> bool {
        true
    }
    fn sinks_midi(&self) -> bool {
        true
    }

    fn handle_midi_message(&mut self, message: &MidiMessage, clock: &Clock) {
        self.amp_envelope
            .handle_midi_message(message, clock.seconds);
        self.filter_envelope
            .handle_midi_message(message, clock.seconds);
        match message.status {
            MidiMessageType::NoteOn => {
                self.is_playing = true;
                let frequency = message.to_frequency();
                self.osc_1.set_frequency(frequency);
                self.osc_2.set_frequency(frequency);
            }
            MidiMessageType::NoteOff => {}
            MidiMessageType::ProgramChange => {}
        }

        if self.amp_envelope.is_idle() {
            self.is_playing = false;
        }
    }

    fn tick(&mut self, clock: &Clock) -> bool {
        self.amp_envelope.tick(clock.seconds);
        self.filter_envelope.tick(clock.seconds);

        if self.amp_envelope.is_idle() {
            self.is_playing = false;
        }

        let lfo = self.lfo.process(clock.seconds) * self.lfo_depth;
        if matches!(self.lfo_routing, LfoRouting::Pitch) {
            // Frequency assumes LFO [-1, 1]
            self.osc_1.set_frequency_modulation(lfo);
            self.osc_2.set_frequency_modulation(lfo);
        }

        let osc_1 = self.osc_1.process(clock.seconds);
        let osc_2 = self.osc_2.process(clock.seconds);
        let osc_mix = (osc_1 * self.osc_1_mix + osc_2 * self.osc_2_mix)
            / if !matches!(self.osc_2.waveform, Waveform::None) {
                2.0
            } else {
                1.0
            };

        self.current_value = {
            let filter_full_weight = self.filter_weight;
            let filter = self.filter.filter(osc_mix)
                * (1.0 + self.filter_envelope.value() * self.filter_envelope_weight);
            let filter_mix = filter * filter_full_weight + osc_mix * (1.0 - filter_full_weight);

            let lfo_amplitude_modulation = if matches!(self.lfo_routing, LfoRouting::Amplitude) {
                // Amplitude assumes LFO [0, 1]
                lfo / 2.0 + 0.5
            } else {
                1.0
            };
            self.amp_envelope.value() * filter_mix * lfo_amplitude_modulation
        };

        // TODO temp
        self.amp_envelope.is_idle()
    }

    fn get_audio_sample(&self) -> f32 {
        self.current_value
    }
}


impl Voice {
    pub fn new(waveform: Waveform) -> Self {
        let sound_source = Rc::new(RefCell::new(MiniOscillator::new(waveform)));
        let envelope = MiniEnvelope::new(
            44100, /*TODO*/
            &MiniEnvelopePreset {
                attack_seconds: 0.1,
                decay_seconds: 0.1,
                sustain_percentage: 0.5,
                release_seconds: 0.3,
            },
        );
        Self { envelope }
    }
    fn is_active(&self) -> bool {
        !self.envelope.is_idle()
    }
}

impl DeviceTrait for Voice {
    fn sources_audio(&self) -> bool {
        true
    }
    fn sinks_midi(&self) -> bool {
        true
    }
    fn handle_midi_message(&mut self, message: &MidiMessage, clock: &Clock) {
        self.envelope.handle_midi_message(message, clock.seconds);
    }
    fn tick(&mut self, clock: &Clock) -> bool {
        self.envelope.tick(clock.seconds);
        self.envelope.is_idle()
    }
    fn get_audio_sample(&self) -> f32 {
        self.envelope.value()
    }
}
