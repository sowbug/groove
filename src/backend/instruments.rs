use super::clock::Clock;
use super::devices::DeviceTrait;
use super::midi::{MidiMessage, MidiMessageType};
use crate::backend::midi;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::f32::consts::PI;
use std::rc::Rc;

#[derive(Eq, PartialEq)]
pub enum Waveform {
    Sine,
    Square,
    Triangle,
    Sawtooth,
}

impl Default for Waveform {
    fn default() -> Self {
        Waveform::Sine
    }
}

#[derive(Default)]
pub struct Oscillator {
    waveform: Waveform,
    current_sample: f32,
    frequency: f32,
}

// TODO: these oscillators are pure in a logical sense, but they alias badly in the real world
// of discrete sampling. Investigate replacing with smoothed waveforms.
impl Oscillator {
    pub fn new(waveform: Waveform) -> Oscillator {
        Oscillator {
            waveform,
            ..Default::default()
        }
    }
    pub fn set_frequency(&mut self, frequency: f32) {
        self.frequency = frequency;
    }
}
impl DeviceTrait for Oscillator {
    fn sinks_midi(&self) -> bool {
        true
    }
    fn sources_audio(&self) -> bool {
        true
    }
    fn tick(&mut self, clock: &Clock) -> bool {
        if self.frequency > 0. {
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
            }
        } else {
            self.current_sample = 0.
        }
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

pub struct TimeSignature {
    numerator: usize,
    denominator: usize,
}

impl TimeSignature {
    pub fn new(numerator: usize, denominator: usize) -> TimeSignature {
        TimeSignature {
            numerator,
            denominator,
        }
    }
}
pub struct Sequencer {
    time_signature: TimeSignature,
    midi_ticks_per_second: usize,
    sinks: Vec<Rc<RefCell<dyn DeviceTrait>>>,
    midi_messages: VecDeque<(usize, MidiMessage)>,
}

impl Sequencer {
    pub fn new() -> Sequencer {
        Sequencer {
            time_signature: TimeSignature::new(4, 4),
            midi_ticks_per_second: 0,
            sinks: Vec::new(),
            midi_messages: VecDeque::new(),
        }
    }

    pub fn set_time_signature(&mut self, numerator: usize, denominator: usize) {
        self.time_signature = TimeSignature::new(numerator, denominator);
    }

    pub fn set_midi_ticks_per_second(&mut self, tps: usize) {
        self.midi_ticks_per_second = tps;
    }

    pub fn add_message(&mut self, when: usize, message: MidiMessage) {
        self.midi_messages.push_back((when, message));
    }
    pub fn add_note_on(&mut self, when: usize, which: u8) {
        let midi_message = MidiMessage {
            status: MidiMessageType::NoteOn,
            channel: 0,
            data1: which,
            data2: 0,
        };
        self.midi_messages.push_back((when, midi_message));
    }
    pub fn add_note_off(&mut self, when: usize, which: u8) {
        let midi_message = MidiMessage {
            status: MidiMessageType::NoteOff,
            channel: 0,
            data1: which,
            data2: 0,
        };
        self.midi_messages.push_back((when, midi_message));
    }
}

impl DeviceTrait for Sequencer {
    fn sources_midi(&self) -> bool {
        true
    }

    fn tick(&mut self, clock: &Clock) -> bool {
        if self.midi_messages.is_empty() {
            return true;
        }
        let (when, midi_message) = self.midi_messages.front().unwrap();

        // TODO(miket): I'm getting a bad feeling about the usize and f32 conversions.
        let elapsed_midi_ticks: usize =
            ((clock.seconds * self.midi_ticks_per_second as f32) as u32) as usize;
        if elapsed_midi_ticks >= *when {
            for i in self.sinks.clone() {
                i.borrow_mut().handle_midi_message(midi_message, clock);
            }
            self.midi_messages.pop_front();
        }
        false
    }

    fn connect_midi_sink(&mut self, device: Rc<RefCell<dyn DeviceTrait>>) {
        self.sinks.push(device);
    }
}

enum EnvelopeState {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}
pub struct Envelope {
    child_device: Oscillator,
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
        child_device: Oscillator,
        attack: f32,
        decay: f32,
        sustain: f32,
        release: f32,
    ) -> Envelope {
        if !child_device.sources_audio() {
            panic!("Envelope created with non-audio-producing child device");
        }
        Envelope {
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
                / (state_duration * clock.sample_rate as f32);
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

    fn is_active(&self) -> bool {
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
        self.child_device.tick(clock);

        matches!(self.state, EnvelopeState::Idle)
    }

    fn get_audio_sample(&self) -> f32 {
        self.child_device.get_audio_sample() * self.amplitude
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
        self.child_device.handle_midi_message(message, clock);
    }
}

pub struct Voice {
    //    sound_source: Oscillator,
    envelope: Envelope,
}

impl Voice {
    pub fn new() -> Voice {
        let sound_source = Oscillator::new(Waveform::Sine);
        let envelope = Envelope::new(sound_source, 0.1, 0.1, 0.5, 0.3);
        Voice {
            //            sound_source,
            envelope,
        }
    }
    fn is_active(&self) -> bool {
        self.envelope.is_active()
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
        self.envelope.handle_midi_message(message, clock);
    }
    fn tick(&mut self, clock: &Clock) -> bool {
        self.envelope.tick(clock)
    }
    fn get_audio_sample(&self) -> f32 {
        self.envelope.get_audio_sample()
    }
}
pub struct SimpleSynth {
    voices: Vec<Voice>,
    note_to_voice: HashMap<u8, usize>,
}

impl SimpleSynth {
    pub fn new() -> SimpleSynth {
        let mut synth = SimpleSynth {
            voices: Vec::new(),
            note_to_voice: HashMap::<u8, usize>::new(),
        };
        for _ in 0..8 {
            synth.voices.push(Voice::new());
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
}

impl DeviceTrait for SimpleSynth {
    fn sources_audio(&self) -> bool {
        true
    }
    fn sinks_midi(&self) -> bool {
        true
    }
    fn handle_midi_message(&mut self, message: &MidiMessage, clock: &Clock) {
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
