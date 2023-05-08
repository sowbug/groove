mod drumkit {
    use super::{sampler::SamplerVoice, Sampler};
    use anyhow::anyhow;
    use groove_core::{
        instruments::Synthesizer,
        midi::{
            note_to_frequency, u7, GeneralMidiPercussionProgram, HandlesMidi,
            MidiChannel, MidiMessage,
        },
        traits::{Generates, IsInstrument, Resets, Ticks},
        voices::VoicePerNoteStore, StereoSample,
    };
    use groove_proc_macros::{Control, Params, Uid};
    use groove_utils::Paths;
    use std::{path::Path, sync::Arc};
    pub struct Drumkit {
        #[params]
        name: String,
        uid: usize,
        sample_rate: usize,
        paths: Paths,
        inner_synth: Synthesizer<SamplerVoice>,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Drumkit {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field5_finish(
                f,
                "Drumkit",
                "name",
                &self.name,
                "uid",
                &self.uid,
                "sample_rate",
                &self.sample_rate,
                "paths",
                &self.paths,
                "inner_synth",
                &&self.inner_synth,
            )
        }
    }
    #[automatically_derived]
    impl Drumkit {
        pub const STRUCT_SIZE: usize = 0;
    }
    #[automatically_derived]
    impl groove_core::traits::Controllable for Drumkit {
        fn control_index_count(&self) -> usize {
            Self::STRUCT_SIZE
        }
        fn control_set_param_by_name(
            &mut self,
            name: &str,
            value: groove_core::control::F32ControlValue,
        ) {
            if let Some(index) = self.control_index_for_name(name) {
                self.control_set_param_by_index(index, value);
            } else {
                {
                    ::std::io::_eprint(
                        format_args!(
                            "Warning: couldn\'t set param named \'{0}\'\n", name
                        ),
                    );
                };
            }
        }
        fn control_name_for_index(&self, index: usize) -> Option<String> {
            match index {
                _ => None,
            }
        }
        fn control_index_for_name(&self, name: &str) -> Option<usize> {
            match name {
                _ => None,
            }
        }
        fn control_set_param_by_index(
            &mut self,
            index: usize,
            value: groove_core::control::F32ControlValue,
        ) {
            match index {
                _ => {}
            }
        }
    }
    #[automatically_derived]
    pub struct DrumkitParams {
        pub name: String,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for DrumkitParams {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field1_finish(
                f,
                "DrumkitParams",
                "name",
                &&self.name,
            )
        }
    }
    #[automatically_derived]
    impl ::core::default::Default for DrumkitParams {
        #[inline]
        fn default() -> DrumkitParams {
            DrumkitParams {
                name: ::core::default::Default::default(),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for DrumkitParams {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for DrumkitParams {
        #[inline]
        fn eq(&self, other: &DrumkitParams) -> bool {
            self.name == other.name
        }
    }
    #[automatically_derived]
    impl DrumkitParams {
        pub fn name(&self) -> String {
            self.name
        }
        pub fn set_name(&mut self, name: &str) {
            self.name = name.to_string();
        }
    }
    #[automatically_derived]
    impl groove_core::traits::HasUid for Drumkit {
        fn uid(&self) -> usize {
            self.uid
        }
        fn set_uid(&mut self, uid: usize) {
            self.uid = uid;
        }
        fn name(&self) -> &'static str {
            "Drumkit"
        }
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
        fn new_from_files(paths: &Paths, kit_name: &str) -> Self {
            let samples = <[_]>::into_vec(
                #[rustc_box]
                ::alloc::boxed::Box::new([
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
                ]),
            );
            let sample_dirs = <[_]>::into_vec(
                #[rustc_box]
                ::alloc::boxed::Box::new(["elphnt.io", "707"]),
            );
            let voice_store = VoicePerNoteStore::<
                SamplerVoice,
            >::new_with_voices(
                samples
                    .into_iter()
                    .flat_map(|(program, asset_name)| {
                        let filename = paths
                            .build_sample(
                                &sample_dirs,
                                Path::new(
                                    &{
                                        let res = ::alloc::fmt::format(
                                            format_args!("{0}.wav", asset_name),
                                        );
                                        res
                                    },
                                ),
                            );
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
                                Err(
                                    ::anyhow::Error::msg({
                                        let res = ::alloc::fmt::format(
                                            format_args!(
                                                "Unable to load sample from file {0:?}.", filename
                                            ),
                                        );
                                        res
                                    }),
                                )
                            }
                        } else {
                            Err(
                                ::anyhow::Error::msg({
                                    let res = ::alloc::fmt::format(
                                        format_args!(
                                            "Couldn\'t find filename {0:?} in hives", filename
                                        ),
                                    );
                                    res
                                }),
                            )
                        }
                    }),
            );
            Self {
                uid: Default::default(),
                sample_rate: Default::default(),
                inner_synth: Synthesizer::<
                    SamplerVoice,
                >::new_with(Box::new(voice_store)),
                paths: paths.clone(),
                name: kit_name.to_string(),
            }
        }
        pub fn new_with(paths: &Paths, params: DrumkitParams) -> Self {
            Self::new_from_files(paths, params.name.as_ref())
        }
        pub fn sample_rate(&self) -> usize {
            self.sample_rate
        }
        pub fn name(&self) -> &str {
            self.name.as_ref()
        }
        pub fn set_name(&mut self, name: &str) {
            self.name = name.to_string();
        }
    }
}
