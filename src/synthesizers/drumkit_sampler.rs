use std::collections::HashMap;

use hound;

use crate::{
    common::{MidiChannel, MidiMessage, MidiMessageType, MonoSample, MIDI_CHANNEL_RECEIVE_ALL},
    primitives::clock::Clock,
    traits::{IsMidiInstrument, SinksControl, SinksControlParam, SinksMidi, SourcesAudio},
};

use super::general_midi::GeneralMidiPercussionProgram;

#[derive(Debug, Default)]
struct Voice {
    samples: Vec<MonoSample>,
    sample_clock_start: usize,
    sample_pointer: usize,
    is_playing: bool,
}

impl Voice {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            samples: Vec::with_capacity(buffer_size),
            ..Default::default()
        }
    }

    pub fn new_from_file(filename: &str) -> Self {
        let mut reader = hound::WavReader::open(filename).unwrap();
        let mut r = Self::new(reader.duration() as usize);
        let i24_max: MonoSample = 2.0f32.powi(24 - 1);
        for sample in reader.samples::<i32>() {
            r.samples.push(sample.unwrap() as MonoSample / i24_max);
        }
        r
    }
}

impl SinksControl for Voice {
    fn handle_control(&mut self, _clock: &Clock, _param: &SinksControlParam) {
        todo!()
    }
}
impl SinksMidi for Voice {
    fn midi_channel(&self) -> crate::common::MidiChannel {
        MIDI_CHANNEL_RECEIVE_ALL
    }

    #[allow(unused_variables)]
    fn set_midi_channel(&mut self, midi_channel: MidiChannel) {}

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
impl SourcesAudio for Voice {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
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

#[derive(Debug, Default)]
pub struct Sampler {
    midi_channel: MidiChannel,
    note_to_voice: HashMap<u8, Voice>,
}
impl IsMidiInstrument for Sampler {}

impl Sampler {
    pub fn new(midi_channel: MidiChannel) -> Self {
        Self {
            midi_channel,
            note_to_voice: HashMap::new(),
        }
    }

    pub fn add_sample_for_note(&mut self, note: u8, filename: &str) -> anyhow::Result<()> {
        self.note_to_voice
            .insert(note, Voice::new_from_file(filename));
        Ok(())
    }

    pub fn new_from_files(midi_channel: MidiChannel) -> Self {
        let mut r = Self::new(midi_channel);
        let samples: [(GeneralMidiPercussionProgram, &str); 21] = [
            (GeneralMidiPercussionProgram::AcousticBassDrum, "BD A"),
            (GeneralMidiPercussionProgram::ElectricBassDrum, "BD B"),
            (GeneralMidiPercussionProgram::ClosedHiHat, "CH"),
            (GeneralMidiPercussionProgram::PedalHiHat, "CH"),
            (GeneralMidiPercussionProgram::HandClap, "Clap"),
            (GeneralMidiPercussionProgram::RideBell, "Cowbell"),
            (GeneralMidiPercussionProgram::CrashCymbal1, "Crash"),
            (GeneralMidiPercussionProgram::CrashCymbal2, "Crash"),
            (GeneralMidiPercussionProgram::OpenHiHat, "OH"),
            (GeneralMidiPercussionProgram::RideCymbal1, "Ride"),
            (GeneralMidiPercussionProgram::RideCymbal2, "Ride"),
            (GeneralMidiPercussionProgram::SideStick, "Rimshot"),
            (GeneralMidiPercussionProgram::AcousticSnare, "SD A"),
            (GeneralMidiPercussionProgram::ElectricSnare, "SD B"),
            (GeneralMidiPercussionProgram::Tambourine, "Tambourine"),
            (GeneralMidiPercussionProgram::LowTom, "Tom Lo"),
            (GeneralMidiPercussionProgram::LowMidTom, "Tom Lo"),
            (GeneralMidiPercussionProgram::HiMidTom, "Tom Mid"),
            (GeneralMidiPercussionProgram::HighTom, "Tom Hi"),
            (GeneralMidiPercussionProgram::HighAgogo, "Tom Hi"),
            (GeneralMidiPercussionProgram::LowAgogo, "Tom Lo"),
        ];
        for (program, filename) in samples {
            let result = r.add_sample_for_note(
                program as u8,
                format!("samples/707/{} 707.wav", filename).as_str(),
            );
            if result.is_err() {
                panic!("failed to load a sample: {}", filename);
            }
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
    fn midi_channel(&self) -> crate::common::MidiChannel {
        self.midi_channel
    }

    fn set_midi_channel(&mut self, midi_channel: MidiChannel) {
        self.midi_channel = midi_channel;
    }

    fn handle_midi_for_channel(&mut self, clock: &Clock, message: &MidiMessage) {
        match message.status {
            MidiMessageType::NoteOn => {
                let note: u8 = message.data1;
                if let Some(voice) = self.note_to_voice.get_mut(&note) {
                    voice.handle_midi_for_channel(clock, message);
                }
            }
            MidiMessageType::NoteOff => {
                let note: u8 = message.data1;
                if let Some(voice) = self.note_to_voice.get_mut(&note) {
                    voice.handle_midi_for_channel(clock, message);
                }
            }
            MidiMessageType::ProgramChange => {}
        }
    }
}

impl SourcesAudio for Sampler {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        // TODO: this looks a lot like a Mixer
        let mut sum = 0.0;
        for v in self.note_to_voice.values_mut() {
            sum += v.source_audio(clock);
        }
        sum
        // couldn't use this because map gives us a non-mut
        //
        // self.note_to_voice
        //     .values()
        //     .map(|v| v.get_audio_sample())
        //     .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loading() {
        let _ = Sampler::new_from_files(MIDI_CHANNEL_RECEIVE_ALL);
    }
}
