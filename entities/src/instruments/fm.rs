// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    control::F32ControlValue,
    generators::{AdsrParams, Envelope, Oscillator, Waveform},
    instruments::Synthesizer,
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

    /// modulator_depth 0.0 means no modulation; 1.0 means maximum
    modulator_depth: Normal,

    /// modulator_frequency is based on carrier frequency and modulator_ratio
    modulator_ratio: ParameterType,

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

    fn set_pan(&mut self, value: BipolarNormal) {
        self.dca.set_pan(value);
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
        let mut r = BipolarNormal::from(0.0);
        for _ in 0..tick_count {
            if self.is_playing() {
                let modulator_magnitude =
                    self.modulator.value() * self.modulator_envelope.value() * self.modulator_depth;
                self.carrier.set_linear_frequency_modulation(
                    modulator_magnitude.value() * self.modulator_beta,
                );
                r = self.carrier.value() * self.carrier_envelope.value();
                self.carrier_envelope.tick(tick_count);
                self.modulator_envelope.tick(tick_count);
                self.carrier.tick(tick_count);
                self.modulator.tick(tick_count);
                if !self.is_playing() && self.steal_is_underway {
                    self.steal_is_underway = false;
                    self.note_on(self.note_on_key, self.note_on_velocity);
                }
            }
        }
        self.sample = if self.is_playing() {
            self.dca.transform_audio_to_stereo(Sample::from(r))
        } else {
            StereoSample::from(StereoSample::SILENCE)
        };
    }
}
impl FmVoice {
    pub fn new_with(
        sample_rate: usize,
        modulator_depth: Normal,
        modulator_ratio: ParameterType,
        modulator_beta: ParameterType,
        carrier_envelope: AdsrParams,
        modulator_envelope: AdsrParams,
    ) -> Self {
        Self {
            sample: Default::default(),
            carrier: Oscillator::new_with(sample_rate),
            modulator: Oscillator::new_with_waveform(sample_rate, Waveform::Sine),
            modulator_depth,
            modulator_ratio,
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

    pub fn set_modulator_depth(&mut self, modulator_depth: Normal) {
        self.modulator_depth = modulator_depth;
    }

    pub fn set_modulator_ratio(&mut self, modulator_ratio: ParameterType) {
        self.modulator_ratio = modulator_ratio;
    }

    pub fn set_modulator_beta(&mut self, modulator_beta: ParameterType) {
        self.modulator_beta = modulator_beta;
    }

    pub fn modulator_depth(&self) -> Normal {
        self.modulator_depth
    }

    pub fn modulator_ratio(&self) -> f64 {
        self.modulator_ratio
    }

    pub fn modulator_beta(&self) -> f64 {
        self.modulator_beta
    }
}

#[derive(Control, Debug, Uid)]
pub struct FmSynthesizer {
    uid: usize,
    inner_synth: Synthesizer<FmVoice>,

    #[controllable]
    depth: Normal,

    #[controllable]
    ratio: ParameterType,

    #[controllable]
    beta: ParameterType,
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
        let (depth, ratio, beta) = if let Some(first_voice) = voice_store.voices().next() {
            (
                first_voice.modulator_depth(),
                first_voice.modulator_ratio(),
                first_voice.modulator_beta(),
            )
        } else {
            (Default::default(), Default::default(), Default::default())
        };
        Self {
            uid: Default::default(),
            inner_synth: Synthesizer::<FmVoice>::new_with(sample_rate, voice_store),
            depth,
            ratio,
            beta,
        }
    }

    pub fn set_depth(&mut self, depth: Normal) {
        self.depth = depth;
        self.inner_synth
            .voices_mut()
            .for_each(|v| v.set_modulator_depth(depth));
    }

    pub fn set_ratio(&mut self, ratio: ParameterType) {
        self.ratio = ratio;
        self.inner_synth
            .voices_mut()
            .for_each(|v| v.set_modulator_ratio(ratio));
    }

    pub fn set_beta(&mut self, beta: ParameterType) {
        self.beta = beta;
        self.inner_synth
            .voices_mut()
            .for_each(|v| v.set_modulator_beta(beta));
    }

    pub fn set_control_depth(&mut self, depth: F32ControlValue) {
        self.set_depth(Normal::from(depth.0));
    }

    // TODO: this is another case where having a better-defined incoming type
    // would help us do the right thing. We're mapping 0.0...1.0 to 0.0..very
    // high, but probably not higher than 32 or 64, and integer ratios make more
    // sense than fractional ones. What's that?
    pub fn set_control_ratio(&mut self, ratio: F32ControlValue) {
        self.set_ratio(ratio.0 as ParameterType);
    }

    // TODO same
    pub fn set_control_beta(&mut self, beta: F32ControlValue) {
        self.set_beta(beta.0 as ParameterType);
    }

    pub fn depth(&self) -> Normal {
        self.depth
    }

    pub fn ratio(&self) -> f64 {
        self.ratio
    }

    pub fn beta(&self) -> f64 {
        self.beta
    }
}
