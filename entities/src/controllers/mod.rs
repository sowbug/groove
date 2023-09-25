// Copyright (c) 2023 Mike Tsao. All rights reserved.

use ensnare_core::midi::prelude::*;
use serde::{Deserialize, Serialize};

pub use calculator::Calculator;
pub use control_trip::{ControlPath, ControlStep};

mod calculator;
mod control_trip;
mod lfo;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename = "midi", rename_all = "kebab-case")]
pub struct MidiChannelParams {
    pub midi_in: MidiChannel,
    pub midi_out: MidiChannel,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename = "midi", rename_all = "kebab-case")]
pub struct MidiChannelInputParams {
    pub midi_in: MidiChannel,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename = "midi", rename_all = "kebab-case")]
pub struct MidiChannelOutputParams {
    pub midi_out: MidiChannel,
}

