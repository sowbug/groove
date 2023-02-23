use super::{
    IsStereoSampleVoice, IsVoice, PlaysNotes, PlaysNotesEventTracker, Synthesizer,
    VoicePerNoteStore,
};
use crate::{
    common::F32ControlValue,
    midi::{GeneralMidiPercussionProgram, MidiChannel},
    traits::{Controllable, Generates, HandlesMidi, HasUid, IsInstrument, Resets, Ticks},
    utils::Paths,
    Sampler, StereoSample,
};
use groove_macros::{Control, Uid};
use midly::{num::u7, MidiMessage};
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
    event_tracker: PlaysNotesEventTracker,
}
impl IsStereoSampleVoice for DrumkitSamplerVoice {}
impl IsVoice<StereoSample> for DrumkitSamplerVoice {}
impl PlaysNotes for DrumkitSamplerVoice {
    fn is_playing(&self) -> bool {
        self.is_playing
    }

    fn has_pending_events(&self) -> bool {
        self.event_tracker.has_pending_events()
    }

    fn enqueue_note_on(&mut self, key: u8, velocity: u8) {
        // This instrument doesn't care about key because each Voice always
        // plays the same note, but it's more consistent to pass it in to
        // PlaysNotesEventTracker.
        if self.is_active() {
            // TODO: it's unclear whether this needs to be implemented. There could
            // definitely be a transient if a note interrupts its own playback.
            // Let's revisit. For now, let's just respect the contract and turn the
            // steal into a note-on.
            self.event_tracker.enqueue_steal(key, velocity);
        } else {
            self.event_tracker.enqueue_note_on(key, velocity);
        }
    }

    fn enqueue_aftertouch(&mut self, velocity: u8) {
        self.event_tracker.enqueue_aftertouch(velocity);
    }

    fn enqueue_note_off(&mut self, velocity: u8) {
        self.event_tracker.enqueue_note_off(velocity);
    }

    fn set_pan(&mut self, _value: f32) {
        // We don't do stereo....  yet
    }
}

impl DrumkitSamplerVoice {
    pub fn new_with(sample_rate: usize) -> Self {
        Self {
            sample_rate,
            sample: Default::default(),
            ticks: Default::default(),
            samples: Default::default(),
            sample_clock_start: Default::default(),
            sample_pointer: Default::default(),
            is_playing: Default::default(),
            event_tracker: Default::default(),
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
        if self.event_tracker.note_off_is_pending {
            self.is_playing = false;
        }
        if self.event_tracker.note_on_is_pending {
            self.sample_pointer = 0;
            self.sample_clock_start = ticks;
            self.is_playing = true;
        }
        if self.event_tracker.aftertouch_is_pending {
            // TODO: do something
        }
        self.event_tracker.clear_pending();
    }
}
impl Generates<StereoSample> for DrumkitSamplerVoice {
    fn value(&self) -> StereoSample {
        self.sample
    }

    #[allow(unused_variables)]
    fn batch_values(&mut self, values: &mut [StereoSample]) {
        todo!()
    }
}
impl Resets for DrumkitSamplerVoice {
    fn reset(&mut self, sample_rate: usize) {
        self.sample_rate = sample_rate;
        self.ticks = 0;
    }
}
impl Ticks for DrumkitSamplerVoice {
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
impl Generates<StereoSample> for DrumkitSampler {
    fn value(&self) -> StereoSample {
        self.inner_synth.value()
    }

    fn batch_values(&mut self, values: &mut [StereoSample]) {
        self.inner_synth.batch_values(values);
    }
}
impl Resets for DrumkitSampler {
    fn reset(&mut self, sample_rate: usize) {
        self.inner_synth.reset(sample_rate);
    }
}
impl Ticks for DrumkitSampler {
    fn tick(&mut self, tick_count: usize) {
        self.inner_synth.tick(tick_count);
    }
}
impl HandlesMidi for DrumkitSampler {
    fn handle_midi_message(
        &mut self,
        message: &midly::MidiMessage,
    ) -> Option<Vec<(MidiChannel, MidiMessage)>> {
        self.inner_synth.handle_midi_message(message)
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
    use crate::{common::DEFAULT_SAMPLE_RATE, instruments::tests::is_voice_makes_any_sound_at_all};

    use super::*;

    #[test]
    fn test_loading() {
        let _ = DrumkitSampler::new_from_files(DEFAULT_SAMPLE_RATE);
    }

    #[test]
    fn drumkit_sampler_makes_any_sound_at_all() {
        let mut voice = DrumkitSamplerVoice::new_from_file(
            DEFAULT_SAMPLE_RATE,
            "test-data/square-440Hz-1-second-mono-24-bit-PCM.wav",
        );
        voice.enqueue_note_on(1, 127);

        assert!(
            is_voice_makes_any_sound_at_all(&mut voice),
            "once triggered, DrumkitSampler voice should make a sound"
        );
    }
}
