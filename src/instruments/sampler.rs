use crate::{
    clock::Clock,
    common::MonoSample,
    messages::EntityMessage,
    midi::MidiMessage,
    traits::{HasUid, IsInstrument, Response, SourcesAudio, Updateable},
};

#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct Sampler {
    uid: usize,
    samples: Vec<MonoSample>,
    sample_clock_start: usize,
    sample_pointer: usize,
    is_playing: bool,
    root_frequency: f32, // TODO: make not dead

    pub(crate) filename: String,
}
impl IsInstrument for Sampler {}
impl SourcesAudio for Sampler {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
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
            let sample = *self.samples.get(self.sample_pointer).unwrap_or(&0.0);
            sample
        } else {
            0.0
        }
    }
}
impl Updateable for Sampler {
    type Message = EntityMessage;

    fn update(&mut self, clock: &Clock, message: Self::Message) -> Response<Self::Message> {
        #[allow(unused_variables)]
        match message {
            Self::Message::Midi(channel, message) => match message {
                MidiMessage::NoteOff { key, vel } => {
                    self.is_playing = false;
                }
                MidiMessage::NoteOn { key, vel } => {
                    self.sample_pointer = 0;
                    self.sample_clock_start = clock.samples();
                    self.is_playing = true;
                }
                _ => {}
            },
            _ => {}
        }
        Response::none()
    }
}
impl HasUid for Sampler {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}

impl Sampler {
    pub(crate) fn new_with(buffer_size: usize) -> Self {
        Self {
            samples: Vec::with_capacity(buffer_size),
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    pub fn new_from_file(filename: &str) -> Self {
        let mut reader = hound::WavReader::open(filename).unwrap();
        let mut r = Self::new_with(reader.duration() as usize);
        for sample in reader.samples::<i16>() {
            r.samples
                .push(sample.unwrap() as MonoSample / i16::MAX as MonoSample);
        }
        r.filename = filename.to_string();
        r
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
