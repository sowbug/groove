use super::{HandlesMidi, IsVoice, PlaysNotes, Synthesizer, VoicePerNoteStore};
use crate::{
    clock::Clock,
    common::{F32ControlValue, Sample, SampleType},
    messages::EntityMessage,
    midi::GeneralMidiPercussionProgram,
    traits::{Controllable, HasUid, IsInstrument, Response, SourcesAudio, Updateable},
    utils::Paths,
    StereoSample,
};
use groove_macros::{Control, Uid};
use midly::num::u7;
use std::str::FromStr;
use strum_macros::{Display, EnumString, FromRepr};

#[derive(Debug, Default)]
struct DrumkitSamplerVoice {
    samples: Vec<Sample>,
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
}

impl DrumkitSamplerVoice {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            samples: Vec::with_capacity(buffer_size),
            ..Default::default()
        }
    }

    pub fn new_from_file(filename: &str) -> Self {
        let mut reader = hound::WavReader::open(filename).unwrap();
        let mut r = Self::new(reader.duration() as usize);
        let i24_max: SampleType = 2.0f64.powi(24 - 1);
        for sample in reader.samples::<i32>() {
            r.samples
                .push(Sample::from(sample.unwrap() as SampleType / i24_max));
        }
        r
    }

    fn handle_pending_note_events(&mut self, clock: &Clock) {
        if self.note_on_is_pending {
            self.note_on_is_pending = false;
            self.sample_pointer = 0;
            self.sample_clock_start = clock.samples();
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
        self.handle_pending_note_events(clock);
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

        StereoSample::from(if self.is_playing {
            *self
                .samples
                .get(self.sample_pointer)
                .unwrap_or(&Sample::SILENCE)
        } else {
            Sample::SILENCE
        })
    }
}

#[derive(Control, Debug, Uid)]
pub struct DrumkitSampler {
    uid: usize,
    inner_synth: Synthesizer<DrumkitSamplerVoice>,
    kit_name: String,
}
impl IsInstrument for DrumkitSampler {}
impl SourcesAudio for DrumkitSampler {
    fn source_audio(&mut self, clock: &Clock) -> crate::StereoSample {
        self.inner_synth.source_audio(clock)
    }
}
impl Updateable for DrumkitSampler {
    type Message = EntityMessage;

    fn update(&mut self, _clock: &Clock, message: Self::Message) -> Response<Self::Message> {
        #[allow(unused_variables)]
        match message {
            Self::Message::Midi(channel, midi_message) => {
                self.inner_synth.handle_midi_message(&midi_message);
            }
            _ => todo!(),
        }
        Response::none()
    }
}

impl DrumkitSampler {
    pub(crate) fn new_from_files() -> Self {
        let mut voice_store = Box::new(VoicePerNoteStore::<DrumkitSamplerVoice>::default());

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
                    Box::new(DrumkitSamplerVoice::new_from_file(filename)),
                );
            } else {
                eprintln!("Unable to load sample {asset_name}.");
            }
        }
        Self::new_with(voice_store, "707")
    }

    pub fn kit_name(&self) -> &str {
        self.kit_name.as_ref()
    }

    fn new_with(voice_store: Box<VoicePerNoteStore<DrumkitSamplerVoice>>, kit_name: &str) -> Self {
        Self {
            uid: Default::default(),
            inner_synth: Synthesizer::<DrumkitSamplerVoice>::new_with(voice_store),
            kit_name: kit_name.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loading() {
        let _ = DrumkitSampler::new_from_files();
    }
}
