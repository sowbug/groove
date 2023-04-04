// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::effects::{BiQuadFilterLowPass24db, BiQuadFilterLowPass24dbNano};
use core::fmt::Debug;
use groove_core::{
    generators::{Envelope, EnvelopeParams, Oscillator, OscillatorNano},
    instruments::Synthesizer,
    midi::{note_to_frequency, HandlesMidi, MidiChannel, MidiMessage},
    traits::{
        Generates, GeneratesEnvelope, IsInstrument, IsStereoSampleVoice, IsVoice, PlaysNotes,
        Resets, StoresVoices, Ticks, TransformsAudio,
    },
    voices::StealingVoiceStore,
    BipolarNormal, Dca, DcaParams, FrequencyHz, Normal, Sample, StereoSample,
};
use groove_proc_macros::{Nano, Uid};
use std::str::FromStr;
use strum::EnumCount;
use strum_macros::{Display, EnumCount as EnumCountMacro, EnumString, FromRepr, IntoStaticStr};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, EnumCountMacro, FromRepr, PartialEq)]
#[cfg_attr(
    feature = "serialization",
    derive(Serialize, Deserialize),
    serde(rename = "lfo-routing", rename_all = "kebab-case")
)]
pub enum LfoRouting {
    #[default]
    None,
    Amplitude,
    Pitch,
    PulseWidth,
    FilterCutoff,
}

#[derive(Debug)]
pub struct WelshVoice {
    oscillators: Vec<Oscillator>,
    oscillator_2_sync: bool,
    oscillator_mix: Normal, // 1.0 = entirely osc 0, 0.0 = entirely osc 1.
    amp_envelope: Envelope,
    dca: Dca,

    lfo: Oscillator,
    lfo_routing: LfoRouting,
    lfo_depth: Normal,

    filter: BiQuadFilterLowPass24db,
    filter_cutoff_start: Normal,
    filter_cutoff_end: Normal,
    filter_envelope: Envelope,

    note_on_key: u8,
    note_on_velocity: u8,
    steal_is_underway: bool,

