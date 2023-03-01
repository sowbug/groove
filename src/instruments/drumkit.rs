use super::{
    sampler::{Sampler, SamplerVoice},
    Synthesizer, VoicePerNoteStore,
};
use crate::{midi::GeneralMidiPercussionProgram, utils::Paths};
use groove_core::{
    control::F32ControlValue,
    midi::{note_to_frequency, u7, HandlesMidi, MidiChannel, MidiMessage},
    traits::{Controllable, Generates, HasUid, IsInstrument, Resets, Ticks},
    StereoSample,
};
use groove_macros::{Control, Uid};
use std::{str::FromStr, sync::Arc};
use strum_macros::{Display, EnumString, FromRepr};

#[derive(Control, Debug, Uid)]
pub struct Drumkit {
    uid: usize,
    inner_synth: Synthesizer<SamplerVoice>,
    kit_name: String,
}
impl IsInstrument for Drumkit {}
impl Generates<StereoSample> for Drumkit {
    fn value(&self) -> StereoSample {
        self.inner_synth.value()
    }

    fn batch_values(&mut self, values: &mut [StereoSample]) {
        self.inner_synth.batch_values(values);
    }
}
impl Resets for Drumkit {
    fn reset(&mut self, sample_rate: usize) {
        self.inner_synth.reset(sample_rate);
    }
}
impl Ticks for Drumkit {
    fn tick(&mut self, tick_count: usize) {
        self.inner_synth.tick(tick_count);
    }
}
impl HandlesMidi for Drumkit {
    fn handle_midi_message(
        &mut self,
        message: &MidiMessage,
    ) -> Option<Vec<(MidiChannel, MidiMessage)>> {
        self.inner_synth.handle_midi_message(message)
    }
}

impl Drumkit {
    pub(crate) fn new_from_files(sample_rate: usize) -> Self {
        let mut voice_store = Box::new(VoicePerNoteStore::<SamplerVoice>::new_with(sample_rate));

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
                if let Ok(samples) = Sampler::read_samples_from_file(filename) {
                    let program = program as u8;
                    voice_store.add_voice(
                        u7::from(program),
                        Box::new(SamplerVoice::new_with_samples(
                            sample_rate,
                            Arc::new(samples),
                            note_to_frequency(program),
                        )),
                    );
                } else {
                    eprintln!("Unable to load sample from file {filename}.");
                }
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
        voice_store: Box<VoicePerNoteStore<SamplerVoice>>,
        kit_name: &str,
    ) -> Self {
        Self {
            uid: Default::default(),
            inner_synth: Synthesizer::<SamplerVoice>::new_with(sample_rate, voice_store),
            kit_name: kit_name.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::DEFAULT_SAMPLE_RATE;

    #[test]
    fn test_loading() {
        let _ = Drumkit::new_from_files(DEFAULT_SAMPLE_RATE);
    }
}
