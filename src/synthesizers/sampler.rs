use crate::{
    clock::Clock,
    common::{rrc, rrc_downgrade, MonoSample, Rrc, Ww},
    midi::{MidiChannel, MidiMessage, MidiMessageType},
    traits::{IsMutable, SinksMidi, SourcesAudio},
};

#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct Sampler {
    pub(crate) me: Ww<Self>,
    midi_channel: MidiChannel,
    samples: Vec<MonoSample>,
    sample_clock_start: usize,
    sample_pointer: usize,
    is_playing: bool,
    is_muted: bool,
    root_frequency: f32, // TODO: make not dead

    pub(crate) filename: String,
}

impl Sampler {
    fn new(midi_channel: MidiChannel, buffer_size: usize) -> Self {
        Self {
            midi_channel,
            samples: Vec::with_capacity(buffer_size),
            ..Default::default()
        }
    }

    #[allow(dead_code)] // TODO: add a setting for Sampler
    pub fn new_wrapped_with(midi_channel: MidiChannel, buffer_size: usize) -> Rrc<Self> {
        let wrapped = rrc(Self::new(midi_channel, buffer_size));
        wrapped.borrow_mut().me = rrc_downgrade(&wrapped);
        wrapped
    }

    #[allow(dead_code)]
    pub fn new_from_file(midi_channel: MidiChannel, filename: &str) -> Self {
        let mut reader = hound::WavReader::open(filename).unwrap();
        let mut r = Self::new(midi_channel, reader.duration() as usize);
        for sample in reader.samples::<i16>() {
            r.samples
                .push(sample.unwrap() as MonoSample / i16::MAX as MonoSample);
        }
        r.filename = filename.to_string();
        r
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
                self.sample_clock_start = clock.samples();
                self.is_playing = true;
            }
            MidiMessageType::NoteOff => {
                self.is_playing = false;
            }
            MidiMessageType::ProgramChange => {}
            MidiMessageType::Controller => todo!(),
            // TODO: there's way too much duplication across synths and samplers
            // and voices
        }
    }
}
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
impl IsMutable for Sampler {
    fn is_muted(&self) -> bool {
        self.is_muted
    }

    fn set_muted(&mut self, is_muted: bool) {
        self.is_muted = is_muted;
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
