pub(crate) mod controllers;
pub(crate) mod effects;
pub(crate) mod instruments;
pub(crate) mod patches;
pub(crate) mod songs;

use self::{
    controllers::{ControlTargetSettings, ControllerSettings},
    effects::EffectSettings,
    instruments::InstrumentSettings,
};
use crate::{
    clock::{BeatValue, TimeSignature},
    controllers::{Note, Pattern},
    Orchestrator,
};
use groove_core::{
    time::{Clock, PerfectTimeUnit},
    ParameterType,
};
use serde::{Deserialize, Serialize};

pub type DeviceId = String;

type MidiChannel = u8;

#[derive(Debug, Clone)]
pub enum LoadError {
    #[allow(dead_code)]
    FileError,
    FormatError,
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
impl PatternSettings {
    pub fn into_pattern(&self) -> Pattern<Note> {
        let mut r = Pattern::<Note> {
            note_value: self.note_value.clone(),
            notes: Vec::default(),
        };
        for note_sequence in self.notes.iter() {
            let mut note_vec = Vec::default();
            for note in note_sequence.iter() {
                note_vec.push(Note {
                    key: Pattern::<Note>::note_to_value(note),
                    velocity: 127,
                    duration: PerfectTimeUnit(1.0),
                });
            }
            r.notes.push(note_vec);
        }
        r
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TrackSettings {
    pub id: DeviceId,
    pub midi_channel: MidiChannel,

    #[serde(rename = "patterns")]
    pub pattern_ids: Vec<DeviceId>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ClockSettings {
    #[serde(skip)]
    sample_rate: usize, // Samples per second; granularity of a tick().

    #[serde(rename = "bpm")]
    beats_per_minute: f32,

    #[serde(rename = "time-signature")]
    time_signature: TimeSignatureSettings,

    #[serde(skip)]
    midi_ticks_per_second: usize,
}
impl Default for ClockSettings {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            beats_per_minute: 128.0,
            time_signature: TimeSignatureSettings { top: 4, bottom: 4 },
            midi_ticks_per_second: 960,
        }
    }
}
#[allow(clippy::from_over_into)]
impl Into<Clock> for ClockSettings {
    fn into(self) -> Clock {
        Clock::new_with(
            self.sample_rate,
            self.beats_per_minute as ParameterType,
            self.midi_ticks_per_second,
        )
    }
}
#[allow(clippy::from_over_into)]
impl Into<Orchestrator> for ClockSettings {
    fn into(self) -> Orchestrator {
        Orchestrator::new_with(self.sample_rate, self.beats_per_minute as ParameterType)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ControlSettings {
    pub id: DeviceId,
    pub source: DeviceId,
    pub target: ControlTargetSettings,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TimeSignatureSettings {
    pub top: usize,
    pub bottom: usize,
}
impl Default for TimeSignatureSettings {
    fn default() -> Self {
        Self { top: 4, bottom: 4 }
    }
}
#[allow(clippy::from_over_into)]
impl Into<TimeSignature> for TimeSignatureSettings {
    fn into(self) -> TimeSignature {
        let r = TimeSignature::new_with(self.top, self.bottom);
        if let Ok(ts) = r {
            ts
        } else {
            panic!("Failed to instantiate TimeSignature: {}", r.err().unwrap())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ClockSettings, TimeSignatureSettings};

    impl ClockSettings {
        const TEST_SAMPLE_RATE: usize = 44100;
        const TEST_BPM: f32 = 99.0;

        pub(crate) fn new_with(
            sample_rate: usize,
            beats_per_minute: f32,
            time_signature: (usize, usize),
        ) -> Self {
            Self {
                sample_rate,
                beats_per_minute,
                time_signature: TimeSignatureSettings {
                    top: time_signature.0,
                    bottom: time_signature.1,
                },
                midi_ticks_per_second: 960, // TODO
            }
        }

        pub fn new_test() -> Self {
            Self::new_with(
                ClockSettings::TEST_SAMPLE_RATE,
                ClockSettings::TEST_BPM,
                (4, 4),
            )
        }
    }
}
