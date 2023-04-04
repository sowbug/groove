// Copyright (c) 2023 Mike Tsao. All rights reserved.

use anyhow::{anyhow, Result};
use groove_core::{
    instruments::Synthesizer,
    midi::{note_to_frequency, HandlesMidi, MidiChannel, MidiMessage},
    traits::{Generates, IsInstrument, IsStereoSampleVoice, IsVoice, PlaysNotes, Resets, Ticks},
    voices::VoiceStore,
    BipolarNormal, FrequencyHz, ParameterType, Sample, SampleType, StereoSample,
};
use groove_proc_macros::{Nano, Uid};
use hound::WavReader;
use std::{
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    str::FromStr,
    sync::Arc,
};
use strum::EnumCount;
use strum_macros::{Display, EnumCount as EnumCountMacro, EnumString, FromRepr, IntoStaticStr};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub(crate) struct SamplerVoice {
    sample_rate: usize,
    samples: Arc<Vec<StereoSample>>,

    root_frequency: FrequencyHz,
    frequency: FrequencyHz,

    was_reset: bool,
    is_playing: bool,
    sample_pointer: ParameterType,
    sample_pointer_delta: ParameterType,
}
impl IsVoice<StereoSample> for SamplerVoice {}
impl IsStereoSampleVoice for SamplerVoice {}
impl PlaysNotes for SamplerVoice {
    fn is_playing(&self) -> bool {
        self.is_playing
    }

