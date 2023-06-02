// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::{sampler::SamplerVoice, Sampler};
use anyhow::anyhow;
use groove_core::{
    instruments::Synthesizer,
    midi::{
        note_to_frequency, u7, GeneralMidiPercussionProgram, HandlesMidi, MidiChannel, MidiMessage,
    },
    time::SampleRate,
    traits::{Configurable, Generates, IsInstrument, Ticks},
    voices::VoicePerNoteStore,
    StereoSample,
};
use groove_proc_macros::{Control, Params, Uid};
use groove_utils::Paths;
use std::{path::Path, sync::Arc};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Control, Params, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Drumkit {
    #[params]
    name: String,

    uid: usize,
    #[cfg_attr(feature = "serialization", serde(skip))]
    sample_rate: SampleRate,
    #[cfg_attr(feature = "serialization", serde(skip))]
    paths: Paths,
    #[cfg_attr(feature = "serialization", serde(skip))]
    inner_synth: Synthesizer<SamplerVoice>,
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
impl Configurable for Drumkit {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.inner_synth.update_sample_rate(sample_rate);
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
    fn new_from_files(paths: &Paths, kit_name: &str) -> Self {
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

        let sample_dirs = vec!["elphnt.io", "707"];

        let voice_store = VoicePerNoteStore::<SamplerVoice>::new_with_voices(
            samples.into_iter().flat_map(|(program, asset_name)| {
                let filename =
                    paths.build_sample(&sample_dirs, Path::new(&format!("{asset_name}.wav")));
                if let Ok(file) = paths.search_and_open(filename.as_path()) {
                    if let Ok(samples) = Sampler::read_samples_from_file(&file) {
                        let program = program as u8;
                        Ok((
                            u7::from(program),
                            SamplerVoice::new_with_samples(
                                Arc::new(samples),
                                note_to_frequency(program),
                            ),
                        ))
                    } else {
                        Err(anyhow!("Unable to load sample from file {:?}.", filename))
                    }
                } else {
                    Err(anyhow!("Couldn't find filename {:?} in hives", filename))
                }
            }),
        );

        Self {
            uid: Default::default(),
            sample_rate: Default::default(),
            inner_synth: Synthesizer::<SamplerVoice>::new_with(Box::new(voice_store)),
            paths: paths.clone(),
            name: kit_name.to_string(),
        }
    }

    pub fn new_with(params: &DrumkitParams, paths: &Paths) -> Self {
        // TODO: we're hardcoding samples/. Figure out a way to use the
        // system.
        Self::new_from_files(paths, params.name.as_ref())
    }

    #[cfg(feature = "iced-framework")]
    pub fn update(&mut self, message: DrumkitMessage) {
        match message {
            DrumkitMessage::Drumkit(s) => *self = Self::new_with(&self.paths, s),
            _ => self.derived_update(message),
        }
    }

    pub fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }
}

#[cfg(feature = "egui-framework")]
mod gui {
    use super::Drumkit;
    use eframe::egui::Ui;
    use groove_core::traits::gui::Shows;

    impl Shows for Drumkit {
        fn show(&mut self, ui: &mut Ui) {
            ui.label(self.name());
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
//     // fn loading() {
//     //     let _ = Drumkit::new_from_files(DEFAULT_SAMPLE_RATE,         PathBuf::from("test-data/;
//     // }
// }
