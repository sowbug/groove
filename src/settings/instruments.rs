use super::{patches::SynthPatch, MidiChannel};
use crate::{
    instruments::{
        drumkit_sampler,
        welsh::{self, PatchName},
    },
    traits::{IsInstrument, TestInstrument},
    GrooveMessage,
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
    Welsh {
        #[serde(rename = "midi-in")]
        midi_input_channel: MidiChannel,
        #[serde(rename = "preset")]
        preset_name: PatchName,
    },
    #[serde(rename_all = "kebab-case")]
    Drumkit {
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
    ) -> (MidiChannel, Box<dyn IsInstrument<Message = GrooveMessage>>) {
        if load_only_test_entities {
            #[allow(unused_variables)]
            let midi_input_channel = match self {
                InstrumentSettings::Test { midi_input_channel } => *midi_input_channel,
                InstrumentSettings::Welsh {
                    midi_input_channel,
                    preset_name,
                } => *midi_input_channel,
                InstrumentSettings::Drumkit {
                    midi_input_channel,
                    preset_name,
                } => *midi_input_channel,
            };
            return (
                midi_input_channel,
                Box::new(TestInstrument::<GrooveMessage>::default()),
            );
        }
        match self {
            InstrumentSettings::Test { midi_input_channel } => (
                *midi_input_channel,
                Box::new(TestInstrument::<GrooveMessage>::default()),
            ),
            InstrumentSettings::Welsh {
                midi_input_channel,
                preset_name,
            } => (
                *midi_input_channel,
                Box::new(welsh::WelshSynth::new_with(
                    sample_rate,
                    SynthPatch::by_name(preset_name),
                )),
            ),
            InstrumentSettings::Drumkit {
                midi_input_channel,
                preset_name: _preset,
            } => (
                *midi_input_channel,
                Box::new(drumkit_sampler::Sampler::new_from_files()),
            ),
        }
    }
}
