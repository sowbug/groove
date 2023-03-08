// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::synthesizer::Synthesizer;
use groove_core::{
    generators::{Envelope, Oscillator},
    midi::{note_to_frequency, HandlesMidi, MidiChannel, MidiMessage},
    traits::{
        Generates, GeneratesEnvelope, IsInstrument, IsStereoSampleVoice, IsVoice, PlaysNotes,
        Resets, StoresVoices, Ticks,
    },
    BipolarNormal, Dca, Normal, ParameterType, Sample, StereoSample,
};
use groove_macros::{Control, Uid};
use std::{fmt::Debug, str::FromStr};
use strum_macros::{Display, EnumString, FromRepr};

#[derive(Debug)]
pub struct FmVoice {
    sample: StereoSample,
    carrier: Oscillator,
    modulator: Oscillator,
    modulator_depth: ParameterType,
    envelope: Envelope,
    dca: Dca,

    note_on_key: u8,
    note_on_velocity: u8,
    steal_is_underway: bool,
}
impl IsStereoSampleVoice for FmVoice {}
impl IsVoice<StereoSample> for FmVoice {}
impl PlaysNotes for FmVoice {
    fn is_playing(&self) -> bool {
        !self.envelope.is_idle()
    }

    fn note_on(&mut self, key: u8, velocity: u8) {
        if self.is_playing() {
            self.steal_is_underway = true;
            self.note_on_key = key;
            self.note_on_velocity = velocity;
            self.envelope.trigger_shutdown();
        } else {
            self.set_frequency_hz(note_to_frequency(key));
            self.envelope.trigger_attack();
        }
    }

    fn aftertouch(&mut self, _velocity: u8) {
        todo!()
    }

    fn note_off(&mut self, _velocity: u8) {
        self.envelope.trigger_release();
    }

    fn set_pan(&mut self, value: f32) {
        self.dca.set_pan(BipolarNormal::from(value));
    }
}
impl Generates<StereoSample> for FmVoice {
    fn value(&self) -> StereoSample {
        self.sample
    }

    #[allow(unused_variables)]
    fn batch_values(&mut self, values: &mut [StereoSample]) {
        todo!()
    }
}
impl Resets for FmVoice {
    fn reset(&mut self, sample_rate: usize) {
        self.envelope.reset(sample_rate);
        self.carrier.reset(sample_rate);
        self.modulator.reset(sample_rate);
    }
}
impl Ticks for FmVoice {
    fn tick(&mut self, tick_count: usize) {
        let was_playing = self.is_playing();
        self.carrier.set_frequency_modulation(BipolarNormal::from(
            self.modulator.value() * self.modulator_depth,
        ));
        self.envelope.tick(tick_count);
        self.carrier.tick(tick_count);
        self.modulator.tick(tick_count);
        let r = self.carrier.value() * self.envelope.value().value();
        if was_playing && !self.is_playing() {
            if self.steal_is_underway {
                self.steal_is_underway = false;
                self.note_on(self.note_on_key, self.note_on_velocity);
            }
        }
        self.sample = self.dca.transform_audio_to_stereo(Sample(r));
    }
}
impl FmVoice {
    pub(crate) fn new_with(sample_rate: usize) -> Self {
        Self {
            sample: Default::default(),
            carrier: Oscillator::new_with(sample_rate),
            modulator: Oscillator::new_with(sample_rate),
            modulator_depth: 100.0,
            envelope: Envelope::new_with(sample_rate, 0.1, 0.1, Normal::new(0.8), 0.25),
            dca: Default::default(),
            note_on_key: Default::default(),
            note_on_velocity: Default::default(),
            steal_is_underway: Default::default(),
        }
    }

    pub fn new_with_modulator_frequency(
        sample_rate: usize,
        modulator_frequency: ParameterType,
    ) -> Self {
        let mut modulator = Oscillator::new_with(sample_rate);
        modulator.set_frequency(modulator_frequency);
        let mut r = Self::new_with(sample_rate);
        r.modulator = modulator;
        r
    }

    #[allow(dead_code)]
    pub fn modulator_frequency(&self) -> ParameterType {
        self.modulator.frequency()
    }

    #[allow(dead_code)]
    pub fn set_modulator_frequency(&mut self, value: ParameterType) {
        self.modulator.set_frequency(value);
    }

    fn set_frequency_hz(&mut self, frequency_hz: ParameterType) {
        self.carrier.set_frequency(frequency_hz);
    }
}

#[derive(Control, Debug, Uid)]
pub struct FmSynthesizer {
    uid: usize,
    inner_synth: Synthesizer<FmVoice>,
}
impl IsInstrument for FmSynthesizer {}
impl Generates<StereoSample> for FmSynthesizer {
    fn value(&self) -> StereoSample {
        self.inner_synth.value()
    }

    fn batch_values(&mut self, values: &mut [StereoSample]) {
        self.inner_synth.batch_values(values);
    }
}
impl Resets for FmSynthesizer {
    fn reset(&mut self, sample_rate: usize) {
        self.inner_synth.reset(sample_rate)
    }
}
impl Ticks for FmSynthesizer {
    fn tick(&mut self, tick_count: usize) {
        self.inner_synth.tick(tick_count);
    }
}
impl HandlesMidi for FmSynthesizer {
    fn handle_midi_message(
        &mut self,
        message: &MidiMessage,
    ) -> Option<Vec<(MidiChannel, MidiMessage)>> {
        self.inner_synth.handle_midi_message(message)
    }
}
impl FmSynthesizer {
    pub fn new_with_voice_store(
        sample_rate: usize,
        voice_store: Box<dyn StoresVoices<Voice = FmVoice>>,
    ) -> Self {
        Self {
            uid: Default::default(),
            inner_synth: Synthesizer::<FmVoice>::new_with(sample_rate, voice_store),
        }
    }
}
