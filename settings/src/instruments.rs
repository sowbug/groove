// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::{patches::WelshPatchSettings, MidiChannel};
use crate::patches::FmSynthesizerSettings;
use groove_core::midi::note_description_to_frequency;
use groove_entities::instruments::{Drumkit, FmSynthesizer, Sampler};
use groove_orchestration::Entity;
use groove_toys::ToyInstrument;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstrumentSettings {
    #[serde(rename_all = "kebab-case")]
    Test {
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
        base_path: &PathBuf,
        load_only_test_entities: bool,
    ) -> (MidiChannel, Entity) {
        if load_only_test_entities {
            let midi_input_channel = match self {
                InstrumentSettings::Test { midi_input_channel } => *midi_input_channel,
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
                Entity::ToyInstrument(Box::new(ToyInstrument::new_with(sample_rate))),
            );
        }
        match self {
            InstrumentSettings::Test { midi_input_channel } => (
                *midi_input_channel,
                Entity::ToyInstrument(Box::new(ToyInstrument::new_with(sample_rate))),
            ),
            InstrumentSettings::Welsh {
                midi_input_channel,
                preset_name,
            } => (
                *midi_input_channel,
                Entity::WelshSynth(Box::new(
                    WelshPatchSettings::by_name(base_path, preset_name)
                        .into_welsh_synth(sample_rate),
                )),
            ),
            InstrumentSettings::Drumkit {
                midi_input_channel,
                preset_name: _preset,
            } => (
                *midi_input_channel,
                Entity::Drumkit(Box::new(Drumkit::new_from_files(
                    sample_rate,
                    PathBuf::from("samples/707"),
                ))),
            ),
            InstrumentSettings::Sampler {
                midi_input_channel,
                filename,
                root,
            } => {
                // TODO: where should this logic live?
                let root_frequency = note_description_to_frequency(root.to_string(), 0.0);
                let root_frequency = if root_frequency > 0.0 {
                    Some(root_frequency)
                } else {
                    None
                };
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
            } => (
                *midi_input_channel,
                Entity::FmSynthesizer(Box::new(FmSynthesizer::new_with_params(
                    sample_rate,
                    voice.into_params(),
                ))),
            ),
        }
    }
}
