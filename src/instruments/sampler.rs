use crate::{
    clock::Clock,
    common::{F32ControlValue, Sample, SampleType},
    messages::EntityMessage,
    midi::MidiMessage,
    traits::{Controllable, HasUid, IsInstrument, Response, SourcesAudio, Updateable},
    StereoSample,
};
use groove_macros::{Control, Uid};
use hound::WavReader;
use std::{fs::File, io::BufReader, str::FromStr};
use strum_macros::{Display, EnumString, FromRepr};

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
        // TODO: when we got rid of WatchesClock, we lost the concept of "done."
        // Be on the lookout for clipped audio.
        if self.sample_clock_start > clock.samples() {
            self.is_playing = false;
            self.sample_pointer = 0;
        } else {
            self.sample_pointer = clock.samples() - self.sample_clock_start;
            if self.sample_pointer >= self.samples.len() {
                self.is_playing = false;
                self.sample_pointer = 0;
            }
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
impl Updateable for Sampler {
    type Message = EntityMessage;

    fn update(&mut self, clock: &Clock, message: Self::Message) -> Response<Self::Message> {
        #[allow(unused_variables)]
        if let Self::Message::Midi(channel, message) = message {
            match message {
                MidiMessage::NoteOff { key, vel } => {
                    self.is_playing = false;
                }
                MidiMessage::NoteOn { key, vel } => {
                    self.sample_pointer = 0;
                    self.sample_clock_start = clock.samples();
                    self.is_playing = true;
                }
                _ => {}
            }
        }
        Response::none()
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
