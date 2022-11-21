use crate::{
    clock::Clock,
    common::MonoSample,
    messages::GrooveMessage,
    midi::{GeneralMidiPercussionProgram, MidiMessage},
    traits::{HasUid, IsInstrument, Updateable, SourcesAudio},
};
use std::collections::HashMap;

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
impl Updateable for Voice {
    type Message = GrooveMessage; // TODO

    fn update(
        &mut self,
        clock: &Clock,
        message: Self::Message,
    ) -> crate::traits::EvenNewerCommand<Self::Message> {
        #[allow(unused_variables)]
        match message {
            GrooveMessage::Midi(_channel, message) => match message {
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
        crate::traits::EvenNewerCommand::none()
    }
}
impl SourcesAudio for Voice {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        if self.sample_clock_start > clock.samples() {
            // TODO: this stops the clock-moves-backward explosion.
            // Come up with a more robust way to handle the sample pointer.
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

#[derive(Debug, Default)]
pub struct Sampler {
    uid: usize,
    note_to_voice: HashMap<u8, Voice>,
    pub(crate) kit_name: String,
}
impl IsInstrument for Sampler {}
impl SourcesAudio for Sampler {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        self.note_to_voice
            .values_mut()
            .map(|v| v.source_audio(clock))
            .sum()
    }
}
impl Updateable for Sampler {
    type Message = GrooveMessage;

    fn update(
        &mut self,
        clock: &Clock,
        message: Self::Message,
    ) -> crate::traits::EvenNewerCommand<Self::Message> {
        #[allow(unused_variables)]
        match message {
            GrooveMessage::Midi(channel, midi_message) => match midi_message {
                MidiMessage::NoteOff { key, vel } => {
                    if let Some(voice) = self.note_to_voice.get_mut(&u8::from(key)) {
                        voice.update(clock, message);
                    }
                }
                MidiMessage::NoteOn { key, vel } => {
                    if let Some(voice) = self.note_to_voice.get_mut(&u8::from(key)) {
                        voice.update(clock, message);
                    }
                }
                MidiMessage::Aftertouch { key, vel } => {
                    if let Some(voice) = self.note_to_voice.get_mut(&u8::from(key)) {
                        voice.update(clock, message);
                    }
                }
                _ => todo!(),
            },
            _ => todo!(),
        }
        crate::traits::EvenNewerCommand::none()
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
    fn new() -> Self {
        Default::default()
    }

    pub fn add_sample_for_note(&mut self, note: u8, filename: &str) -> anyhow::Result<()> {
        self.note_to_voice
            .insert(note, Voice::new_from_file(filename));
        Ok(())
    }

    pub(crate) fn new_from_files() -> Self {
        let mut r = Self::new();
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
                format!("samples/707/{filename} 707.wav").as_str(),
            );
            if result.is_err() {
                panic!("failed to load a sample: {filename}");
            }
        }
        r.kit_name = "707".to_string();
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loading() {
        let _ = Sampler::new_from_files();
    }
}