    sample: StereoSample,
    ticks: usize,
}
impl IsStereoSampleVoice for WelshVoice {}
impl IsVoice<StereoSample> for WelshVoice {}
impl PlaysNotes for WelshVoice {
    fn is_playing(&self) -> bool {
        !self.amp_envelope.is_idle()
    }
    fn note_on(&mut self, key: u8, velocity: u8) {
        if self.is_playing() {
            self.steal_is_underway = true;
            self.note_on_key = key;
            self.note_on_velocity = velocity;
            self.amp_envelope.trigger_shutdown();
        } else {
            self.amp_envelope.trigger_attack();
            self.filter_envelope.trigger_attack();
            self.set_frequency_hz(note_to_frequency(key));
        }
    }
    fn aftertouch(&mut self, _velocity: u8) {
        // TODO: do something
    }
    fn note_off(&mut self, _velocity: u8) {
        self.amp_envelope.trigger_release();
        self.filter_envelope.trigger_release();
    }
    fn set_pan(&mut self, value: BipolarNormal) {
        self.dca.set_pan(value)
    }
}
impl Generates<StereoSample> for WelshVoice {
    fn value(&self) -> StereoSample {
        self.sample
    }
    fn batch_values(&mut self, _samples: &mut [StereoSample]) {
        todo!()
    }
}
impl Resets for WelshVoice {
    fn reset(&mut self, sample_rate: usize) {
        self.ticks = 0;
        self.lfo.reset(sample_rate);
        self.amp_envelope.reset(sample_rate);
        self.filter_envelope.reset(sample_rate);
        self.oscillators
            .iter_mut()
            .for_each(|o| o.reset(sample_rate));
    }
}
impl Ticks for WelshVoice {
    fn tick(&mut self, tick_count: usize) {
        for _ in 0..tick_count {
            self.ticks += 1;
            // It's important for the envelope tick() methods to be called after
            // their handle_note_* methods are called, but before we check whether
            // amp_envelope.is_idle(), because the tick() methods are what determine
            // the current idle state.
            //
            // TODO: this seems like an implementation detail that maybe should be
            // hidden from the caller.
            let (amp_env_amplitude, filter_env_amplitude) = self.tick_envelopes();

            // TODO: various parts of this loop can be precalculated.

            self.sample = if self.is_playing() {
                // TODO: ideally, these entities would get a tick() on every
                // voice tick(), but they are surprisingly expensive. So we will
                // skip calling them unless we're going to look at their output.
                // This means that they won't get a time slice as often as the
                // voice will. If this becomes a problem, we can add something
                // like an empty_tick() method to the Ticks trait that lets
                // entities stay in sync, but skipping any real work that would
                // cost time.
                if !matches!(self.lfo_routing, LfoRouting::None) {
                    self.lfo.tick(1);
                }
                self.oscillators.iter_mut().for_each(|o| o.tick(1));

                // LFO
                let lfo = self.lfo.value();
                if matches!(self.lfo_routing, LfoRouting::Pitch) {
                    let lfo_for_pitch = lfo * self.lfo_depth;
                    for o in self.oscillators.iter_mut() {
                        o.set_frequency_modulation(lfo_for_pitch);
                    }
                }

                // Oscillators
                let len = self.oscillators.len();
                let osc_sum = match len {
                    0 => BipolarNormal::from(0.0),
                    1 => self.oscillators[0].value() * self.oscillator_mix,
                    2 => {
                        if self.oscillator_2_sync && self.oscillators[0].should_sync() {
                            self.oscillators[1].sync();
                        }
                        self.oscillators[0].value() * self.oscillator_mix
                            + self.oscillators[1].value()
                                * (Normal::maximum() - self.oscillator_mix)
                    }
                    _ => todo!(),
                };

                // Filters
                //
                // https://aempass.blogspot.com/2014/09/analog-and-welshs-synthesizer-cookbook.html
                if self.filter_cutoff_end != Normal::zero() {
                    let new_cutoff_percentage = self.filter_cutoff_start
                        + (1.0 - self.filter_cutoff_start)
                            * self.filter_cutoff_end
                            * filter_env_amplitude;
                    self.filter.set_cutoff(new_cutoff_percentage.into());
                } else if matches!(self.lfo_routing, LfoRouting::FilterCutoff) {
                    let lfo_for_cutoff = lfo * self.lfo_depth;
                    self.filter.set_cutoff(
                        (self.filter_cutoff_start * (lfo_for_cutoff.value() + 1.0)).into(),
                    );
                }
                let filtered_mix = self.filter.transform_channel(0, Sample::from(osc_sum)).0;

                // LFO amplitude modulation
                let lfo_for_amplitude =
                    Normal::from(if matches!(self.lfo_routing, LfoRouting::Amplitude) {
                        lfo * self.lfo_depth
                    } else {
                        BipolarNormal::zero()
                    });

                // Final
                self.dca.transform_audio_to_stereo(Sample(
                    filtered_mix * amp_env_amplitude.value() * lfo_for_amplitude.value(),
                ))
            } else {
                StereoSample::SILENCE
            };
        }
    }
}
impl WelshVoice {
    fn tick_envelopes(&mut self) -> (Normal, Normal) {
        if self.is_playing() {
            self.amp_envelope.tick(1);
            self.filter_envelope.tick(1);
            if self.is_playing() {
                return (self.amp_envelope.value(), self.filter_envelope.value());
            }

            if self.steal_is_underway {
                self.steal_is_underway = false;
                self.note_on(self.note_on_key, self.note_on_velocity);
            }
        }
        (Normal::zero(), Normal::zero())
    }

    fn set_frequency_hz(&mut self, frequency_hz: FrequencyHz) {
        // It's safe to set the frequency on a fixed-frequency oscillator; the
        // fixed frequency is stored separately and takes precedence.
        self.oscillators.iter_mut().for_each(|o| {
            o.set_frequency(frequency_hz);
        });
    }

    pub fn new_with(
        oscillators: Vec<Oscillator>,
        oscillator_2_sync: bool,
        oscillator_mix: Normal,
        amp_envelope: Envelope,
        filter: BiQuadFilterLowPass24db,
        filter_cutoff_start: Normal,
        filter_cutoff_end: Normal,
        filter_envelope: Envelope,
        lfo: Oscillator,
        lfo_routing: LfoRouting,
        lfo_depth: Normal,
    ) -> WelshVoice {
        Self {
            oscillators,
            oscillator_2_sync,
            oscillator_mix,
            amp_envelope,
            dca: Dca::new_with_params(DcaParams::default()),
            lfo,
            lfo_routing,
            lfo_depth,
            filter,
            filter_cutoff_start,
            filter_cutoff_end,
            filter_envelope,
            sample: Default::default(),
            ticks: Default::default(),
            note_on_key: Default::default(),
            note_on_velocity: Default::default(),
            steal_is_underway: Default::default(),
        }
    }

