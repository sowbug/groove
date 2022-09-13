pub mod automation;
pub mod effects;
pub mod song;

use serde::{Deserialize, Serialize};

use crate::{
    common::DeviceId,
    primitives::clock::{BeatValue, TimeSignature},
    synthesizers::welsh::PresetName,
};

use self::effects::EffectSettings;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum InstrumentType {
    Welsh,
    Drumkit,
}

type MidiChannel = u8;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum InstrumentSettings {
    #[serde(rename_all = "kebab-case")]
    Welsh {
        id: DeviceId,
        #[serde(rename = "midi-in")]
        midi_input_channel: MidiChannel,
        #[serde(rename = "preset")]
        preset_name: PresetName,
    },
    #[serde(rename_all = "kebab-case")]
    Drumkit {
        id: DeviceId,
        #[serde(rename = "midi-in")]
        midi_input_channel: MidiChannel,
        #[serde(rename = "preset")]
        preset_name: String,
    },
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum DeviceSettings {
    Instrument(InstrumentSettings),
    Sequencer(DeviceId),
    Effect(EffectSettings),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct PatternSettings {
    pub id: DeviceId,
    pub beat_value: Option<BeatValue>,
    pub notes: Vec<Vec<String>>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct TrackSettings {
    pub id: DeviceId,
    pub midi_channel: MidiChannel,

    #[serde(rename = "patterns")]
    pub pattern_ids: Vec<DeviceId>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct ClockSettings {
    #[serde(rename = "sample-rate")]
    samples_per_second: usize, // Samples per second; granularity of a tick().

    #[serde(rename = "bpm")]
    beats_per_minute: f32,

    #[serde(rename = "time-signature")]
    time_signature: TimeSignature,
}

impl ClockSettings {
    #[allow(dead_code)]
    pub(crate) fn new(
        samples_per_second: usize,
        beats_per_minute: f32,
        time_signature: (u32, u32),
    ) -> Self {
        Self {
            samples_per_second,
            beats_per_minute,
            time_signature: TimeSignature {
                top: time_signature.0,
                bottom: time_signature.1,
            },
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

    pub fn time_signature(&self) -> TimeSignature {
        self.time_signature
    }

    #[allow(dead_code)]
    pub(crate) fn bpm(&self) -> f32 {
        self.beats_per_minute
    }
}

impl Default for ClockSettings {
    fn default() -> Self {
        Self {
            samples_per_second: 44100,
            beats_per_minute: 128.0,
            time_signature: TimeSignature { top: 4, bottom: 4 },
        }
    }
}
