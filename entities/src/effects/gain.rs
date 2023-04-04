// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    traits::{IsEffect, TransformsAudio},
    Normal, Sample,
};
use groove_proc_macros::{Nano, Uid};
use std::str::FromStr;

use strum::EnumCount;
use strum_macros::{Display, EnumCount as EnumCountMacro, EnumString, FromRepr, IntoStaticStr};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Nano, Uid)]
pub struct Gain {
    uid: usize,

    #[nano]
    ceiling: Normal,
}
impl IsEffect for Gain {}
impl TransformsAudio for Gain {
    fn transform_channel(&mut self, _channel: usize, input_sample: Sample) -> Sample {
        Sample(input_sample.0 * self.ceiling.value())
    }
}
impl Gain {
    pub fn new_with(params: GainNano) -> Self {
        Self {
            uid: Default::default(),
            ceiling: params.ceiling,
        }
    }

    pub fn ceiling(&self) -> Normal {
        self.ceiling
    }

    pub fn set_ceiling(&mut self, ceiling: Normal) {
        self.ceiling = ceiling;
    }

    pub fn update(&mut self, message: GainMessage) {
        match message {
            GainMessage::Gain(s) => *self = Self::new_with(s),
            _ => self.derived_update(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use groove_core::{traits::Generates, StereoSample};
    use groove_toys::{ToyAudioSource, ToyAudioSourceNano};

    #[test]
    fn test_gain_mainline() {
        let mut gain = Gain::new_with(GainNano {
            ceiling: Normal::new(0.5),
        });
        assert_eq!(
            gain.transform_audio(
                ToyAudioSource::new_with(ToyAudioSourceNano {
                    level: ToyAudioSource::LOUD
                })
                .value()
            ),
            StereoSample::from(0.5)
        );
    }
}
