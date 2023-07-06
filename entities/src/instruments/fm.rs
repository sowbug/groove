// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    generators::{Envelope, EnvelopeParams, Oscillator, OscillatorParams, Waveform},
    instruments::Synthesizer,
    midi::{note_to_frequency, HandlesMidi, MidiChannel, MidiMessage, MidiMessagesFn},
    time::SampleRate,
    traits::{
        Configurable, Generates, GeneratesEnvelope, IsStereoSampleVoice, IsVoice, PlaysNotes, Ticks,
    },
    voices::StealingVoiceStore,
    BipolarNormal, Dca, DcaParams, FrequencyHz, Normal, ParameterType, Ratio, Sample, StereoSample,
};
use groove_proc_macros::{Control, IsInstrument, Params, Uid};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Default)]
pub struct FmVoice {
    sample: StereoSample,
    carrier: Oscillator,
    modulator: Oscillator,

    /// modulator_depth 0.0 means no modulation; 1.0 means maximum
    modulator_depth: Normal,

    /// modulator_frequency is based on carrier frequency and modulator_ratio
    modulator_ratio: Ratio,

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

    sample_rate: SampleRate,
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
}
impl Generates<StereoSample> for FmVoice {
    fn value(&self) -> StereoSample {
        self.sample
    }

    #[allow(unused_variables)]
    fn generate_batch_values(&mut self, values: &mut [StereoSample]) {
        todo!()
    }
}
impl Configurable for FmVoice {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.carrier_envelope.update_sample_rate(sample_rate);
        self.modulator_envelope.update_sample_rate(sample_rate);
        self.carrier.update_sample_rate(sample_rate);
        self.modulator.update_sample_rate(sample_rate);
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
            StereoSample::SILENCE
        };
    }
}
impl FmVoice {
    pub fn new_with(params: &FmSynthParams) -> Self {
        Self {
            sample_rate: Default::default(),
            sample: Default::default(),
            carrier: Oscillator::new_with(&OscillatorParams::default_with_waveform(Waveform::Sine)),
            modulator: Oscillator::new_with(&OscillatorParams::default_with_waveform(
                Waveform::Sine,
            )),
            modulator_depth: params.depth,
            modulator_ratio: params.ratio,
            modulator_beta: params.beta,
            carrier_envelope: Envelope::new_with(&params.carrier_envelope),
            modulator_envelope: Envelope::new_with(&params.modulator_envelope),
            dca: Dca::new_with(&params.dca),
            note_on_key: Default::default(),
            note_on_velocity: Default::default(),
            steal_is_underway: Default::default(),
        }
    }

    #[allow(dead_code)]
    pub fn modulator_frequency(&self) -> FrequencyHz {
        self.modulator.frequency()
    }

    #[allow(dead_code)]
    pub fn set_modulator_frequency(&mut self, value: FrequencyHz) {
        self.modulator.set_frequency(value);
    }

    fn set_frequency_hz(&mut self, frequency_hz: FrequencyHz) {
        self.carrier.set_frequency(frequency_hz);
        self.modulator
            .set_frequency(frequency_hz * self.modulator_ratio);
    }

    pub fn set_modulator_depth(&mut self, modulator_depth: Normal) {
        self.modulator_depth = modulator_depth;
    }

    pub fn set_modulator_ratio(&mut self, modulator_ratio: Ratio) {
        self.modulator_ratio = modulator_ratio;
    }

    pub fn set_modulator_beta(&mut self, modulator_beta: ParameterType) {
        self.modulator_beta = modulator_beta;
    }

    pub fn modulator_depth(&self) -> Normal {
        self.modulator_depth
    }

    pub fn modulator_ratio(&self) -> Ratio {
        self.modulator_ratio
    }

    pub fn modulator_beta(&self) -> f64 {
        self.modulator_beta
    }

    // TODO: we'll have to be smarter about subbing in a new envelope, possibly
    // while the voice is playing.
    pub fn set_carrier_envelope(&mut self, envelope: Envelope) {
        self.carrier_envelope = envelope;
    }

    pub fn set_modulator_envelope(&mut self, envelope: Envelope) {
        self.modulator_envelope = envelope;
    }

    fn set_gain(&mut self, gain: Normal) {
        self.dca.set_gain(gain);
    }

    fn set_pan(&mut self, pan: BipolarNormal) {
        self.dca.set_pan(pan);
    }
}

