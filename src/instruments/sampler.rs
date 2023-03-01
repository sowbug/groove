use super::{SimpleVoiceStore, Synthesizer};
use crate::midi::MidiUtils;
use groove_core::{
    control::F32ControlValue,
    midi::{HandlesMidi, MidiChannel, MidiMessage},
    traits::{
        Controllable, Generates, HasUid, IsInstrument, IsStereoSampleVoice, IsVoice, PlaysNotes,
        Resets, Ticks,
    },
    ParameterType, Sample, SampleType, StereoSample,
};
use groove_macros::{Control, Uid};
use hound::WavReader;
use std::{fs::File, io::BufReader, str::FromStr, sync::Arc};
use strum_macros::{Display, EnumString, FromRepr};

#[derive(Debug)]
pub(crate) struct SamplerVoice {
    sample_rate: usize,
    samples: Arc<Vec<StereoSample>>,

    root_frequency: ParameterType,
    frequency: ParameterType,

    was_reset: bool,
    is_playing: bool,
    sample_pointer: f64,
    sample_pointer_delta: f64,
}
impl IsVoice<StereoSample> for SamplerVoice {}
impl IsStereoSampleVoice for SamplerVoice {}
impl PlaysNotes for SamplerVoice {
    fn is_playing(&self) -> bool {
        self.is_playing
    }

    fn has_pending_events(&self) -> bool {
        false
    }

    #[allow(unused_variables)]
    fn note_on(&mut self, key: u8, velocity: u8) {
        self.is_playing = true;
        self.sample_pointer = 0.0;
        self.frequency = MidiUtils::note_to_frequency(key);
        self.sample_pointer_delta = self.frequency / self.root_frequency;
    }

    #[allow(unused_variables)]
    fn aftertouch(&mut self, velocity: u8) {
        todo!()
    }

    #[allow(unused_variables)]
    fn note_off(&mut self, velocity: u8) {
        self.is_playing = false;
        self.sample_pointer = 0.0;
    }

    #[allow(unused_variables)]
    fn set_pan(&mut self, value: f32) {
        todo!()
    }
}
impl Generates<StereoSample> for SamplerVoice {
    fn value(&self) -> StereoSample {
        self.samples[self.sample_pointer as usize]
    }

    #[allow(unused_variables)]
    fn batch_values(&mut self, values: &mut [StereoSample]) {
        todo!()
    }
}
impl Ticks for SamplerVoice {
    fn tick(&mut self, tick_count: usize) {
        for _ in 0..tick_count {
            if self.is_playing {
                if !self.was_reset {
                    self.sample_pointer += self.sample_pointer_delta;
                }
                if self.sample_pointer as usize >= self.samples.len() {
                    self.is_playing = false;
                    self.sample_pointer = 0.0;
                }
            }
            if self.was_reset {
                self.was_reset = false;
            }
        }
    }
}
impl Resets for SamplerVoice {
    fn reset(&mut self, sample_rate: usize) {
        self.sample_rate = sample_rate;
        self.was_reset = true;
    }
}
impl SamplerVoice {
    pub fn new_with_samples(
        sample_rate: usize,
        samples: Arc<Vec<StereoSample>>,
        root_frequency: ParameterType,
    ) -> Self {
        if !root_frequency.is_normal() {
            panic!("strange number given for root frequency: {root_frequency}");
        }
        Self {
            sample_rate,
            samples,
            root_frequency,
            frequency: Default::default(),
            was_reset: true,
            is_playing: Default::default(),
            sample_pointer: Default::default(),
            sample_pointer_delta: Default::default(),
        }
    }
}

#[derive(Control, Debug, Uid)]
pub struct Sampler {
    uid: usize,
    inner_synth: Synthesizer<SamplerVoice>,
}
impl IsInstrument for Sampler {}
impl HandlesMidi for Sampler {
    fn handle_midi_message(
        &mut self,
        message: &MidiMessage,
    ) -> Option<Vec<(MidiChannel, MidiMessage)>> {
        self.inner_synth.handle_midi_message(message)
    }
}
impl Generates<StereoSample> for Sampler {
    fn value(&self) -> StereoSample {
        self.inner_synth.value()
    }

