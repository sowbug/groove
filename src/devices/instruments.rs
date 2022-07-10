use super::midi::{MidiMessage, MidiMessageType};
use super::traits::DeviceTrait;
use crate::primitives::clock::Clock;
use crate::primitives::envelopes::{MiniEnvelope};
use crate::primitives::filter::{MiniFilter, MiniFilterType};
use crate::primitives::oscillators::{MiniOscillator, Waveform};
use std::cell::RefCell;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::rc::Rc;

pub struct Voice {
    envelope: MiniEnvelope,
}

impl Voice {
    pub fn new(waveform: Waveform) -> Self {
        let sound_source = Rc::new(RefCell::new(MiniOscillator::new(waveform, 0.0)));
        let envelope = MiniEnvelope::new(44100 /*TODO*/, 0.1, 0.1, 0.5, 0.3);
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

#[derive(Default)]
pub struct CelloSynth2 {
    is_playing: bool,
    current_value: f32,

    osc_1: MiniOscillator,
    osc_2: MiniOscillator,

    amp_envelope: MiniEnvelope,
    filter_envelope: MiniEnvelope,

    filter_1: MiniFilter,
    filter_2: MiniFilter,
}

impl CelloSynth2 {
    const OSC_1_PULSE_WIDTH: f32 = 0.1;

    const AMP_ENV_ATTACK_SECONDS: f32 = 0.06;
    const AMP_ENV_DECAY_SECONDS: f32 = 0.0;
    const AMP_ENV_SUSTAIN_PERCENTAGE: f32 = 1.;
    const AMP_ENV_RELEASE_SECONDS: f32 = 0.3;

    const FILTER_ENV_ATTACK_SECONDS: f32 = 0.0;
    const FILTER_ENV_DECAY_SECONDS: f32 = 3.29;
    const FILTER_ENV_SUSTAIN_PERCENTAGE: f32 = 0.78;
    const FILTER_ENV_RELEASE_SECONDS: f32 = 0.0;

    const LFO_FREQUENCY: f32 = 7.5;
    const LFO_DEPTH: f32 = 0.05;

    pub fn new(sample_rate: u32) -> Self {
        Self {
            osc_1: MiniOscillator::new_pwm_square(Self::OSC_1_PULSE_WIDTH, 0.),
            osc_2: MiniOscillator::new(Waveform::Square, 0.),
            amp_envelope: MiniEnvelope::new(
                sample_rate,
                Self::AMP_ENV_ATTACK_SECONDS,
                Self::AMP_ENV_DECAY_SECONDS,
                Self::AMP_ENV_SUSTAIN_PERCENTAGE,
                Self::AMP_ENV_RELEASE_SECONDS,
            ),
            filter_envelope: MiniEnvelope::new(
                sample_rate,
                Self::FILTER_ENV_ATTACK_SECONDS,
                Self::FILTER_ENV_DECAY_SECONDS,
                Self::FILTER_ENV_SUSTAIN_PERCENTAGE,
                Self::FILTER_ENV_RELEASE_SECONDS,
            ),
            filter_1: MiniFilter::new(MiniFilterType::SecondOrderLowPass, 44100, 40., 0.),
            filter_2: MiniFilter::new(MiniFilterType::FirstOrderLowPass, 44100, 40., 0.),
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

        let osc_mix = (self.osc_1.process(clock.seconds) + self.osc_2.process(clock.seconds)) / 2.;

        let phase_normalized_lfo = Self::LFO_FREQUENCY * (clock.seconds as f32);
        let lfo = (phase_normalized_lfo * 2.0 * PI).sin();

        const LPF_1_WEIGHT: f32 = 0.1;
        const LPF_2_WEIGHT: f32 = 0.1;
        let filter_1_weight = LPF_1_WEIGHT * self.filter_envelope.value();
        let filter_2_weight = LPF_2_WEIGHT * self.filter_envelope.value();
        let filter1 =
            self.filter_1.filter(osc_mix) * filter_1_weight + osc_mix * (1.0 - filter_1_weight);
        let filter2 =
            self.filter_2.filter(osc_mix) * filter_2_weight + osc_mix * (1.0 - filter_2_weight);
        let filter_mix = (filter1 + filter2) / 2.;

        let amplitude = self.amp_envelope.value()
            * filter_mix
            * (1. + lfo * Self::LFO_DEPTH - Self::LFO_DEPTH / 2.0);

        self.current_value = amplitude;

        // TODO temp
        self.amp_envelope.is_idle()
    }

    fn get_audio_sample(&self) -> f32 {
        self.current_value
    }
}

#[derive(Default)]
pub struct AngelsSynth {
    is_playing: bool,
    frequency: f32,
    current_value: f32,

    amp_envelope: MiniEnvelope,

    filter_1: MiniFilter,
    filter_2: MiniFilter,
}

impl AngelsSynth {
    const AMP_ENV_ATTACK_SECONDS: f32 = 0.32;
    const AMP_ENV_DECAY_SECONDS: f32 = 0.0;
    const AMP_ENV_SUSTAIN_PERCENTAGE: f32 = 1.;
    const AMP_ENV_RELEASE_SECONDS: f32 = 0.93;

    const LFO_FREQUENCY: f32 = 2.4;
    const LFO_DEPTH: f32 = 0.0002;

    pub fn new(sample_rate: u32) -> Self {
        Self {
            amp_envelope: MiniEnvelope::new(
                sample_rate,
                Self::AMP_ENV_ATTACK_SECONDS,
                Self::AMP_ENV_DECAY_SECONDS,
                Self::AMP_ENV_SUSTAIN_PERCENTAGE,
                Self::AMP_ENV_RELEASE_SECONDS,
            ),
            filter_1: MiniFilter::new(MiniFilterType::SecondOrderLowPass, 44100, 900., 0.7),
            filter_2: MiniFilter::new(MiniFilterType::FirstOrderLowPass, 44100, 900., 0.7),
            ..Default::default()
        }
    }
}

impl DeviceTrait for AngelsSynth {
    fn sources_audio(&self) -> bool {
        true
    }
    fn sinks_midi(&self) -> bool {
        true
    }

    fn handle_midi_message(&mut self, message: &MidiMessage, clock: &Clock) {
        self.amp_envelope
            .handle_midi_message(message, clock.seconds);
        match message.status {
            MidiMessageType::NoteOn => {
                self.is_playing = true;
                self.frequency = message.to_frequency();
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

        if self.amp_envelope.is_idle() {
            self.is_playing = false;
        }

        let phase_normalized_lfo = Self::LFO_FREQUENCY * (clock.seconds as f32);
        let lfo =
            4.0 * (phase_normalized_lfo - (0.75 + phase_normalized_lfo).floor() + 0.25).abs() - 1.0;

        let freq_lfo = self.frequency * (1. + lfo * Self::LFO_DEPTH);
        let phase_normalized_pitch = freq_lfo * (clock.seconds as f32);

        let osc1 = { 2.0 * (phase_normalized_pitch - (0.5 + phase_normalized_pitch).floor()) };

        let osc_mix = osc1;

        const LPF_1_WEIGHT: f32 = 0.55;
        const LPF_2_WEIGHT: f32 = 0.55;
        let filter_1_weight = LPF_1_WEIGHT * 1.0;
        let filter_2_weight = LPF_2_WEIGHT * 1.0;
        let filter1 =
            self.filter_1.filter(osc_mix) * filter_1_weight + osc_mix * (1.0 - filter_1_weight);
        let filter2 =
            self.filter_2.filter(osc_mix) * filter_2_weight + osc_mix * (1.0 - filter_2_weight);
        let filter_mix = (filter1 + filter2) / 2.;

        let amplitude = { self.amp_envelope.value() * filter_mix };

        self.current_value = amplitude;

        // TODO temp
        self.amp_envelope.is_idle()
    }

    fn get_audio_sample(&self) -> f32 {
        self.current_value
    }
}
