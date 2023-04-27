// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! The `groove-settings` crate manages serialization. Why is this more than
//! `#[derive(Deserialize, Serialize)]`? Because the
//! [Groove](groove_core::Groove) vision is that project files be as easy to work with
//! as a GUI DAW, and that means that the file format should remain relatively
//! stable, even if the structs that understand that format change. Keeping a
//! set of serialized structs separate from the inner engine helps reduce
//! file-format churn.

pub use songs::SongSettings;

pub mod controllers;
pub(crate) mod effects;
pub(crate) mod instruments;
pub(crate) mod patches;
pub(crate) mod songs;

use self::{
    controllers::{ControlTargetSettings, ControllerSettings},
    effects::EffectSettings,
    instruments::InstrumentSettings,
};
use groove_core::time::{BeatValue, PerfectTimeUnit, TimeSignature};
use groove_entities::controllers::{Note, Pattern};
use serde::{Deserialize, Serialize};
use strum_macros::FromRepr;

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
    pub note_value: Option<BeatValueSettings>,
    pub notes: Vec<Vec<String>>,
}
impl PatternSettings {
    pub fn into_pattern(&self) -> Pattern<Note> {
        let note_value = self
            .note_value
            .as_ref()
            .map(|note_value| note_value.into_beat_value());
        let mut r = Pattern::<Note> {
            note_value,
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

#[derive(Clone, Debug, Default, Deserialize, FromRepr, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BeatValueSettings {
    Octuple = 128,   // large/maxima
    Quadruple = 256, // long
    Double = 512,    // breve
    Whole = 1024,    // semibreve
    Half = 2048,     // minim
    #[default]
    Quarter = 4096, // crotchet
    Eighth = 8192,   // quaver
    Sixteenth = 16384, // semiquaver
    ThirtySecond = 32768, // demisemiquaver
    SixtyFourth = 65536, // hemidemisemiquaver
    OneHundredTwentyEighth = 131072, // semihemidemisemiquaver / quasihemidemisemiquaver
    TwoHundredFiftySixth = 262144, // demisemihemidemisemiquaver
    FiveHundredTwelfth = 524288, // winner winner chicken dinner
}
impl BeatValueSettings {
    pub fn into_beat_value(&self) -> BeatValue {
        match self {
            BeatValueSettings::Octuple => BeatValue::Octuple,
            BeatValueSettings::Quadruple => BeatValue::Quadruple,
            BeatValueSettings::Double => BeatValue::Double,
            BeatValueSettings::Whole => BeatValue::Whole,
            BeatValueSettings::Half => BeatValue::Half,
            BeatValueSettings::Quarter => BeatValue::Quarter,
            BeatValueSettings::Eighth => BeatValue::Eighth,
            BeatValueSettings::Sixteenth => BeatValue::Sixteenth,
            BeatValueSettings::ThirtySecond => BeatValue::ThirtySecond,
            BeatValueSettings::SixtyFourth => BeatValue::SixtyFourth,
            BeatValueSettings::OneHundredTwentyEighth => BeatValue::OneHundredTwentyEighth,
            BeatValueSettings::TwoHundredFiftySixth => BeatValue::TwoHundredFiftySixth,
            BeatValueSettings::FiveHundredTwelfth => BeatValue::FiveHundredTwelfth,
        }
    }
}
