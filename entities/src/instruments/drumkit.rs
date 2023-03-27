// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::{sampler::SamplerVoice, Sampler};
use anyhow::anyhow;
use groove_core::{
    instruments::Synthesizer,
    midi::{
        note_to_frequency, u7, GeneralMidiPercussionProgram, HandlesMidi, MidiChannel, MidiMessage,
    },
    traits::{Generates, IsInstrument, Resets, Ticks},
    voices::VoicePerNoteStore,
    StereoSample,
};
use groove_macros::{Control, Synchronization, Uid};
use std::{path::PathBuf, str::FromStr, sync::Arc};
use strum::EnumCount;
use strum_macros::{
    Display, EnumCount as EnumCountMacro, EnumIter, EnumString, FromRepr, IntoStaticStr,
};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

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
    pub fn new_from_files(sample_rate: usize, base_dir: PathBuf) -> Self {
        let samples = vec![
            (GeneralMidiPercussionProgram::AcousticBassDrum, "Kick 1 R1"),
            (GeneralMidiPercussionProgram::ElectricBassDrum, "Kick 2 R1"),
            (GeneralMidiPercussionProgram::ClosedHiHat, "Hat Closed R1"),
            (GeneralMidiPercussionProgram::PedalHiHat, "Hat Closed R2"),
            (GeneralMidiPercussionProgram::HandClap, "Clap R1"),
            (GeneralMidiPercussionProgram::RideBell, "Cowbell R1"),
            (GeneralMidiPercussionProgram::CrashCymbal1, "Crash R1"),
            (GeneralMidiPercussionProgram::CrashCymbal2, "Crash R2"),
            (GeneralMidiPercussionProgram::OpenHiHat, "Hat Open R1"),
            (GeneralMidiPercussionProgram::RideCymbal1, "Ride R1"),
            (GeneralMidiPercussionProgram::RideCymbal2, "Ride R2"),
            (GeneralMidiPercussionProgram::SideStick, "Rim R1"),
            (GeneralMidiPercussionProgram::AcousticSnare, "Snare 1 R1"),
            (GeneralMidiPercussionProgram::ElectricSnare, "Snare 2 R1"),
            (GeneralMidiPercussionProgram::Tambourine, "Tambourine R1"),
            (GeneralMidiPercussionProgram::LowTom, "Tom 1 R1"),
            (GeneralMidiPercussionProgram::LowMidTom, "Tom 1 R2"),
            (GeneralMidiPercussionProgram::HiMidTom, "Tom 2 R1"),
            (GeneralMidiPercussionProgram::HighTom, "Tom 3 R1"),
            (GeneralMidiPercussionProgram::HighAgogo, "Cowbell R3"),
            (GeneralMidiPercussionProgram::LowAgogo, "Cowbell R4"),
        ];

        Self::new_with(
            sample_rate,
            Box::new(VoicePerNoteStore::<SamplerVoice>::new_with_voices(
                sample_rate,
                samples.into_iter().flat_map(|(program, asset_name)| {
                    let mut path = base_dir.clone();
                    path.push(format!("{asset_name}.wav").as_str());

                    if let Some(filename) = path.to_str() {
                        if let Ok(samples) = Sampler::read_samples_from_file(filename) {
                            let program = program as u8;
                            Ok((
                                u7::from(program),
                                SamplerVoice::new_with_samples(
                                    sample_rate,
                                    Arc::new(samples),
                                    note_to_frequency(program),
                                ),
                            ))
                        } else {
                            Err(anyhow!("Unable to load sample from file {filename}."))
                        }
                    } else {
                        Err(anyhow!("Unable to load sample {asset_name}."))
                    }
                }),
            )),
            "707",
        )
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

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::tests::DEFAULT_SAMPLE_RATE;

// // TODO: this is a bad test. It uses production data. When we have a more
// flexible way to load drumkits, switch over to that.
//
//     // #[test]
//     // fn test_loading() {
//     //     let _ = Drumkit::new_from_files(DEFAULT_SAMPLE_RATE,         PathBuf::from("test-data/;
//     // }
// }
