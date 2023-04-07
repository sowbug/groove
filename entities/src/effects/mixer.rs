// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    traits::{IsEffect, Resets, TransformsAudio},
    Sample,
};
use groove_proc_macros::{Nano, Uid};
use std::str::FromStr;

use strum::EnumCount;
use strum_macros::{Display, EnumCount as EnumCountMacro, EnumString, FromRepr, IntoStaticStr};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Nano, Uid)]
pub struct Mixer {
    uid: usize,
}
impl IsEffect for Mixer {}
impl Resets for Mixer {
    fn reset(&mut self, _sample_rate: usize) {}
}
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
    pub fn new_with(_params: MixerNano) -> Self {
        Self {
            ..Default::default()
        }
    }

    #[allow(unreachable_patterns)]
    pub fn update(&mut self, message: MixerMessage) {
        match message {
            MixerMessage::Mixer(s) => *self = Self::new_with(s),
            _ => self.derived_update(message),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn mixer_mainline() {
        // This could be replaced with a test, elsewhere, showing that
        // Orchestrator's gather_audio() method can gather audio.
    }
}
