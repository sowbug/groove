pub mod control;
pub mod effects;
pub mod song;

use serde::{Deserialize, Serialize};

use crate::{
    common::DeviceId,
    primitives::clock::{BeatValue, TimeSignature},
    synthesizers::welsh::PresetName,
};

use self::effects::EffectSettings;

type MidiChannel = u8;

#[derive(Clone, Debug, Deserialize, Serialize)]
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
    #[serde(rename_all = "kebab-case")]
    Arpeggiator {
        id: DeviceId,
        #[serde(rename = "midi-in")]
        midi_input_channel: MidiChannel,
        #[serde(rename = "midi-out")]
        midi_output_channel: MidiChannel,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeviceSettings {
    Instrument(InstrumentSettings),
    Effect(EffectSettings),
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

    #[allow(dead_code)]
    pub fn time_signature(&self) -> TimeSignature {
        self.time_signature
    }

    #[allow(dead_code)]
    pub(crate) fn bpm(&self) -> f32 {
        self.beats_per_minute
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
