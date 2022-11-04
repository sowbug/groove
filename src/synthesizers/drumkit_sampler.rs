use crate::{
    clock::Clock,
    common::{rrc, rrc_downgrade, MonoSample, Rrc, Ww, MONO_SAMPLE_SILENCE},
    midi::{GeneralMidiPercussionProgram, MidiChannel, MidiMessage, MIDI_CHANNEL_RECEIVE_ALL},
    traits::{HasOverhead, IsMidiInstrument, Overhead, SinksMidi, SourcesAudio},
};
use std::collections::HashMap;

#[derive(Debug, Default)]
struct Voice {
    overhead: Overhead,

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

impl SinksMidi for Voice {
    fn midi_channel(&self) -> MidiChannel {
        MIDI_CHANNEL_RECEIVE_ALL
    }

    #[allow(unused_variables)]
    fn set_midi_channel(&mut self, midi_channel: MidiChannel) {}

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
impl HasOverhead for Voice {
    fn overhead(&self) -> &Overhead {
        &self.overhead
    }

    fn overhead_mut(&mut self) -> &mut Overhead {
        &mut self.overhead
    }
}

#[derive(Debug, Default)]
pub struct Sampler {
    pub(crate) me: Ww<Self>,
    overhead: Overhead,

    midi_channel: MidiChannel,
    note_to_voice: HashMap<u8, Voice>,

    pub(crate) kit_name: String,
}
impl IsMidiInstrument for Sampler {}

impl Sampler {
    fn new(midi_channel: MidiChannel) -> Self {
        Self {
            midi_channel,
            ..Default::default()
        }
    }

    pub fn add_sample_for_note(&mut self, note: u8, filename: &str) -> anyhow::Result<()> {
        self.note_to_voice
            .insert(note, Voice::new_from_file(filename));
        Ok(())
    }

    fn new_from_files(midi_channel: MidiChannel) -> Self {
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
                format!("samples/707/{filename} 707.wav").as_str(),
            );
            if result.is_err() {
                panic!("failed to load a sample: {filename}");
            }
        }
        r.kit_name = "707".to_string();
        r
    }

    pub fn new_wrapped_from_files(midi_channel: MidiChannel) -> Rrc<Self> {
        let wrapped = rrc(Self::new_from_files(midi_channel));
        wrapped.borrow_mut().me = rrc_downgrade(&wrapped);
        wrapped
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
        channel: &MidiChannel,
        message: &MidiMessage,
    ) {
        #[allow(unused_variables)]
        match message {
            MidiMessage::NoteOff { key, vel } => {
                if let Some(voice) = self.note_to_voice.get_mut(&u8::from(*key)) {
                    voice.handle_midi_for_channel(clock, channel, message);
                }
            }
            MidiMessage::NoteOn { key, vel } => {
                if let Some(voice) = self.note_to_voice.get_mut(&u8::from(*key)) {
                    voice.handle_midi_for_channel(clock, channel, message);
                }
            }
            MidiMessage::Aftertouch { key, vel } => todo!(),
            MidiMessage::Controller { controller, value } => todo!(),
            MidiMessage::ProgramChange { program } => todo!(),
            MidiMessage::ChannelAftertouch { vel } => todo!(),
            MidiMessage::PitchBend { bend } => todo!(),
        }
    }
}

impl SourcesAudio for Sampler {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        if !self.overhead().is_enabled() || self.overhead().is_muted() {
            return MONO_SAMPLE_SILENCE;
        }

        self.note_to_voice
            .values_mut()
            .map(|v| v.source_audio(clock))
            .sum()
    }
}
impl HasOverhead for Sampler {
    fn overhead(&self) -> &Overhead {
        &self.overhead
    }

    fn overhead_mut(&mut self) -> &mut Overhead {
        &mut self.overhead
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
