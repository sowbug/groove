use hound;

use crate::{common::MidiMessageType, devices::traits::DeviceTrait};

#[derive(Default)]
#[allow(dead_code)]
pub struct Sampler {
    samples: Vec<f32>,
    sample_clock_start: usize,
    sample_pointer: usize,
    is_playing: bool,
    root_frequency: f32, // TODO: make not dead
}

impl Sampler {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            samples: Vec::with_capacity(buffer_size),
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    pub fn new_from_file(filename: &str) -> Self {
        let mut reader = hound::WavReader::open(filename).unwrap();
        let mut r = Self::new(reader.duration() as usize);
        for sample in reader.samples::<i16>() {
            r.samples.push(sample.unwrap() as f32 / i16::MAX as f32);
        }
        r
    }
}

impl DeviceTrait for Sampler {
    fn sources_audio(&self) -> bool {
        true
    }
    fn sinks_midi(&self) -> bool {
        true
    }

    fn handle_midi_message(
        &mut self,
        message: &crate::common::MidiMessage,
        clock: &crate::primitives::clock::Clock,
    ) {
        match message.status {
            MidiMessageType::NoteOn => {
                self.sample_pointer = 0;
                self.sample_clock_start = clock.samples as usize;
                self.is_playing = true;
            }
            MidiMessageType::NoteOff => {
                self.is_playing = false;
            }
            MidiMessageType::ProgramChange => {}
        }
    }

    fn get_audio_sample(&mut self) -> f32 {
        if self.is_playing {
            let sample: f32 = *self
                .samples
                .get(self.sample_pointer as usize)
                .unwrap_or(&0.0f32);
            sample
        } else {
            0.0
        }
    }

    fn tick(&mut self, clock: &crate::primitives::clock::Clock) -> bool {
        self.sample_pointer = clock.samples as usize - self.sample_clock_start;
        if self.sample_pointer >= self.samples.len() {
            self.is_playing = false;
            self.sample_pointer = 0;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loading() {
        let _ = Sampler::new_from_file("samples/test.wav");
    }
}