    pub fn new_with_params(sample_rate: usize, params: WelshSynthNano) -> Self {
        Self {
            oscillators: vec![
                Oscillator::new_with(sample_rate, params.oscillator_1),
                Oscillator::new_with(sample_rate, params.oscillator_2),
            ],
            oscillator_2_sync: params.oscillator_sync,
            oscillator_mix: params.oscillator_mix,
            amp_envelope: Envelope::new_with(sample_rate, params.envelope()),
            dca: Dca::new_with_params(params.dca),
            lfo: Oscillator::new_with(sample_rate, params.lfo),
            lfo_routing: params.lfo_routing,
            lfo_depth: params.lfo_depth,
            filter: BiQuadFilterLowPass24db::new_with(sample_rate, params.filter),
            filter_cutoff_start: params.filter_cutoff_start,
            filter_cutoff_end: params.filter_cutoff_end,
            filter_envelope: Envelope::new_with(sample_rate, params.filter_envelope),
            note_on_key: Default::default(),
            note_on_velocity: Default::default(),
            steal_is_underway: Default::default(),
            sample: Default::default(),
            ticks: Default::default(),
        }
    }
}

#[derive(Debug, Nano, Uid)]
pub struct WelshSynth {
    uid: usize,
    inner_synth: Synthesizer<WelshVoice>,

    #[nano(control = false)]
    oscillator_1: OscillatorNano,
    #[nano(control = false)]
    oscillator_2: OscillatorNano,
    #[nano]
    oscillator_sync: bool,
    #[nano]
    oscillator_mix: Normal,

    #[nano(control = false)]
    envelope: EnvelopeParams,

    #[nano(control = false)]
    dca: DcaParams,

    #[nano(control = false)]
    lfo: OscillatorNano,
    #[nano(control = false)]
    lfo_routing: LfoRouting,
    #[nano]
    lfo_depth: Normal,

    #[nano(control = false)]
    filter: BiQuadFilterLowPass24dbNano,

    #[nano]
    filter_cutoff_start: Normal,

    #[nano]
    filter_cutoff_end: Normal,

    #[nano(control = false)]
    filter_envelope: EnvelopeParams,

    #[nano]
    pan: BipolarNormal,
}
impl IsInstrument for WelshSynth {}
impl Generates<StereoSample> for WelshSynth {
    fn value(&self) -> StereoSample {
        self.inner_synth.value()
    }

    fn batch_values(&mut self, values: &mut [StereoSample]) {
        self.inner_synth.batch_values(values);
    }
}
impl Resets for WelshSynth {
    fn reset(&mut self, sample_rate: usize) {
        self.inner_synth.reset(sample_rate);
    }
}
impl Ticks for WelshSynth {
    fn tick(&mut self, tick_count: usize) {
        self.inner_synth.tick(tick_count);
    }
}
impl HandlesMidi for WelshSynth {
    fn handle_midi_message(
        &mut self,
        message: &MidiMessage,
    ) -> Option<Vec<(MidiChannel, MidiMessage)>> {
        match message {
            #[allow(unused_variables)]
            MidiMessage::ProgramChange { program } => {
                todo!()
                // if let Some(program) = GeneralMidiProgram::from_u8(program.as_int()) {
                //     if let Ok(_preset) = WelshSynth::general_midi_preset(&program) {
                //         //  self.preset = preset;
                //     } else {
                //         println!("unrecognized patch from MIDI program change: {}", &program);
                //     }
                // }
                // None
            }
            _ => self.inner_synth.handle_midi_message(message),
        }
    }
}
impl WelshSynth {
    #[deprecated]
    pub fn new_with(
        sample_rate: usize,
        voice_store: Box<dyn StoresVoices<Voice = WelshVoice>>,
    ) -> Self {
        Self {
            uid: Default::default(),
            inner_synth: Synthesizer::<WelshVoice>::new_with(sample_rate, voice_store),

            oscillator_1: todo!(),
            oscillator_2: todo!(),
            pan: Default::default(),
            envelope: Default::default(),
            filter_envelope: Default::default(),
            oscillator_sync: todo!(),
            oscillator_mix: todo!(),
            dca: todo!(),
            lfo: todo!(),
            lfo_routing: todo!(),
            lfo_depth: todo!(),
            filter: todo!(),
            filter_cutoff_start: todo!(),
            filter_cutoff_end: todo!(),
        }
    }

