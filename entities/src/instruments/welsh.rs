use super::synthesizer::Synthesizer;
use crate::effects::BiQuadFilter;
use groove_core::{
    generators::{Envelope, Oscillator},
    midi::{note_to_frequency, HandlesMidi, MidiChannel, MidiMessage},
    traits::{
        Generates, GeneratesEnvelope, IsInstrument, IsStereoSampleVoice, IsVoice, PlaysNotes,
        Resets, StoresVoices, Ticks, TransformsAudio,
    },
    BipolarNormal, Dca, Normal, ParameterType, Sample, StereoSample,
};
use groove_macros::{Control, Uid};
use std::str::FromStr;
use strum_macros::{Display, EnumString, FromRepr};

#[derive(Debug)]
pub enum LfoRouting {
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
    oscillator_mix: f64, // 1.0 = entirely osc 0, 0.0 = entirely osc 1.
    amp_envelope: Envelope,
    dca: Dca,

    lfo: Oscillator,
    lfo_routing: LfoRouting,
    lfo_depth: Normal,

    filter: BiQuadFilter,
    filter_cutoff_start: f32,
    filter_cutoff_end: f32,
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
    fn set_pan(&mut self, value: f32) {
        self.dca.set_pan(BipolarNormal::from(value))
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
                    let lfo_for_pitch = lfo * self.lfo_depth.value();
                    for o in self.oscillators.iter_mut() {
                        o.set_frequency_modulation(BipolarNormal::from(lfo_for_pitch));
                    }
                }

                // Oscillators
                let len = self.oscillators.len();
                let osc_sum = match len {
                    0 => 0.0,
                    1 => self.oscillators[0].value() * self.oscillator_mix,
                    2 => {
                        if self.oscillator_2_sync && self.oscillators[0].should_sync() {
                            self.oscillators[1].sync();
                        }
                        self.oscillators[0].value() * self.oscillator_mix
                            + self.oscillators[1].value() * (1.0 - self.oscillator_mix)
                    }
                    _ => todo!(),
                };

                // Filters
                //
                // https://aempass.blogspot.com/2014/09/analog-and-welshs-synthesizer-cookbook.html
                if self.filter_cutoff_end != 0.0 {
                    let new_cutoff_percentage = self.filter_cutoff_start
                        + (1.0 - self.filter_cutoff_start)
                            * self.filter_cutoff_end
                            * filter_env_amplitude.value() as f32;
                    self.filter.set_cutoff_pct(new_cutoff_percentage);
                } else if matches!(self.lfo_routing, LfoRouting::FilterCutoff) {
                    let lfo_for_cutoff = lfo * self.lfo_depth.value();
                    self.filter
                        .set_cutoff_pct(self.filter_cutoff_start * (1.0 + lfo_for_cutoff as f32));
                }
                let filtered_mix = self.filter.transform_channel(0, Sample::from(osc_sum)).0;

                // LFO amplitude modulation
                let lfo_for_amplitude = if matches!(self.lfo_routing, LfoRouting::Amplitude) {
                    // LFO ranges from [-1, 1], so convert to something that can silence or double the volume.
                    lfo * self.lfo_depth.value() + 1.0
                } else {
                    1.0
                };

                // Final
                self.dca.transform_audio_to_stereo(Sample(
                    filtered_mix * amp_env_amplitude.value() * lfo_for_amplitude,
                ))
            } else {
                StereoSample::SILENCE
            };
        }
    }
}
impl WelshVoice {
    fn tick_envelopes(&mut self) -> (Normal, Normal) {
        let was_playing = self.is_playing();
        self.amp_envelope.tick(1);
        let amp_amplitude = self.amp_envelope.value();
        self.filter_envelope.tick(1);
        let filter_amplitude = self.filter_envelope.value();

        if was_playing && !self.is_playing() {
            if self.steal_is_underway {
                self.steal_is_underway = false;
                self.note_on(self.note_on_key, self.note_on_velocity);
            }
        }

        (amp_amplitude, filter_amplitude)
    }

    fn set_frequency_hz(&mut self, frequency_hz: ParameterType) {
        // It's safe to set the frequency on a fixed-frequency oscillator; the
        // fixed frequency is stored separately and takes precedence.
        self.oscillators.iter_mut().for_each(|o| {
            o.set_frequency(frequency_hz);
        });
    }

    pub fn new_with(
        oscillators: Vec<Oscillator>,
        oscillator_2_sync: bool,
        oscillator_mix: f64,
        amp_envelope: Envelope,
        filter: BiQuadFilter,
        filter_cutoff_start: f32,
        filter_cutoff_end: f32,
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
            dca: Default::default(),
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
}

#[derive(Control, Debug, Uid)]
pub struct WelshSynth {
    uid: usize,
    inner_synth: Synthesizer<WelshVoice>,

    // TODO: will it be common for #[controllable] to represent a fake value
    // that's actually propagated to things underneath? If so, do we need a
    // better way to handle this?
    #[controllable]
    #[allow(dead_code)]
    pan: f32,
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
    pub fn new_with(
        sample_rate: usize,
        voice_store: Box<dyn StoresVoices<Voice = WelshVoice>>,
    ) -> Self {
        Self {
            uid: Default::default(),
            inner_synth: Synthesizer::<WelshVoice>::new_with(sample_rate, voice_store),
            pan: Default::default(),
        }
    }

    pub fn preset_name(&self) -> &str {
        "none"
        //        self.preset.name.as_str()
    }

    pub fn pan(&self) -> f32 {
        self.inner_synth.pan()
    }

    pub fn set_pan(&mut self, pan: f32) {
        self.inner_synth.set_pan(pan);
    }

    pub fn set_control_pan(&mut self, value: groove_core::control::F32ControlValue) {
        // TODO: more toil. Let me say this is a bipolar normal
        self.set_pan(value.0 * 2.0 - 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{DEFAULT_BPM, DEFAULT_MIDI_TICKS_PER_SECOND, DEFAULT_SAMPLE_RATE};
    use groove_core::{canonicalize_filename, time::Clock, SampleType};

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
        let mut writer = hound::WavWriter::create(canonicalize_filename(basename), spec).unwrap();

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