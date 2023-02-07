use crate::{
    clock::Clock,
    common::{F32ControlValue, Sample, SampleType},
    midi::MidiMessage,
    traits::{Controllable, HasUid, IsInstrument, SourcesAudio},
    StereoSample,
};
use groove_macros::{Control, Uid};
use hound::WavReader;
use std::{fs::File, io::BufReader, str::FromStr};
use strum_macros::{Display, EnumString, FromRepr};

use super::HandlesMidi;

#[derive(Control, Debug, Default, Uid)]
#[allow(dead_code)]
pub struct Sampler {
    uid: usize,
    samples: Vec<StereoSample>,
    sample_clock_start: usize,
    sample_pointer: usize,
    is_playing: bool,
    root_frequency: f32, // TODO: make not dead

    filename: String,
}
impl IsInstrument for Sampler {}
impl SourcesAudio for Sampler {
    fn source_audio(&mut self, clock: &Clock) -> crate::StereoSample {
        if self.sample_clock_start == usize::MAX || clock.was_reset() {
            self.sample_clock_start = clock.samples();
        }

        debug_assert!(self.sample_clock_start <= clock.samples());

        self.sample_pointer = clock.samples() - self.sample_clock_start;
        if self.sample_pointer >= self.samples.len() {
            self.is_playing = false;
            self.sample_pointer = 0;
        }

        if self.is_playing {
            *self
                .samples
                .get(self.sample_pointer)
                .unwrap_or(&StereoSample::SILENCE)
        } else {
            StereoSample::SILENCE
        }
    }
}
impl HandlesMidi for Sampler {
    fn handle_midi_message(&mut self, message: &MidiMessage) {
        #[allow(unused_variables)]
        match message {
            MidiMessage::NoteOff { key, vel } => {
                self.is_playing = false;
            }
            MidiMessage::NoteOn { key, vel } => {
                self.is_playing = true;

                self.sample_pointer = 0;

                // Slight hack to tell ourselves to record the sample start time
                // on next source_audio().
                self.sample_clock_start = usize::MAX;
            }
            _ => {}
        }
    }
}

impl Sampler {
    fn read_samples<T>(
        reader: &mut WavReader<BufReader<File>>,
        channels: u16,
        scale_factor: SampleType,
    ) -> Vec<StereoSample>
    where
        Sample: From<T>,
        T: hound::Sample,
    {
        let mut samples = Vec::default();
        if channels == 1 {
            for sample in reader.samples::<T>() {
                if let Ok(sample) = sample {
                    let sample = Sample::from(sample) / scale_factor;
                    samples.push(StereoSample::from(sample));
                }
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
        samples
    }
    pub fn read_samples_from_file(filename: &str) -> Vec<StereoSample> {
        let mut reader = hound::WavReader::open(filename).unwrap();
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

    pub fn new_from_file(filename: &str) -> Self {
        let samples = Self::read_samples_from_file(filename);
        let mut r = Self::default();
        r.samples = samples;
        r.filename = String::from(filename);
        r
    }

    pub fn filename(&self) -> &str {
        self.filename.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::Paths;

    #[test]
    fn test_loading() {
        let mut filename = Paths::asset_path();
        filename.push("samples");
        filename.push("test.wav");
        let _ = Sampler::new_from_file(filename.to_str().unwrap());
    }
}
