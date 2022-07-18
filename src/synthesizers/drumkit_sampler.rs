use std::collections::HashMap;

use hound;

use crate::{
    common::{MidiMessageType},
    devices::traits::DeviceTrait,
    general_midi::GeneralMidiPercussionProgram,
};

#[derive(Default)]
struct Voice {
    samples: Vec<f32>,
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
        let i24_max: f32 = 2.0f32.powi(24 - 1);
        for sample in reader.samples::<i32>() {
            r.samples.push(sample.unwrap() as f32 / i24_max);
        }
        r
    }
}

impl DeviceTrait for Voice {
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

    fn get_audio_sample(&self) -> f32 {
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

#[derive(Default)]
pub struct Sampler {
    note_to_voice: HashMap<u8, Voice>,
}

impl Sampler {
    pub fn new() -> Self {
        Self {
            note_to_voice: HashMap::new(),
            ..Default::default()
        }
    }

    pub fn add_sample_for_note(&mut self, note: u8, filename: &str) -> anyhow::Result<()> {
        self.note_to_voice
            .insert(note, Voice::new_from_file(filename));
        Ok(())
    }

    pub fn new_from_files() -> Self {
        let mut r = Self::new();
        let samples: [(GeneralMidiPercussionProgram, &str); 8] = [
            (GeneralMidiPercussionProgram::AcousticBassDrum, "BD A 707"),
            (GeneralMidiPercussionProgram::ElectricBassDrum, "BD B 707"),
            (GeneralMidiPercussionProgram::AcousticSnare, "SD A 707"),
            (GeneralMidiPercussionProgram::ElectricSnare, "SD B 707"),
            (GeneralMidiPercussionProgram::PedalHiHat, "CH 707"),
            (GeneralMidiPercussionProgram::OpenHiHat, "OH 707"),
            (GeneralMidiPercussionProgram::CrashCymbal2, "Crash 707"),
            (GeneralMidiPercussionProgram::HighAgogo, "Tom Hi 707"),
        ];
        for (program, filename) in samples {
            r.add_sample_for_note(program as u8, format!("samples/707/{}.wav", filename).as_str());
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
                let note: u8 = message.data1;
                let voice = self.note_to_voice.get_mut(&note);
                if voice.is_some() {
                    voice.unwrap().handle_midi_message(message, clock);
                }
            }
            MidiMessageType::NoteOff => {
                let note: u8 = message.data1;
                let voice = self.note_to_voice.get_mut(&note);
                if voice.is_some() {
                    voice.unwrap().handle_midi_message(message, clock);
                }
            }
            MidiMessageType::ProgramChange => {}
        }
    }

    fn get_audio_sample(&self) -> f32 {
        self.note_to_voice
            .values()
            .map(|v| v.get_audio_sample())
            .sum()
    }

    fn tick(&mut self, clock: &crate::primitives::clock::Clock) -> bool {
        for voice in self.note_to_voice.values_mut() {
            voice.tick(clock);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loading() {
        let synth = Sampler::new_from_files();
    }
}
