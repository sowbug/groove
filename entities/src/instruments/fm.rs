// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::synthesizer::Synthesizer;
use groove_core::{
    generators::{AdsrParams, Envelope, Oscillator, Waveform},
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

    /// modulator_frequency is based on carrier frequency and modulator_ratio
    modulator_ratio: ParameterType,

    /// modulator_depth 0.0 means no modulation; 1.0 means maximum
    modulator_depth: Normal,

    /// Ranges from 0.0 to very high.
    ///
    /// - 0.0: no effect
    /// - 0.1: change is visible on scope but not audible
    /// - 1.0: audible change
    /// - 10.0: dramatic change,
    /// - 100.0: extreme.
    modulator_beta: ParameterType,

    carrier_envelope: Envelope,
    modulator_envelope: Envelope,
    dca: Dca,

    note_on_key: u8,
    note_on_velocity: u8,
    steal_is_underway: bool,
}
impl IsStereoSampleVoice for FmVoice {}
impl IsVoice<StereoSample> for FmVoice {}
impl PlaysNotes for FmVoice {
    fn is_playing(&self) -> bool {
        !self.carrier_envelope.is_idle()
    }

    fn note_on(&mut self, key: u8, velocity: u8) {
        if self.is_playing() {
            self.steal_is_underway = true;
            self.note_on_key = key;
            self.note_on_velocity = velocity;
            self.carrier_envelope.trigger_shutdown();
            self.modulator_envelope.trigger_shutdown();
        } else {
            self.set_frequency_hz(note_to_frequency(key));
            self.carrier_envelope.trigger_attack();
            self.modulator_envelope.trigger_attack();
        }
    }

    fn aftertouch(&mut self, _velocity: u8) {
        todo!()
    }

    fn note_off(&mut self, _velocity: u8) {
        self.carrier_envelope.trigger_release();
        self.modulator_envelope.trigger_release();
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
        self.carrier_envelope.reset(sample_rate);
        self.modulator_envelope.reset(sample_rate);
        self.carrier.reset(sample_rate);
        self.modulator.reset(sample_rate);
    }
}
impl Ticks for FmVoice {
    fn tick(&mut self, tick_count: usize) {
        if self.is_playing() {
            let modulator_magnitude =
                self.modulator.value() * self.modulator_envelope.value() * self.modulator_depth;
            self.carrier
                .set_linear_frequency_modulation(modulator_magnitude.value() * self.modulator_beta);
            let r = self.carrier.value() * self.carrier_envelope.value();
            self.carrier_envelope.tick(tick_count);
            self.modulator_envelope.tick(tick_count);
            self.carrier.tick(tick_count);
            self.modulator.tick(tick_count);
            if self.is_playing() {
                self.sample = self.dca.transform_audio_to_stereo(Sample::from(r));
                return;
            } else if self.steal_is_underway {
                self.steal_is_underway = false;
                self.note_on(self.note_on_key, self.note_on_velocity);
            }
        }
        self.sample = StereoSample::SILENCE;
    }
}
impl FmVoice {
    pub fn new_with(
        sample_rate: usize,
        modulator_ratio: ParameterType,
        modulator_depth: Normal,
        modulator_beta: ParameterType,
        carrier_envelope: AdsrParams,
        modulator_envelope: AdsrParams,
    ) -> Self {
        Self {
            sample: Default::default(),
            carrier: Oscillator::new_with(sample_rate),
            modulator: Oscillator::new_with_waveform(sample_rate, Waveform::Sine),
            modulator_ratio,
            modulator_depth,
            modulator_beta,
            carrier_envelope: Envelope::new_with(sample_rate, carrier_envelope),
            modulator_envelope: Envelope::new_with(sample_rate, modulator_envelope),
            dca: Default::default(),
            note_on_key: Default::default(),
            note_on_velocity: Default::default(),
            steal_is_underway: Default::default(),
        }
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
        self.modulator
            .set_frequency(frequency_hz * self.modulator_ratio);
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
