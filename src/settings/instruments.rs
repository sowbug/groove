use super::{patches::SynthPatch, MidiChannel};
use crate::{
    entities::BoxedEntity,
    instruments::{
        drumkit_sampler,
        welsh::{self},
        FmSynthesizer, SimpleSynthesizer,
    },
    traits::TestInstrument,
    Sampler,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstrumentSettings {
    #[serde(rename_all = "kebab-case")]
    Test {
        #[serde(rename = "midi-in")]
        midi_input_channel: MidiChannel,
    },
    #[serde(rename_all = "kebab-case")]
    SimpleSynth {
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
    },
    #[serde(rename_all = "kebab-case")]
    FmSynthesizer {
        #[serde(rename = "midi-in")]
        midi_input_channel: MidiChannel,
        #[serde(rename = "preset")]
        preset_name: String,
    },
    // TODO Sampler
}

impl InstrumentSettings {
    pub(crate) fn instantiate(
        &self,
        sample_rate: usize,
        load_only_test_entities: bool,
    ) -> (MidiChannel, BoxedEntity) {
        if load_only_test_entities {
            let midi_input_channel = match self {
                InstrumentSettings::Test { midi_input_channel } => *midi_input_channel,
                InstrumentSettings::SimpleSynth { midi_input_channel } => *midi_input_channel,
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
                BoxedEntity::TestInstrument(Box::new(TestInstrument::new_with(sample_rate))),
            );
        }
        match self {
            InstrumentSettings::Test { midi_input_channel } => (
                *midi_input_channel,
                BoxedEntity::TestInstrument(Box::new(TestInstrument::new_with(sample_rate))),
            ),
            InstrumentSettings::SimpleSynth { midi_input_channel } => (
                *midi_input_channel,
                BoxedEntity::SimpleSynthesizer(Box::new(SimpleSynthesizer::new(sample_rate))),
            ),
            InstrumentSettings::Welsh {
                midi_input_channel,
                preset_name,
            } => (
                *midi_input_channel,
                BoxedEntity::WelshSynth(Box::new(welsh::WelshSynth::new_with(
                    sample_rate,
                    SynthPatch::by_name(preset_name),
                ))),
            ),
            InstrumentSettings::Drumkit {
                midi_input_channel,
                preset_name: _preset,
            } => (
                *midi_input_channel,
                BoxedEntity::DrumkitSampler(Box::new(
                    drumkit_sampler::DrumkitSampler::new_from_files(),
                )),
            ),
            InstrumentSettings::Sampler {
                midi_input_channel,
                filename,
            } => (
                *midi_input_channel,
                BoxedEntity::Sampler(Box::new(Sampler::new_from_file(&filename))),
            ),
            InstrumentSettings::FmSynthesizer {
                midi_input_channel,
                preset_name: preset,
            } => (
                *midi_input_channel,
                BoxedEntity::FmSynthesizer(Box::new(FmSynthesizer::new_with_preset(
                    sample_rate,
                    &FmSynthesizer::preset_for_name(preset),
                ))),
            ),
        }
    }
}