    pub fn new_with_params(sample_rate: usize, params: WelshSynthNano) -> Self {
        const VOICE_CAPACITY: usize = 8;
        let voice_store =
            StealingVoiceStore::<WelshVoice>::new_with_voice(sample_rate, VOICE_CAPACITY, || {
                WelshVoice::new_with_params(sample_rate, params)
            });
        Self {
            uid: Default::default(),
            inner_synth: Synthesizer::<WelshVoice>::new_with(sample_rate, Box::new(voice_store)),
            pan: params.dca().pan(),
            envelope: params.envelope(),
            filter_envelope: params.filter_envelope(),
            oscillator_1: params.oscillator_1(),
            oscillator_2: params.oscillator_2(),
            oscillator_sync: params.oscillator_sync(),
            oscillator_mix: params.oscillator_mix(),
            dca: params.dca(),
            lfo: params.lfo(),
            lfo_routing: params.lfo_routing(),
            lfo_depth: params.lfo_depth(),
            filter: params.filter(),
            filter_cutoff_start: params.filter_cutoff_start(),
            filter_cutoff_end: params.filter_cutoff_end(),
        }
    }

    pub fn preset_name(&self) -> &str {
        "none"
        //        self.preset.name.as_str()
    }

    pub fn pan(&self) -> f32 {
        self.inner_synth.pan()
    }

    pub fn set_pan(&mut self, pan: BipolarNormal) {
        self.pan = pan;
        self.inner_synth.voices_mut().for_each(|v| v.set_pan(pan));
    }

    // TODO: this pattern sucks. I knew it was going to be icky. Think about how
    // to make it less copy/paste.
    pub fn update(&mut self, message: WelshSynthMessage) {
        match message {
            WelshSynthMessage::WelshSynth(_e) => {
                // TODO: this will be a lot of work.
            }
            WelshSynthMessage::Dca(_) => todo!(),
            WelshSynthMessage::Envelope(envelope) => self.set_envelope(envelope),
            WelshSynthMessage::Filter(_) => todo!(),
            WelshSynthMessage::FilterCutoffEnd(_) => todo!(),
            WelshSynthMessage::FilterCutoffStart(_) => todo!(),
            WelshSynthMessage::FilterEnvelope(filter_envelope) => {
                self.set_filter_envelope(filter_envelope)
            }
            WelshSynthMessage::Lfo(_) => todo!(),
            WelshSynthMessage::LfoDepth(_) => todo!(),
            WelshSynthMessage::LfoRouting(_) => todo!(),
            WelshSynthMessage::Oscillator1(_) => todo!(),
            WelshSynthMessage::Oscillator2(_) => todo!(),
            WelshSynthMessage::OscillatorMix(_) => todo!(),
            WelshSynthMessage::OscillatorSync(_) => todo!(),
            WelshSynthMessage::Pan(pan) => self.set_pan(pan),
        }
    }

    pub fn envelope(&self) -> EnvelopeParams {
        self.envelope
    }

    pub fn filter_envelope(&self) -> EnvelopeParams {
        self.filter_envelope
    }

    pub fn set_envelope(&mut self, envelope: EnvelopeParams) {
        self.envelope = envelope;
    }

    pub fn set_filter_envelope(&mut self, filter_envelope: EnvelopeParams) {
        self.filter_envelope = filter_envelope;
    }

    pub fn set_oscillator_1(&mut self, oscillator_1: OscillatorNano) {
        self.oscillator_1 = oscillator_1;
    }

    pub fn set_oscillator_2(&mut self, oscillator_2: OscillatorNano) {
        self.oscillator_2 = oscillator_2;
    }

    pub fn set_oscillator_sync(&mut self, oscillator_sync: bool) {
        self.oscillator_sync = oscillator_sync;
    }

    pub fn set_oscillator_mix(&mut self, oscillator_mix: Normal) {
        self.oscillator_mix = oscillator_mix;
    }

    pub fn set_lfo(&mut self, lfo: OscillatorNano) {
        self.lfo = lfo;
    }

