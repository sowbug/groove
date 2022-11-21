use crate::{
    clock::Clock,
    common::{rrc, MonoSample, Rrc, Ww},
    messages::GrooveMessage,
    midi::{MidiChannel, MidiMessage},
    traits::{
        HasUid, NewIsInstrument, NewUpdateable, SinksMidi, SourcesAudio,
    },
};

#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct Sampler {
    uid: usize,



    midi_channel: MidiChannel,
    samples: Vec<MonoSample>,
    sample_clock_start: usize,
    sample_pointer: usize,
    is_playing: bool,
    root_frequency: f32, // TODO: make not dead

    pub(crate) filename: String,
}
impl NewIsInstrument for Sampler {}
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
impl NewUpdateable for Sampler {
    type Message = GrooveMessage;
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
    pub(crate) fn new_with(midi_channel: MidiChannel, buffer_size: usize) -> Self {
        Self {
            midi_channel,
            samples: Vec::with_capacity(buffer_size),
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    pub fn new_from_file(midi_channel: MidiChannel, filename: &str) -> Self {
        let mut reader = hound::WavReader::open(filename).unwrap();
        let mut r = Self::new_with(midi_channel, reader.duration() as usize);
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

    fn handle_midi_for_channel(
        &mut self,
        clock: &Clock,
        _channel: &MidiChannel,
        message: &MidiMessage,
    ) {
        #[allow(unused_variables)]
        match message {
            MidiMessage::NoteOff { key, vel } => {
                self.is_playing = false;
            }
            MidiMessage::NoteOn { key, vel } => {
                self.sample_pointer = 0;
                self.sample_clock_start = clock.samples();
                self.is_playing = true;
            }
            MidiMessage::Aftertouch { key, vel } => todo!(),
            MidiMessage::Controller { controller, value } => todo!(),
            MidiMessage::ProgramChange { program } => todo!(),
            MidiMessage::ChannelAftertouch { vel } => todo!(),
            MidiMessage::PitchBend { bend } => todo!(),
        }
        // TODO: there's way too much duplication across synths and samplers
        // and voices
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
