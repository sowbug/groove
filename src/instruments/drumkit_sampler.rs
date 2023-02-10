use super::{GeneratesSamples, HandlesMidi, IsVoice, PlaysNotes, Synthesizer, VoicePerNoteStore};
use crate::{
    clock::Clock,
    common::F32ControlValue,
    midi::GeneralMidiPercussionProgram,
    traits::{Controllable, HasUid, IsInstrument, SourcesAudio, Ticks},
    utils::Paths,
    Sampler, StereoSample,
};
use groove_macros::{Control, Uid};
use midly::num::u7;
use std::str::FromStr;
use strum_macros::{Display, EnumString, FromRepr};

#[derive(Debug)]
struct DrumkitSamplerVoice {
    sample_rate: usize,
    sample: StereoSample,
    ticks: usize,

    samples: Vec<StereoSample>,
    sample_clock_start: usize,
    sample_pointer: usize,

    is_playing: bool,
    note_on_is_pending: bool,
    note_on_velocity: u8,
    note_off_is_pending: bool,
    note_off_velocity: u8,
    aftertouch_is_pending: bool,
    aftertouch_velocity: u8,
}
impl IsVoice for DrumkitSamplerVoice {}
impl PlaysNotes for DrumkitSamplerVoice {
    fn is_playing(&self) -> bool {
        self.is_playing
    }

    fn are_events_pending(&self) -> bool {
        self.note_on_is_pending || self.aftertouch_is_pending || self.note_off_is_pending
    }

    fn set_frequency_hz(&mut self, _frequency_hz: f32) {
        // not applicable for this kind of sampler
    }

    fn enqueue_note_on(&mut self, velocity: u8) {
        self.note_on_is_pending = true;
        self.note_on_velocity = velocity;
    }

    fn enqueue_aftertouch(&mut self, velocity: u8) {
        self.aftertouch_is_pending = true;
        self.aftertouch_velocity = velocity;
    }

    fn enqueue_note_off(&mut self, velocity: u8) {
        self.note_off_is_pending = true;
        self.note_off_velocity = velocity;
    }

    fn set_pan(&mut self, _value: f32) {
        // We don't do stereo....  yet
    }
}

impl DrumkitSamplerVoice {
    pub fn new_with(sample_rate: usize) -> Self {
        Self {
            sample_rate: sample_rate,
            sample: Default::default(),
            ticks: Default::default(),
            samples: Default::default(),
            sample_clock_start: Default::default(),
            sample_pointer: Default::default(),
            is_playing: Default::default(),
            note_on_is_pending: Default::default(),
            note_on_velocity: Default::default(),
            note_off_is_pending: Default::default(),
            note_off_velocity: Default::default(),
            aftertouch_is_pending: Default::default(),
            aftertouch_velocity: Default::default(),
        }
    }
    pub fn new_from_file(sample_rate: usize, filename: &str) -> Self {
        let mut r = Self::new_with(sample_rate);
        r.samples = Sampler::read_samples_from_file(filename);
        // TODO we're sorta kinda ignoring that edge case where the sample's
        // sample rate doesn't match the current sample rate.............
        r
    }

    // TODO get rid of ticks arg when source_audio() is gone
    fn handle_pending_note_events(&mut self, ticks: usize) {
        if self.note_on_is_pending {
            self.note_on_is_pending = false;
            self.sample_pointer = 0;
            self.sample_clock_start = ticks;
            self.is_playing = true;
        }
        if self.aftertouch_is_pending {
            self.aftertouch_is_pending = false;
            // TODO: do something
        }
        if self.note_off_is_pending {
            self.note_off_is_pending = false;
            self.is_playing = false;
        }
    }
}
impl SourcesAudio for DrumkitSamplerVoice {
    fn source_audio(&mut self, clock: &Clock) -> crate::StereoSample {
        self.handle_pending_note_events(clock.samples());
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
            *self
                .samples
                .get(self.sample_pointer)
                .unwrap_or(&StereoSample::SILENCE)
        } else {
            StereoSample::SILENCE
        }
    }
}
impl GeneratesSamples for DrumkitSamplerVoice {
    fn sample(&self) -> StereoSample {
        self.sample
    }

    fn batch_sample(&mut self, samples: &mut [StereoSample]) {
        todo!()
    }
}
impl Ticks for DrumkitSamplerVoice {
    fn reset(&mut self, sample_rate: usize) {
        self.sample_rate = sample_rate;
        self.ticks = 0;
    }

    fn tick(&mut self, tick_count: usize) {
        for _ in 0..tick_count {
            self.ticks += 1;
            self.handle_pending_note_events(self.ticks);
            if self.sample_clock_start > self.ticks {
                // TODO: this stops the clock-moves-backward explosion.
                // Come up with a more robust way to handle the sample pointer.
                self.is_playing = false;
                self.sample_pointer = 0;
            } else {
                self.sample_pointer = self.ticks - self.sample_clock_start;
                if self.sample_pointer >= self.samples.len() {
                    self.is_playing = false;
                    self.sample_pointer = 0;
                }
            }

            self.sample = if self.is_playing {
                *self
                    .samples
                    .get(self.sample_pointer)
                    .unwrap_or(&StereoSample::SILENCE)
            } else {
                StereoSample::SILENCE
            };
        }
    }
}

#[derive(Control, Debug, Uid)]
pub struct DrumkitSampler {
    uid: usize,
    inner_synth: Synthesizer<DrumkitSamplerVoice>,
    kit_name: String,
}
impl IsInstrument for DrumkitSampler {}
impl HandlesMidi for DrumkitSampler {
    fn handle_midi_message(&mut self, message: &midly::MidiMessage) {
        self.inner_synth.handle_midi_message(message);
    }
}
impl SourcesAudio for DrumkitSampler {
    fn source_audio(&mut self, clock: &Clock) -> crate::StereoSample {
        self.inner_synth.source_audio(clock)
    }
}

impl DrumkitSampler {
    pub(crate) fn new_from_files(sample_rate: usize) -> Self {
        let mut voice_store = Box::new(VoicePerNoteStore::<DrumkitSamplerVoice>::new_with(
            sample_rate,
        ));

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
        let mut base_dir = Paths::asset_path();
        base_dir.push("samples");
        base_dir.push("707");

        for (program, asset_name) in samples {
            let mut path = base_dir.clone();
            path.push(format!("{asset_name} 707.wav").as_str());

            if let Some(filename) = path.to_str() {
                voice_store.add_voice(
                    u7::from(program as u8),
                    Box::new(DrumkitSamplerVoice::new_from_file(sample_rate, filename)),
                );
            } else {
                eprintln!("Unable to load sample {asset_name}.");
            }
        }
        Self::new_with(sample_rate, voice_store, "707")
    }

    pub fn kit_name(&self) -> &str {
        self.kit_name.as_ref()
    }

    fn new_with(
        sample_rate: usize,
        voice_store: Box<VoicePerNoteStore<DrumkitSamplerVoice>>,
        kit_name: &str,
    ) -> Self {
        Self {
            uid: Default::default(),
            inner_synth: Synthesizer::<DrumkitSamplerVoice>::new_with(sample_rate, voice_store),
            kit_name: kit_name.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loading() {
        let _ = DrumkitSampler::new_from_files(Clock::DEFAULT_SAMPLE_RATE);
    }
}
