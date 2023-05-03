// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::patches::WelshPatchSettings;
use super::MidiChannel;
use groove_core::Normal;
use groove_entities::{
    controllers::MidiChannelInputNano,
    instruments::{
        Drumkit, DrumkitNano, FmSynth, FmSynthNano, Sampler, SamplerNano, WelshSynth,
        WelshSynthNano,
    },
};
use groove_orchestration::Entity;
use groove_toys::{ToyInstrument, ToyInstrumentNano};
use groove_utils::Paths;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WelshPatchWrapper {
    name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstrumentSettings {
    #[serde(rename_all = "kebab-case")]
    ToyInstrument(MidiChannelInputNano, ToyInstrumentNano),
    #[serde(rename_all = "kebab-case")]
    Welsh(MidiChannelInputNano, WelshPatchWrapper),
    #[serde(rename_all = "kebab-case")]
    WelshRaw(MidiChannelInputNano, WelshSynthNano),
    #[serde(rename_all = "kebab-case")]
    Drumkit(MidiChannelInputNano, DrumkitNano),
    #[serde(rename_all = "kebab-case")]
    Sampler(MidiChannelInputNano, SamplerNano),
    #[serde(rename_all = "kebab-case")]
    FmSynthesizer(MidiChannelInputNano, FmSynthNano),
}

impl InstrumentSettings {
    pub(crate) fn instantiate(
        &self,
        paths: &Paths,
        load_only_test_entities: bool,
    ) -> (MidiChannel, Entity) {
        if load_only_test_entities {
            let midi_input_channel = match self {
                InstrumentSettings::ToyInstrument(midi, ..)
                | InstrumentSettings::Welsh(midi, ..)
                | InstrumentSettings::WelshRaw(midi, ..)
                | InstrumentSettings::Drumkit(midi, ..)
                | InstrumentSettings::Sampler(midi, ..)
                | InstrumentSettings::FmSynthesizer(midi, ..) => midi.midi_in,
            };
            return (
                midi_input_channel,
                Entity::ToyInstrument(Box::new(ToyInstrument::new_with(ToyInstrumentNano {
                    fake_value: Normal::from(0.23498239),
                }))),
            );
        }
        match self {
            InstrumentSettings::ToyInstrument(midi, params) => (
                midi.midi_in,
                Entity::ToyInstrument(Box::new(ToyInstrument::new_with(params.clone()))),
            ),
            InstrumentSettings::Welsh(midi, patch) => (
                midi.midi_in,
                Entity::WelshSynth(Box::new(WelshSynth::new_with(
                    WelshPatchSettings::by_name(paths, &patch.name).derive_welsh_synth_nano(),
                ))),
            ),
            InstrumentSettings::WelshRaw(midi, params) => (
                midi.midi_in,
                Entity::WelshSynth(Box::new(WelshSynth::new_with(params.clone()))),
            ),
            InstrumentSettings::Drumkit(midi, params) => (
                midi.midi_in,
                Entity::Drumkit(Box::new(Drumkit::new_with(paths, params.clone()))),
            ),
            InstrumentSettings::Sampler(midi, params) => (
                midi.midi_in,
                Entity::Sampler(Box::new(Sampler::new_with(paths, params.clone()))),
            ),
            InstrumentSettings::FmSynthesizer(midi, params) => (
                midi.midi_in,
                Entity::FmSynth(Box::new(FmSynth::new_with(params.clone()))),
            ),
        }
    }
}
