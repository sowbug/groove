// Copyright (c) 2023 Mike Tsao. All rights reserved.

use anyhow::{anyhow, Result};
use ensnare::prelude::*;
use groove_core::{
    instruments::Synthesizer,
    midi::{note_to_frequency, HandlesMidi, MidiChannel, MidiMessage, MidiMessagesFn},
    time::SampleRate,
    traits::{
        Configurable, Generates, IsStereoSampleVoice, IsVoice, PlaysNotes, Serializable, Ticks,
    },
    voices::{VoiceCount, VoiceStore},
};
use groove_proc_macros::{Control, IsInstrument, Params, Uid};
use groove_utils::Paths;
use hound::WavReader;
use std::{fs::File, io::BufReader, path::Path, sync::Arc};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Default)]
pub struct SamplerVoice {
    sample_rate: SampleRate,
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
}
impl Generates<StereoSample> for SamplerVoice {
    fn value(&self) -> StereoSample {
        self.samples[self.sample_pointer as usize]
    }

    #[allow(unused_variables)]
    fn generate_batch_values(&mut self, values: &mut [StereoSample]) {
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
impl Serializable for SamplerVoice {}
impl Configurable for SamplerVoice {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.was_reset = true;
    }
}
impl SamplerVoice {
    pub fn new_with_samples(samples: Arc<Vec<StereoSample>>, root_frequency: FrequencyHz) -> Self {
        if !root_frequency.value().is_normal() {
            panic!("strange number given for root frequency: {root_frequency}");
        }
        Self {
            sample_rate: Default::default(),
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

#[derive(Debug, Control, IsInstrument, Params, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Sampler {
    uid: Uid,

    #[cfg_attr(feature = "serialization", serde(skip))]
    inner_synth: Synthesizer<SamplerVoice>,

    #[params]
    filename: String,

    #[control]
    #[params]
    root: FrequencyHz,

    calculated_root: FrequencyHz,
}
impl HandlesMidi for Sampler {
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
impl Generates<StereoSample> for Sampler {
    fn value(&self) -> StereoSample {
        self.inner_synth.value()
    }

    #[allow(dead_code, unused_variables)]
    fn generate_batch_values(&mut self, values: &mut [StereoSample]) {
        self.inner_synth.generate_batch_values(values);
    }
}
impl Ticks for Sampler {
    fn tick(&mut self, tick_count: usize) {
        self.inner_synth.tick(tick_count)
    }
}
impl Serializable for Sampler {}
impl Configurable for Sampler {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.inner_synth.update_sample_rate(sample_rate)
    }
}
impl Sampler {
    pub fn new_with(params: &SamplerParams, paths: &Paths) -> Self {
        let path = paths.build_sample(&Vec::default(), Path::new(&params.filename()));
        if let Ok(file) = paths.search_and_open(path.as_path()) {
            if let Ok(mut f2) = file.try_clone() {
                if let Ok(samples) = Self::read_samples_from_file(&file) {
                    let samples = Arc::new(samples);

                    let calculated_root_frequency = if params.root().value() > 0.0 {
                        params.root()
                    } else if let Ok(embedded_root_note) = Self::read_riff_metadata(&mut f2) {
                        note_to_frequency(embedded_root_note)
                    } else {
                        FrequencyHz::from(440.0)
                    };

                    Self {
                        uid: Default::default(),
                        inner_synth: Synthesizer::<SamplerVoice>::new_with(Box::new(
                            VoiceStore::<SamplerVoice>::new_with_voice(VoiceCount::from(8), || {
                                SamplerVoice::new_with_samples(
                                    Arc::clone(&samples),
                                    calculated_root_frequency,
                                )
                            }),
                        )),
                        filename: params.filename().to_string(),
                        root: params.root(),
                        calculated_root: calculated_root_frequency,
                    }
                } else {
                    panic!("Couldn't load sample {}", &params.filename());
                }
            } else {
                panic!("Couldn't create second file handle to read metadata");
            }
        } else {
            panic!("Couldn't read file {:?}", path);
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
    fn read_riff_metadata(_file: &mut File) -> Result<u8> {
        Err(anyhow!("riff_io crate is excluded"))
        // let riff = riff_io::RiffFile::open_with_file_handle(file)?;
        // let entries = riff.read_entries()?;
        // for entry in entries {
        //     match entry {
        //         riff_io::Entry::Chunk(chunk) => {
        //             // looking for chunk_id 'acid'
        //             if chunk.chunk_id == [97, 99, 105, 100] {
        //                 file.seek(std::io::SeekFrom::Start(chunk.data_offset as u64))?;
        //                 let mut bytes = Vec::default();
        //                 bytes.resize(chunk.data_size, 0);
        //                 let _ = file.read(&mut bytes)?;

        //                 let root_note_set = bytes[0] & 0x02 != 0;
        //                 let pitch_b = bytes[0] & 0x10 != 0;

        //                 if root_note_set {
        //                     // TODO: find a real WAV that has the pitch_b flag set
        //                     let root_note = bytes[4] - if pitch_b { 12 } else { 0 };
        //                     return Ok(root_note);
        //                 }
        //             }
        //         }
        //         _ => {}
        //     }
        // }
        // Err(anyhow!("Couldn't find root note in acid RIFF chunk"))
    }

    fn read_samples<T>(
        reader: &mut WavReader<BufReader<&File>>,
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

    pub fn read_samples_from_file(file: &File) -> anyhow::Result<Vec<StereoSample>> {
        let mut reader = hound::WavReader::new(BufReader::new(file))?;
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

    pub fn root(&self) -> FrequencyHz {
        self.root
    }

    pub fn set_root(&mut self, root: FrequencyHz) {
        self.root = root;
        todo!("propagate to voices")
    }

    pub fn calculated_root(&self) -> FrequencyHz {
        self.calculated_root
    }

    pub fn set_calculated_root(&mut self, calculated_root: FrequencyHz) {
        self.calculated_root = calculated_root;
    }

    pub fn filename(&self) -> &str {
        self.filename.as_ref()
    }

    pub fn set_filename(&mut self, filename: String) {
        self.filename = filename;
    }
}

#[cfg(feature = "egui-framework")]
mod gui {
    use super::Sampler;
    use eframe::egui::Ui;
    use groove_core::traits::{gui::Displays, HasUid};

    impl Displays for Sampler {
        fn ui(&mut self, ui: &mut Ui) -> eframe::egui::Response {
            ui.label(self.name())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn paths_with_test_data_dir() -> Paths {
        let mut paths = Paths::default();
        paths.push_hive(Paths::test_data_rel());
        paths
    }

    #[test]
    fn loading() {
        let paths = paths_with_test_data_dir();
        let sampler = Sampler::new_with(
            &SamplerParams {
                filename: "stereo-pluck.wav".to_string(),
                root: 0.0.into(),
            },
            &paths,
        );
        assert_eq!(sampler.calculated_root(), FrequencyHz::from(440.0));
    }

    #[test]
    #[ignore = "Re-enable when SamplerParams knows how to handle String"]
    fn reading_acidized_metadata() {
        let filename = PathBuf::from("riff-acidized.wav");
        let mut file = std::fs::File::open(filename).unwrap();
        let root_note = Sampler::read_riff_metadata(&mut file);
        assert!(root_note.is_ok());
        assert_eq!(root_note.unwrap(), 57);

        let filename = PathBuf::from("riff-not-acidized.wav");
        let mut file = std::fs::File::open(filename).unwrap();
        let root_note = Sampler::read_riff_metadata(&mut file);
        assert!(root_note.is_err());
    }

    //    #[test]
    #[allow(dead_code)]
    fn reading_smpl_metadata() {
        let filename = PathBuf::from("riff-with-smpl.wav");
        let mut file = std::fs::File::open(filename).unwrap();
        let root_note = Sampler::read_riff_metadata(&mut file);
        assert!(root_note.is_ok());
        assert_eq!(root_note.unwrap(), 255);
    }

    #[test]
    #[ignore = "riff_io crate is disabled, so we can't read root frequencies from files"]
    fn loading_with_root_frequency() {
        let paths = paths_with_test_data_dir();
        let sampler = Sampler::new_with(
            &SamplerParams {
                filename: "riff-acidized.wav".to_string(),
                root: 0.0.into(),
            },
            &paths,
        );
        eprintln!("calculated {} ", sampler.calculated_root());
        assert_eq!(
            sampler.calculated_root(),
            note_to_frequency(57),
            "acidized WAV should produce sample with embedded root note"
        );

        let sampler = Sampler::new_with(
            &SamplerParams {
                filename: "riff-acidized.wav".to_string(),
                root: 123.0.into(),
            },
            &paths,
        );
        assert_eq!(
            sampler.calculated_root(),
            FrequencyHz::from(123.0),
            "specified parameter should override acidized WAV's embedded root note"
        );

        let sampler = Sampler::new_with(
            &SamplerParams {
                filename: "riff-not-acidized.wav".to_string(),
                root: 123.0.into(),
            },
            &paths,
        );
        assert_eq!(
            sampler.calculated_root(),
            FrequencyHz::from(123.0),
            "specified parameter should be used for non-acidized WAV"
        );

        let sampler = Sampler::new_with(
            &SamplerParams {
                filename: "riff-not-acidized.wav".to_string(),
                root: 0.0.into(),
            },
            &paths,
        );
        assert_eq!(
            sampler.calculated_root(),
            note_to_frequency(69),
            "If there is neither an acidized WAV nor a provided frequency, sample should have root note A4 (440Hz)"
        );
    }

    #[test]
    fn sampler_makes_any_sound_at_all() {
        let paths = paths_with_test_data_dir();
        let file = paths.search_and_open_with_file_type(
            groove_utils::FileType::Sample,
            Path::new("square-440Hz-1-second-mono-24-bit-PCM.wav"),
        );
        assert!(file.is_ok());
        let samples = Sampler::read_samples_from_file(&file.unwrap());
        assert!(samples.is_ok());
        let samples = samples.unwrap();
        let mut voice = SamplerVoice::new_with_samples(Arc::new(samples), FrequencyHz::from(440.0));
        voice.note_on(1, 127);

        // Skip a few frames in case attack is slow
        voice.tick(5);
        assert!(
            voice.value() != StereoSample::SILENCE,
            "once triggered, SamplerVoice should make a sound"
        );
    }
}
