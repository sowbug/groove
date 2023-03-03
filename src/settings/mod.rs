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
    clock::{BeatValue, PerfectTimeUnit, TimeSignature},
    controllers::{Note, Pattern},
    Clock,
};
use groove_core::ParameterType;
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
impl Into<Pattern<Note>> for PatternSettings {
    fn into(self) -> Pattern<Note> {
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ClockSettings {
    #[serde(skip)]
    sample_rate: usize, // Samples per second; granularity of a tick().

    #[serde(rename = "bpm")]
    beats_per_minute: f32,

    #[serde(rename = "time-signature")]
    time_signature: TimeSignature,

    #[serde(skip)]
    midi_ticks_per_second: usize,
}

impl ClockSettings {
    pub(crate) fn new_with(
        sample_rate: usize,
        beats_per_minute: f32,
        time_signature: (usize, usize),
    ) -> Self {
        Self {
            sample_rate,
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
        self.sample_rate
    }

    pub fn set_sample_rate(&mut self, sample_rate: usize) {
        self.sample_rate = sample_rate;
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
        (seconds * self.sample_rate as f32) as usize
    }

    // TODO: Horrible precision problems
    pub fn beats_per_sample(&self) -> f32 {
        (self.bpm() / 60.0) / self.sample_rate() as f32
    }

    #[allow(dead_code)]
    pub(crate) fn set_time_signature(&mut self, time_signature: TimeSignature) {
        self.time_signature = time_signature;
    }
}
impl Default for ClockSettings {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            beats_per_minute: 128.0,
            time_signature: TimeSignature { top: 4, bottom: 4 },
            midi_ticks_per_second: 960,
        }
    }
}
impl Into<Clock> for ClockSettings {
    fn into(self) -> Clock {
        Clock::new_with(
            self.sample_rate,
            self.beats_per_minute as ParameterType,
            self.midi_ticks_per_second,
        )
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ControlSettings {
    pub id: DeviceId,
    pub source: DeviceId,
    pub target: ControlTargetSettings,
}

#[cfg(test)]
impl ClockSettings {
    const TEST_SAMPLE_RATE: usize = 44100;
    const TEST_BPM: f32 = 99.0;
    pub fn new_test() -> Self {
        Self::new_with(
            ClockSettings::TEST_SAMPLE_RATE,
            ClockSettings::TEST_BPM,
            (4, 4),
        )
    }
}