    #[allow(unused_variables)]
    fn note_on(&mut self, key: u8, velocity: u8) {
        self.is_playing = true;
        self.sample_pointer = 0.0;
        self.frequency = note_to_frequency(key);
        self.sample_pointer_delta = (self.frequency / self.root_frequency).into();
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
    fn set_pan(&mut self, value: BipolarNormal) {
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
        root_frequency: FrequencyHz,
    ) -> Self {
        if !root_frequency.value().is_normal() {
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

#[derive(Debug, Nano, Uid)]
pub struct Sampler {
    uid: usize,
    inner_synth: Synthesizer<SamplerVoice>,

    #[nano]
    root_frequency: FrequencyHz,
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
    pub fn new_with_filename(
        sample_rate: usize,
        filename: &str,
        root_frequency: Option<FrequencyHz>,
    ) -> Self {
        if let Ok(samples) = Self::read_samples_from_file(filename) {
            let samples = Arc::new(samples);

            let root_frequency = if let Some(root_frequency) = root_frequency {
                root_frequency
            } else if let Ok(root_frequency) = Self::read_riff_metadata(filename) {
                note_to_frequency(root_frequency)
            } else {
                FrequencyHz::from(440.0)
            };

            Self {
                uid: Default::default(),
                inner_synth: Synthesizer::<SamplerVoice>::new_with(
                    sample_rate,
                    Box::new(VoiceStore::<SamplerVoice>::new_with_voice(
                        sample_rate,
                        8,
                        || {
                            SamplerVoice::new_with_samples(
                                sample_rate,
                                Arc::clone(&samples),
                                root_frequency,
                            )
                        },
                    )),
                ),
                root_frequency,
            }
        } else {
            panic!("Couldn't load sample {}", filename);
        }
    }

    // https://forums.cockos.com/showthread.php?t=227118
    //
    // ** The acid chunk goes a little something like this:
    // **
    // ** 4 bytes          'acid'
    // ** 4 bytes (int)     length of chunk starting at next byte
    // **
    // ** 4 bytes (int)     type of file:
    // **        this appears to be a bit mask,however some combinations
    // **        are probably impossible and/or qualified as "errors"
    // **
    // **        0x01 On: One Shot         Off: Loop
    // **        0x02 On: Root note is Set Off: No root
    // **        0x04 On: Stretch is On,   Off: Strech is OFF
    // **        0x08 On: Disk Based       Off: Ram based
    // **        0x10 On: ??????????       Off: ????????? (Acidizer puts that ON)
    // **
    // ** 2 bytes (short)      root note
    // **        if type 0x10 is OFF : [C,C#,(...),B] -> [0x30 to 0x3B]
    // **        if type 0x10 is ON  : [C,C#,(...),B] -> [0x3C to 0x47]
    // **         (both types fit on same MIDI pitch albeit different octaves, so who cares)
    // **
    // ** 2 bytes (short)      ??? always set to 0x8000
    // ** 4 bytes (float)      ??? seems to be always 0
    // ** 4 bytes (int)        number of beats
    // ** 2 bytes (short)      meter denominator   //always 4 in SF/ACID
    // ** 2 bytes (short)      meter numerator     //always 4 in SF/ACID
    // **                      //are we sure about the order?? usually its num/denom
    // ** 4 bytes (float)      tempo
    // **
    fn read_riff_metadata(filename: &str) -> Result<u8> {
        let riff = riff_io::RiffFile::open(filename)?;
        let entries = riff.read_entries()?;
        for entry in entries {
            match entry {
                riff_io::Entry::Chunk(chunk) => {
                    // looking for chunk_id 'acid'
                    if chunk.chunk_id == [97, 99, 105, 100] {
                        let mut f = File::open(filename)?;
                        f.seek(SeekFrom::Start(chunk.data_offset as u64))?;
                        let mut bytes = Vec::default();
                        bytes.resize(chunk.data_size, 0);
                        let _ = f.read(&mut bytes)?;

                        let root_note_set = bytes[0] & 0x02 != 0;
                        let pitch_b = bytes[0] & 0x10 != 0;

                        if root_note_set {
                            // TODO: find a real WAV that has the pitch_b flag set
                            let root_note = bytes[4] - if pitch_b { 12 } else { 0 };
                            return Ok(root_note);
                        }
                    }
                }
                _ => {}
            }
        }
        Err(anyhow!("couldn't find root note in acid RIFF chunk"))
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

    pub fn root_frequency(&self) -> FrequencyHz {
        self.root_frequency
    }

    pub fn update(&mut self, message: SamplerMessage) {
        match message {
            SamplerMessage::Sampler(_) => todo!(),
            _ => self.derived_update(message),
        }
    }

    pub fn set_root_frequency(&mut self, root_frequency: FrequencyHz) {
        self.root_frequency = root_frequency;
        todo!("propagate to voices")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::DEFAULT_SAMPLE_RATE;
    use std::path::PathBuf;

    #[test]
    fn test_loading() {
        let filename = PathBuf::from("test-data/stereo-pluck.wav");
        let sampler =
            Sampler::new_with_filename(DEFAULT_SAMPLE_RATE, filename.to_str().unwrap(), None);
        assert_eq!(sampler.root_frequency(), FrequencyHz::from(440.0));
    }

    #[test]
    fn test_reading_acidized_metadata() {
        let filename = PathBuf::from("test-data/riff-acidized.wav");
        let root_note = Sampler::read_riff_metadata(filename.to_str().unwrap());
        assert!(root_note.is_ok());
        assert_eq!(root_note.unwrap(), 57);

        let filename = PathBuf::from("test-data/riff-not-acidized.wav");
        let root_note = Sampler::read_riff_metadata(filename.to_str().unwrap());
        assert!(root_note.is_err());
    }

    //    #[test]
    #[allow(dead_code)]
    fn test_reading_smpl_metadata() {
        let filename = PathBuf::from("test-data/riff-with-smpl.wav");
        let root_note = Sampler::read_riff_metadata(filename.to_str().unwrap());
        assert!(root_note.is_ok());
        assert_eq!(root_note.unwrap(), 255);
    }

    #[test]
    fn test_loading_with_root_frequency() {
        let filename = PathBuf::from("test-data/riff-acidized.wav");

        let sampler =
            Sampler::new_with_filename(DEFAULT_SAMPLE_RATE, filename.to_str().unwrap(), None);
        assert_eq!(
            sampler.root_frequency(),
            note_to_frequency(57),
            "acidized WAV should produce sample with embedded root note"
        );

        let sampler = Sampler::new_with_filename(
            DEFAULT_SAMPLE_RATE,
            filename.to_str().unwrap(),
            Some(FrequencyHz::from(123.0)),
        );
        assert_eq!(
            sampler.root_frequency(),
            FrequencyHz::from(123.0),
            "specified parameter should override acidized WAV's embedded root note"
        );

        let filename = PathBuf::from("test-data/riff-not-acidized.wav");

        let sampler = Sampler::new_with_filename(
            DEFAULT_SAMPLE_RATE,
            filename.to_str().unwrap(),
            Some(FrequencyHz::from(123.0)),
        );
        assert_eq!(
            sampler.root_frequency(),
            FrequencyHz::from(123.0),
            "specified parameter should be used for non-acidized WAV"
        );

        let sampler =
            Sampler::new_with_filename(DEFAULT_SAMPLE_RATE, filename.to_str().unwrap(), None);
        assert_eq!(
            sampler.root_frequency(),
            note_to_frequency(69),
            "If there is neither an acidized WAV nor a provided frequency, sample should have root note A4 (440Hz)"
        );
    }

    #[test]
    fn sampler_makes_any_sound_at_all() {
        let samples =
            Sampler::read_samples_from_file("test-data/square-440Hz-1-second-mono-24-bit-PCM.wav");
        assert!(samples.is_ok());
        let samples = samples.unwrap();
        let mut voice = SamplerVoice::new_with_samples(
            DEFAULT_SAMPLE_RATE,
            Arc::new(samples),
            FrequencyHz::from(440.0),
        );
        voice.note_on(1, 127);

        // Skip a few frames in case attack is slow
        voice.tick(5);
        assert!(
            voice.value() != StereoSample::SILENCE,
            "once triggered, SamplerVoice should make a sound"
        );
    }
}