    pub fn set_lfo_routing(&mut self, lfo_routing: LfoRouting) {
        self.lfo_routing = lfo_routing;
    }

    pub fn set_lfo_depth(&mut self, lfo_depth: Normal) {
        self.lfo_depth = lfo_depth;
    }

    pub fn set_filter(&mut self, filter: BiQuadFilterLowPass24dbNano) {
        self.filter = filter;
    }

    pub fn set_filter_cutoff_start(&mut self, filter_cutoff_start: Normal) {
        self.filter_cutoff_start = filter_cutoff_start;
    }

    pub fn set_filter_cutoff_end(&mut self, filter_cutoff_end: Normal) {
        self.filter_cutoff_end = filter_cutoff_end;
    }

    pub fn set_dca(&mut self, dca: DcaParams) {
        self.dca = dca;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{DEFAULT_BPM, DEFAULT_MIDI_TICKS_PER_SECOND, DEFAULT_SAMPLE_RATE};
    use convert_case::{Case, Casing};
    use groove_core::{time::Clock, util::tests::TestOnlyPaths, SampleType};

    // TODO dedup
    pub fn canonicalize_output_filename_and_path(filename: &str) -> String {
        let mut path = TestOnlyPaths::test_data_path();
        let snake_filename = format!("{}.wav", filename.to_case(Case::Snake)).to_string();
        path.push(snake_filename);
        if let Some(path) = path.to_str() {
            path.to_string()
        } else {
            panic!("trouble creating output path")
        }
    }

    // TODO: refactor out to common test utilities
    #[allow(dead_code)]
    fn write_voice(voice: &mut WelshVoice, duration: f64, basename: &str) {
        let mut clock = Clock::new_with(
            DEFAULT_SAMPLE_RATE,
            DEFAULT_BPM,
            DEFAULT_MIDI_TICKS_PER_SECOND,
        );

        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: clock.sample_rate() as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        const AMPLITUDE: SampleType = i16::MAX as SampleType;
        let mut writer =
            hound::WavWriter::create(canonicalize_output_filename_and_path(basename), spec)
                .unwrap();

        let mut last_recognized_time_point = -1.;
        let time_note_off = duration / 2.0;
        while clock.seconds() < duration {
            if clock.seconds() >= 0.0 && last_recognized_time_point < 0.0 {
                last_recognized_time_point = clock.seconds();
                voice.note_on(60, 127);
                voice.tick_envelopes();
            } else if clock.seconds() >= time_note_off && last_recognized_time_point < time_note_off
            {
                last_recognized_time_point = clock.seconds();
                voice.note_off(127);
                voice.tick_envelopes();
            }

            voice.tick(1);
            let sample = voice.value();
            let _ = writer.write_sample((sample.0 .0 * AMPLITUDE) as i16);
            let _ = writer.write_sample((sample.1 .0 * AMPLITUDE) as i16);
            clock.tick(1);
        }
    }

    // use std::panic;
    // use strum::IntoEnumIterator;
    // #[test]
    // #[should_panic]
    // fn test_presets() {
    //     let clock = Clock::new(&ClockSettings::new_defaults());
    //     for preset in PresetName::iter() {
    //         let result = panic::catch_unwind(|| {
    //             Voice::new(
    //                 MIDI_CHANNEL_RECEIVE_ALL,
    //                 clock.sample_rate(),
    //                 &super::SynthPreset::by_name(&preset),
    //             )
    //         });
    //         if result.is_ok() {
    //             let mut voice = result.unwrap();
    //             let preset_name = preset.to_string();
    //             write_voice(&mut voice, 2.0, &format!("voice_{}", preset_name));
    //         }
    //     }
    // }

    // This code was used to convert Rust representation of 26 Welsh patches to serde YAML.
    // #[derive(Serialize)]
    // struct Foo {
    //     x: Vec<SynthPreset>,
    // }

    // #[test]
    // #[should_panic]
    // fn test_presets() {
    //     for preset in PresetName::iter() {
    //         if let Ok(result) = panic::catch_unwind(|| super::SynthPreset::by_name(&preset)) {
    //             if let Ok(s) = serde_yaml::to_string(&result) {
    //                 if let Ok(_) = std::fs::write(format!("{}.yaml", result.name), s) {
    //                     // great
    //                 }
    //             }
    //         }
    //     }
    // }
}