#[derive(Debug, Control, IsInstrument, Params, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct FmSynth {
    #[control]
    #[params]
    depth: Normal,

    #[control]
    #[params]
    ratio: Ratio,

    #[control]
    #[params]
    beta: ParameterType,

    #[control]
    #[params]
    carrier_envelope: Envelope,

    #[control]
    #[params]
    modulator_envelope: Envelope,

    #[control]
    #[params]
    dca: Dca,

    uid: groove_core::Uid,
    #[cfg_attr(feature = "serialization", serde(skip))]
    inner_synth: Synthesizer<FmVoice>,
}
impl Generates<StereoSample> for FmSynth {
    fn value(&self) -> StereoSample {
        self.inner_synth.value()
    }

    fn generate_batch_values(&mut self, values: &mut [StereoSample]) {
        self.inner_synth.generate_batch_values(values);
    }
}
impl Configurable for FmSynth {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.inner_synth.update_sample_rate(sample_rate)
    }
}
impl Ticks for FmSynth {
    fn tick(&mut self, tick_count: usize) {
        self.inner_synth.tick(tick_count);
    }
}
impl HandlesMidi for FmSynth {
    fn handle_midi_message(
        &mut self,
        channel: MidiChannel,
        message: MidiMessage,
        midi_messages_fn: &mut MidiMessagesFn,
    ) {
        self.inner_synth
            .handle_midi_message(channel, message, midi_messages_fn)
    }
}
impl FmSynth {
    pub fn new_with(params: &FmSynthParams) -> Self {
        const VOICE_CAPACITY: usize = 8;
        let voice_store = StealingVoiceStore::<FmVoice>::new_with_voice(VOICE_CAPACITY, || {
            FmVoice::new_with(&params)
        });

        Self {
            uid: Default::default(),
            inner_synth: Synthesizer::<FmVoice>::new_with(Box::new(voice_store)),
            depth: params.depth(),
            ratio: params.ratio(),
            beta: params.beta(),
            carrier_envelope: Envelope::new_with(&params.carrier_envelope),
            modulator_envelope: Envelope::new_with(&params.modulator_envelope),
            dca: Dca::new_with(&params.dca),
        }
    }

    pub fn set_depth(&mut self, depth: Normal) {
        self.depth = depth;
        self.inner_synth
            .voices_mut()
            .for_each(|v| v.set_modulator_depth(depth));
    }

    pub fn set_ratio(&mut self, ratio: Ratio) {
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

    pub fn depth(&self) -> Normal {
        self.depth
    }

    pub fn ratio(&self) -> Ratio {
        self.ratio
    }

    pub fn beta(&self) -> f64 {
        self.beta
    }

    #[cfg(feature = "iced-framework")]
    pub fn update(&mut self, message: FmSynthMessage) {
        match message {
            FmSynthMessage::FmSynth(_s) => {
                todo!()
            }
            _ => self.derived_update(message),
        }
    }

    // TODO: replace with update_from_params() or whatever that turns out to be
    pub fn set_carrier_envelope(&mut self, carrier_envelope: Envelope) {
        self.carrier_envelope = carrier_envelope;
        self.inner_synth.voices_mut().for_each(|v| {
            v.set_carrier_envelope(Envelope::new_with(&self.carrier_envelope.to_params()))
        });
    }

    pub fn set_modulator_envelope(&mut self, modulator_envelope: Envelope) {
        self.modulator_envelope = modulator_envelope;
        self.inner_synth.voices_mut().for_each(|v| {
            v.set_modulator_envelope(Envelope::new_with(&self.modulator_envelope.to_params()))
        });
    }

    pub fn set_gain(&mut self, gain: Normal) {
        self.dca.set_gain(gain);
        self.inner_synth.voices_mut().for_each(|v| v.set_gain(gain));
    }

    pub fn set_pan(&mut self, pan: BipolarNormal) {
        self.dca.set_pan(pan);
        self.inner_synth.voices_mut().for_each(|v| v.set_pan(pan));
    }

    pub fn dca(&self) -> &Dca {
        &self.dca
    }
}

#[cfg(feature = "egui-framework")]
mod gui {
    use super::FmSynth;
    use eframe::egui::Ui;
    use groove_core::traits::{gui::Shows, HasUid};

    impl Shows for FmSynth {
        fn show(&mut self, ui: &mut Ui) {
            ui.label(self.name());
        }
    }
}
