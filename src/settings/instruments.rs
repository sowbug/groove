use super::{
    patches::{EnvelopeSettings, SynthPatch, WaveformType},
    MidiChannel,
};
use crate::{
    instruments::{
        drumkit_sampler,
        envelopes::AdsrEnvelope,
        oscillators::Oscillator,
        welsh::{self, PatchName},
    },
    messages::EntityMessage,
    traits::{IsInstrument, TestInstrument},
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
    #[serde(rename_all = "kebab-case")]
    Oscillator {
        #[serde(rename = "midi-in")]
        midi_input_channel: MidiChannel,
        waveform: WaveformType,
        frequency: f32,
    },
    #[serde(rename_all = "kebab-case")]
    Envelope {
        #[serde(rename = "midi-in")]
        midi_input_channel: MidiChannel,
        attack: f32,
        decay: f32,
        sustain: f32,
        release: f32,
    },
    // TODO Sampler
}

impl InstrumentSettings {
    pub(crate) fn instantiate(
        &self,
        sample_rate: usize,
        load_only_test_entities: bool,
    ) -> (
        MidiChannel,
        Box<dyn IsInstrument<Message = EntityMessage, ViewMessage = EntityMessage>>,
    ) {
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
                InstrumentSettings::Oscillator {
                    midi_input_channel,
                    waveform,
                    frequency,
                } => *midi_input_channel,
                InstrumentSettings::Envelope {
                    midi_input_channel,
                    attack,
                    decay,
                    sustain,
                    release,
                } => *midi_input_channel,
            };
            return (
                midi_input_channel,
                Box::new(TestInstrument::<EntityMessage>::default()),
            );
        }
        match self {
            InstrumentSettings::Test { midi_input_channel } => (
                *midi_input_channel,
                Box::new(TestInstrument::<EntityMessage>::default()),
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
            InstrumentSettings::Oscillator {
                midi_input_channel,
                waveform,
                frequency,
            } => (
                *midi_input_channel,
                Box::new(Oscillator::new_with_type_and_frequency(
                    *waveform, *frequency,
                )),
            ),
            InstrumentSettings::Envelope {
                midi_input_channel,
                attack,
                decay,
                sustain,
                release,
            } => (
                *midi_input_channel,
                Box::new(AdsrEnvelope::new_with(&EnvelopeSettings {
                    attack: *attack,
                    decay: *decay,
                    sustain: *sustain,
                    release: *release,
                })),
            ),
        }
    }
}
