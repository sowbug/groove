use super::{
    patches::{EnvelopeSettings, SynthPatch, WaveformType},
    MidiChannel,
};
use crate::{
    entities::BoxedEntity,
    instruments::{
        drumkit_sampler,
        envelopes::AdsrEnvelope,
        oscillators::Oscillator,
        welsh::{self},
        FmSynthesizer, SimpleSynthesizer,
    },
    messages::EntityMessage,
    traits::TestInstrument,
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
    FmSynthesizer {
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
    ) -> (MidiChannel, BoxedEntity) {
        if load_only_test_entities {
            #[allow(unused_variables)]
            let midi_input_channel = match self {
                InstrumentSettings::Test { midi_input_channel } => *midi_input_channel,
                InstrumentSettings::SimpleSynth { midi_input_channel } => *midi_input_channel,
                InstrumentSettings::Welsh {
                    midi_input_channel,
                    preset_name,
                } => *midi_input_channel,
                InstrumentSettings::Drumkit {
                    midi_input_channel,
                    preset_name,
                } => *midi_input_channel,
                InstrumentSettings::FmSynthesizer {
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
                BoxedEntity::TestInstrument(Box::new(TestInstrument::<EntityMessage>::default())),
            );
        }
        match self {
            InstrumentSettings::Test { midi_input_channel } => (
                *midi_input_channel,
                BoxedEntity::TestInstrument(Box::new(TestInstrument::<EntityMessage>::default())),
            ),
            InstrumentSettings::SimpleSynth { midi_input_channel } => (
                *midi_input_channel,
                BoxedEntity::SimpleSynthesizer(Box::new(SimpleSynthesizer::default())),
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
            InstrumentSettings::FmSynthesizer {
                midi_input_channel,
                preset_name: preset,
            } => (
                *midi_input_channel,
                BoxedEntity::FmSynthesizer(Box::new(FmSynthesizer::new_with(
                    &FmSynthesizer::preset_for_name(preset),
                ))),
            ),
            InstrumentSettings::Oscillator {
                midi_input_channel,
                waveform,
                frequency,
            } => (
                *midi_input_channel,
                BoxedEntity::Oscillator(Box::new(Oscillator::new_with_type_and_frequency(
                    *waveform, *frequency,
                ))),
            ),
            InstrumentSettings::Envelope {
                midi_input_channel,
                attack,
                decay,
                sustain,
                release,
            } => (
                *midi_input_channel,
                BoxedEntity::AdsrEnvelope(Box::new(AdsrEnvelope::new_with(&EnvelopeSettings {
                    attack: *attack,
                    decay: *decay,
                    sustain: *sustain,
                    release: *release,
                }))),
            ),
        }
    }
}
