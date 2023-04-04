// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::{patches::WelshPatchSettings, MidiChannel};
use crate::patches::FmSynthesizerSettings;
use groove_core::{midi::note_description_to_frequency, Normal};
use groove_entities::instruments::{Drumkit, Sampler};
use groove_orchestration::Entity;
use groove_toys::{ToyInstrument, ToyInstrumentNano};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstrumentSettings {
    #[serde(rename_all = "kebab-case")]
    ToyInstrument {
        #[serde(rename = "midi-in")]
        midi_input_channel: MidiChannel,
    },
    #[serde(rename_all = "kebab-case")]
    Welsh {
        #[serde(rename = "midi-in")]
        midi_input_channel: MidiChannel,
        #[serde(rename = "preset")]
        preset_name: String,
    },
    #[serde(rename_all = "kebab-case")]
    Drumkit {
        #[serde(rename = "midi-in")]
        midi_input_channel: MidiChannel,
        #[serde(rename = "preset")]
        preset_name: String,
    },
    #[serde(rename_all = "kebab-case")]
    Sampler {
        #[serde(rename = "midi-in")]
        midi_input_channel: MidiChannel,
        filename: String,

        /// This can be either a floating-point frequency in Hz or a MIDI note number.
        #[serde(default)]
        root: String,
    },
    #[serde(rename_all = "kebab-case")]
    FmSynthesizer {
        #[serde(rename = "midi-in")]
        midi_input_channel: MidiChannel,
        voice: FmSynthesizerSettings,
    },
}

impl InstrumentSettings {
    pub(crate) fn instantiate(
        &self,
        sample_rate: usize,
        asset_path: &Path,
        load_only_test_entities: bool,
    ) -> (MidiChannel, Entity) {
        if load_only_test_entities {
            let midi_input_channel = match self {
                InstrumentSettings::ToyInstrument { midi_input_channel } => *midi_input_channel,
                InstrumentSettings::Welsh {
                    midi_input_channel, ..
                } => *midi_input_channel,
                InstrumentSettings::Drumkit {
                    midi_input_channel, ..
                } => *midi_input_channel,
                InstrumentSettings::FmSynthesizer {
                    midi_input_channel, ..
                } => *midi_input_channel,
                InstrumentSettings::Sampler {
                    midi_input_channel, ..
                } => *midi_input_channel,
            };
            return (
                midi_input_channel,
                Entity::ToyInstrument(Box::new(ToyInstrument::new_with(
                    sample_rate,
                    ToyInstrumentNano {
                        fake_value: Normal::from(0.23498239),
                    },
                ))),
            );
        }
        match self {
            InstrumentSettings::ToyInstrument { midi_input_channel } => (
                *midi_input_channel,
                Entity::ToyInstrument(Box::new(ToyInstrument::new_with(
                    sample_rate,
                    ToyInstrumentNano {
                        fake_value: Normal::from(0.23498239),
                    },
                ))),
            ),
            InstrumentSettings::Welsh {
                midi_input_channel,
                preset_name,
            } => (
                *midi_input_channel,
                Entity::WelshSynth(Box::new(
                    WelshPatchSettings::by_name(asset_path, preset_name)
                        .derive_welsh_synth(sample_rate),
                )),
            ),
            InstrumentSettings::Drumkit {
                midi_input_channel,
                preset_name: _preset,
            } => {
                // TODO: we're hardcoding samples/. Figure out a way to use the
                // system.
                let base_dir = asset_path.join("samples/elphnt.io/707");
                (
                    *midi_input_channel,
                    Entity::Drumkit(Box::new(Drumkit::new_from_files(sample_rate, base_dir))),
                )
            }
            InstrumentSettings::Sampler {
                midi_input_channel,
                filename,
                root,
            } => {
                // TODO: where should this logic live?
                let root_frequency = note_description_to_frequency(root);
                (
                    *midi_input_channel,
                    Entity::Sampler(Box::new(Sampler::new_with_filename(
                        sample_rate,
                        filename,
                        root_frequency,
                    ))),
                )
            }
            InstrumentSettings::FmSynthesizer {
                midi_input_channel,
                voice,
            } =>
            // (
            //     *midi_input_channel,
            //     Entity::FmSynth(Box::new(FmSynth::new_with_params(
            //         sample_rate,
            //         voice.derive_params(),
            //     ))),
            // ),
            {
                panic!()
            }
        }
    }
}
