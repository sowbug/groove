use super::midi::{MidiMessage, MidiMessageType};
use super::traits::DeviceTrait;
use crate::primitives::clock::Clock;
use crate::primitives::envelopes::{MiniEnvelope, MiniEnvelopePreset};
use crate::primitives::filter::{MiniFilter, MiniFilterType};
use crate::primitives::oscillators::{
    LfoPreset, LfoRouting, MiniOscillator, OscillatorPreset, Waveform,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct Voice {
    envelope: MiniEnvelope,
}

impl Voice {
    pub fn new(waveform: Waveform) -> Self {
        let sound_source = Rc::new(RefCell::new(MiniOscillator::new(waveform)));
        let envelope = MiniEnvelope::new(
            44100, /*TODO*/
            MiniEnvelopePreset {
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

pub struct SimpleSynthPreset {
    oscillator_1_preset: OscillatorPreset,
    oscillator_2_preset: OscillatorPreset,
    // TODO: osc 2 track/sync
    amp_envelope_preset: MiniEnvelopePreset,

    lfo_preset: LfoPreset,

    // TODO: glide, time, unison, voices

    // There is meant to be only one filter, but the Welsh book
    // provides alternate settings depending on the kind of filter
    // your synthesizer has.
    filter_24db_type: MiniFilterType,
    filter_12db_type: MiniFilterType,
    filter_24db_weight: f32,
    filter_12db_weight: f32,
    filter_envelope_preset: MiniEnvelopePreset,
    filter_envelope_weight: f32,
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
            SimpleSynthPreset {
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

    pub fn new(sample_rate: u32, preset: SimpleSynthPreset) -> Self {
        Self {
            osc_1: MiniOscillator::new_from_preset(&preset.oscillator_1_preset),
            osc_2: MiniOscillator::new_from_preset(&preset.oscillator_2_preset),
            osc_1_mix: preset.oscillator_1_preset.mix,
            osc_2_mix: preset.oscillator_2_preset.mix,
            amp_envelope: MiniEnvelope::new(sample_rate, preset.amp_envelope_preset),

            lfo: MiniOscillator::new_lfo(&preset.lfo_preset),
            lfo_routing: preset.lfo_preset.routing,
            lfo_depth: preset.lfo_preset.depth,

            filter: MiniFilter::new(44100, preset.filter_24db_type),
            filter_weight: preset.filter_24db_weight,
            filter_envelope: MiniEnvelope::new(sample_rate, preset.filter_envelope_preset),
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
