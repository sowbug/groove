// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::delay::{DelayLine, Delays};
use groove_core::{
    traits::{IsEffect, TransformsAudio},
    ParameterType, Sample, SampleType,
};
use groove_proc_macros::{Nano, Uid};
use std::str::FromStr;
use strum::EnumCount;
use strum_macros::{Display, EnumCount as EnumCountMacro, EnumString, FromRepr, IntoStaticStr};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

/// Schroeder reverb. Uses four parallel recirculating delay lines feeding into
/// a series of two all-pass delay lines.
#[derive(Debug, Default, Nano, Uid)]
pub struct Chorus {
    uid: usize,

    #[nano]
    voices: usize,

    #[nano]
    delay_factor: usize,

    // what percentage of the output should be processed. 0.0 = all dry (no
    // effect). 1.0 = all wet (100% effect).
    //
    // TODO: maybe handle the wet/dry more centrally. It seems like it'll be
    // repeated a lot.
    #[nano]
    wet_dry_mix: f32,

    delay: DelayLine,
}
impl IsEffect for Chorus {}
impl TransformsAudio for Chorus {
    fn transform_channel(&mut self, _channel: usize, input_sample: Sample) -> Sample {
        let index_offset = self.delay_factor / self.voices;
        let mut sum = self.delay.pop_output(input_sample);
        for i in 1..self.voices as isize {
            sum += self.delay.peek_indexed_output(i * index_offset as isize);
        }
        sum * self.wet_dry_mix as SampleType / self.voices as SampleType
            + input_sample * (1.0 - self.wet_dry_mix)
    }
}
impl Chorus {
    #[allow(dead_code)]
    fn new() -> Self {
        Self::default()
    }

    pub fn new_with(sample_rate: usize, params: ChorusNano) -> Self {
        // TODO: the delay_seconds param feels like a hack
        Self {
            uid: Default::default(),
            voices: params.voices(),
            delay_factor: params.delay_factor(),
            wet_dry_mix: params.wet_dry_mix(),
            delay: DelayLine::new_with(
                sample_rate,
                params.delay_factor as ParameterType / sample_rate as ParameterType,
                1.0,
            ),
        }
    }

    pub fn set_wet_dry_mix(&mut self, wet_pct: f32) {
        self.wet_dry_mix = wet_pct;
    }

    pub fn update(&mut self, message: ChorusMessage) {
        match message {
            ChorusMessage::Chorus(s) => *self = Self::new_with(self.delay.sample_rate(), s),
            ChorusMessage::Voices(voices) => self.set_voices(voices),
            ChorusMessage::DelayFactor(delay_factor) => self.set_delay_factor(delay_factor),
            ChorusMessage::WetDryMix(wet_pct) => self.set_wet_dry_mix(wet_pct),
        }
    }

    pub fn wet_dry_mix(&self) -> f32 {
        self.wet_dry_mix
    }

    pub fn voices(&self) -> usize {
        self.voices
    }

    pub fn set_voices(&mut self, voices: usize) {
        self.voices = voices;
    }

    pub fn delay_factor(&self) -> usize {
        self.delay_factor
    }

    pub fn set_delay_factor(&mut self, delay_factor: usize) {
        self.delay_factor = delay_factor;
    }
}

#[cfg(test)]
mod tests {
    //TODO
}