    #[allow(dead_code, unused_variables)]
    fn batch_values(&mut self, values: &mut [StereoSample]) {
        self.inner_synth.batch_values(values);
    }
}
impl Ticks for Sampler {
    fn tick(&mut self, tick_count: usize) {
        self.inner_synth.tick(tick_count)
    }
}
impl Resets for Sampler {
    fn reset(&mut self, sample_rate: usize) {
        self.inner_synth.reset(sample_rate)
    }
}
impl Sampler {
    pub fn new_with_filename(sample_rate: usize, filename: &str) -> Self {
        if let Ok(samples) = Self::read_samples_from_file(filename) {
            let samples = Arc::new(samples);
            let root_frequency = 440.0; // TODO #6
            Self {
                uid: Default::default(),
                inner_synth: Synthesizer::<SamplerVoice>::new_with(
                    sample_rate,
                    Box::new(SimpleVoiceStore::<SamplerVoice>::new_with_voice(
                        sample_rate,
                        || {
                            SamplerVoice::new_with_samples(
                                sample_rate,
                                Arc::clone(&samples),
                                root_frequency,
                            )
                        },
                    )),
                ),
            }
        } else {
            panic!("Couldn't load sample {}", filename);
        }
    }

    fn read_samples<T>(
        reader: &mut WavReader<BufReader<File>>,
        channels: u16,
        scale_factor: SampleType,
    ) -> anyhow::Result<Vec<StereoSample>>
    where
        Sample: From<T>,
        T: hound::Sample,
    {
        let mut samples = Vec::default();
        if channels == 1 {
            for sample in reader.samples::<T>().flatten() {
                samples.push(StereoSample::from(Sample::from(sample) / scale_factor));
            }
        } else {
            debug_assert_eq!(channels, 2);
            loop {
                let mut iter = reader.samples::<T>();
                let left = iter.next();
                if let Some(Ok(left)) = left {
                    let right = iter.next();
                    if let Some(Ok(right)) = right {
                        let left = Sample::from(left) / scale_factor;
                        let right = Sample::from(right) / scale_factor;
                        samples.push(StereoSample(left, right));
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
        Ok(samples)
    }

    pub fn read_samples_from_file(filename: &str) -> anyhow::Result<Vec<StereoSample>> {
        let mut reader = hound::WavReader::open(filename)?;
        let spec = reader.spec();
        let itype_max: SampleType = 2.0f64.powi(spec.bits_per_sample as i32 - 1);

        match spec.sample_format {
            hound::SampleFormat::Float => {
                Self::read_samples::<f32>(&mut reader, spec.channels, itype_max)
            }
            hound::SampleFormat::Int => {
                Self::read_samples::<i32>(&mut reader, spec.channels, itype_max)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        common::DEFAULT_SAMPLE_RATE, instruments::tests::is_voice_makes_any_sound_at_all,
        utils::Paths,
    };

    #[test]
    fn test_loading() {
        let mut filename = Paths::test_data_path();
        filename.push("stereo-pluck.wav");
        let _ = Sampler::new_with_filename(DEFAULT_SAMPLE_RATE, filename.to_str().unwrap());
    }

    #[test]
    fn sampler_makes_any_sound_at_all() {
        let samples =
            Sampler::read_samples_from_file("test-data/square-440Hz-1-second-mono-24-bit-PCM.wav");
        assert!(samples.is_ok());
        let samples = samples.unwrap();
        let mut voice =
            SamplerVoice::new_with_samples(DEFAULT_SAMPLE_RATE, Arc::new(samples), 440.0);
        voice.note_on(1, 127);

        assert!(
            is_voice_makes_any_sound_at_all(&mut voice),
            "once triggered, SamplerVoice should make a sound"
        );
    }
}
