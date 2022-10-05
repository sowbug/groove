use crate::{
    clock::Clock,
    common::MonoSample,
    midi::{MidiChannel, MidiMessage, MidiMessageType},
    traits::{SinksControl, SinksControlParam, SinksMidi, SourcesAudio},
};

#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct Sampler {
    midi_channel: MidiChannel,
    samples: Vec<MonoSample>,
    sample_clock_start: usize,
    sample_pointer: usize,
    is_playing: bool,
    root_frequency: f32, // TODO: make not dead
}

impl Sampler {
    pub fn new(midi_channel: MidiChannel, buffer_size: usize) -> Self {
        Self {
            midi_channel,
            samples: Vec::with_capacity(buffer_size),
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    pub fn new_from_file(midi_channel: MidiChannel, filename: &str) -> Self {
        let mut reader = hound::WavReader::open(filename).unwrap();
        let mut r = Self::new(midi_channel, reader.duration() as usize);
        for sample in reader.samples::<i16>() {
            r.samples
                .push(sample.unwrap() as MonoSample / i16::MAX as MonoSample);
        }
        r
    }
}

impl SinksControl for Sampler {
    fn handle_control(&mut self, _clock: &Clock, _param: &SinksControlParam) {
        todo!()
    }
}
impl SinksMidi for Sampler {
    fn midi_channel(&self) -> MidiChannel {
        self.midi_channel
    }

    fn set_midi_channel(&mut self, midi_channel: MidiChannel) {
        self.midi_channel = midi_channel;
    }

    fn handle_midi_for_channel(&mut self, clock: &Clock, message: &MidiMessage) {
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
}
impl SourcesAudio for Sampler {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        // TODO: when we got rid of WatchesClock, we lost the concept of "done."
        // Be on the lookout for clipped audio.
        self.sample_pointer = clock.samples as usize - self.sample_clock_start;
        if self.sample_pointer >= self.samples.len() {
            self.is_playing = false;
            self.sample_pointer = 0;
        }

        if self.is_playing {
            let sample = *self
                .samples
                .get(self.sample_pointer as usize)
                .unwrap_or(&0.0);
            sample
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::midi::MIDI_CHANNEL_RECEIVE_NONE;

    use super::*;

    #[test]
    fn test_loading() {
        let _ = Sampler::new_from_file(MIDI_CHANNEL_RECEIVE_NONE, "samples/test.wav");
    }
}
