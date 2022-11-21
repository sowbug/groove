pub(crate) mod control;
pub(crate) mod effects;
pub(crate) mod patches;
pub(crate) mod songs;

use self::{effects::EffectSettings, patches::SynthPatch};
use crate::{
    clock::{BeatValue, TimeSignature},
    common::DeviceId,
    effects::arpeggiator::Arpeggiator,
    instruments::{
        drumkit_sampler,
        welsh::{self, PatchName},
    },
    messages::GrooveMessage,
    traits::{NewIsController, NewIsInstrument},
};
use serde::{Deserialize, Serialize};

type MidiChannel = u8;

#[derive(Debug, Clone)]
pub enum LoadError {
    #[allow(dead_code)]
    FileError,
    FormatError,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstrumentSettings {
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ControllerSettings {
    #[serde(rename_all = "kebab-case")]
    Arpeggiator {
        #[serde(rename = "midi-in")]
        midi_input_channel: MidiChannel,
        #[serde(rename = "midi-out")]
        midi_output_channel: MidiChannel,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeviceSettings {
    Instrument(DeviceId, InstrumentSettings),
    Controller(DeviceId, ControllerSettings),
    Effect(DeviceId, EffectSettings),
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct PatternSettings {
    pub id: DeviceId,
    pub note_value: Option<BeatValue>,
    pub notes: Vec<Vec<String>>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TrackSettings {
    pub id: DeviceId,
    pub midi_channel: MidiChannel,

    #[serde(rename = "patterns")]
    pub pattern_ids: Vec<DeviceId>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ClockSettings {
    #[serde(rename = "sample-rate")]
    samples_per_second: usize, // Samples per second; granularity of a tick().

    #[serde(rename = "bpm")]
    beats_per_minute: f32,

    #[serde(rename = "time-signature")]
    time_signature: TimeSignature,

    #[serde(skip)]
    midi_ticks_per_second: usize,
}

impl ClockSettings {
    #[allow(dead_code)]
    pub(crate) fn new(
        samples_per_second: usize,
        beats_per_minute: f32,
        time_signature: (usize, usize),
    ) -> Self {
        Self {
            samples_per_second,
            beats_per_minute,
            time_signature: TimeSignature {
                top: time_signature.0,
                bottom: time_signature.1,
            },
            midi_ticks_per_second: 960, // TODO
        }
    }

    #[allow(dead_code)]
    pub(crate) fn new_defaults() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn sample_rate(&self) -> usize {
        self.samples_per_second
    }

    pub fn midi_ticks_per_second(&self) -> usize {
        self.midi_ticks_per_second
    }

    pub fn time_signature(&self) -> TimeSignature {
        self.time_signature
    }

    pub fn bpm(&self) -> f32 {
        self.beats_per_minute
    }

    pub fn set_bpm(&mut self, new_value: f32) {
        self.beats_per_minute = new_value;
    }

    pub fn beats_to_samples(&self, beats: f32) -> usize {
        let seconds = beats * 60.0 / self.beats_per_minute;
        (seconds * self.samples_per_second as f32) as usize
    }

    // TODO: Horrible precision problems
    pub fn beats_per_sample(&self) -> f32 {
        (self.bpm() / 60.0) / self.sample_rate() as f32
    }
}

impl Default for ClockSettings {
    fn default() -> Self {
        Self {
            samples_per_second: 44100,
            beats_per_minute: 128.0,
            time_signature: TimeSignature { top: 4, bottom: 4 },
            midi_ticks_per_second: 960,
        }
    }
}

impl InstrumentSettings {
    pub(crate) fn instantiate(
        &self,
        sample_rate: usize,
    ) -> (
        MidiChannel,
        Box<dyn NewIsInstrument<Message = GrooveMessage>>,
    ) {
        match self {
            InstrumentSettings::Welsh {
                midi_input_channel,
                preset_name,
            } => (
                *midi_input_channel,
                Box::new(welsh::Synth::new_with(
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

impl ControllerSettings {
    pub(crate) fn instantiate(
        &self,
        _sample_rate: usize,
    ) -> (
        MidiChannel,
        MidiChannel,
        Box<dyn NewIsController<Message = GrooveMessage>>,
    ) {
        match *self {
            ControllerSettings::Arpeggiator {
                midi_input_channel,
                midi_output_channel,
            } => (
                midi_input_channel,
                midi_output_channel,
                Box::new(Arpeggiator::new_with(midi_output_channel)),
            ),
        }
    }
}

#[cfg(test)]
impl ClockSettings {
    const TEST_SAMPLE_RATE: usize = 256;
    const TEST_BPM: f32 = 99.;
    pub fn new_test() -> Self {
        Self::new(
            ClockSettings::TEST_SAMPLE_RATE,
            ClockSettings::TEST_BPM,
            (4, 4),
        )
    }
}
