// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    traits::{IsEffect, TransformsAudio},
    Sample,
};
use groove_macros::{Control, Synchronization, Uid};
use std::str::FromStr;

use strum::EnumCount;
use strum_macros::{
    Display, EnumCount as EnumCountMacro, EnumIter, EnumString, FromRepr, IntoStaticStr,
};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Synchronization)]
#[cfg_attr(
    feature = "serialization",
    derive(Serialize, Deserialize),
    serde(rename = "mixer", rename_all = "kebab-case")
)]
pub struct MixerParams {}
impl MixerParams {}

#[derive(Clone, Control, Debug, Default, Uid)]
pub struct Mixer {
    uid: usize,
    params: MixerParams,
}
impl IsEffect for Mixer {}
impl TransformsAudio for Mixer {
    fn transform_channel(&mut self, _channel: usize, input_sample: Sample) -> Sample {
        // This is a simple pass-through because it's the job of the
        // infrastructure to provide a sum of all inputs as the input.
        // Eventually this might turn into a weighted mixer, or we might handle
        // that by putting `Gain`s in front.
        input_sample
    }
}
impl Mixer {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn params(&self) -> MixerParams {
        self.params
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_mixer_mainline() {
        // This could be replaced with a test, elsewhere, showing that
        // Orchestrator's gather_audio() method can gather audio.
    }
}
