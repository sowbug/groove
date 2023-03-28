// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::delay::{DelayLine, Delays};
use groove_core::{
    traits::{IsEffect, TransformsAudio},
    ParameterType, Sample, SampleType,
};
use groove_proc_macros::{Control, Synchronization, Uid};
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
    serde(rename = "chorus", rename_all = "kebab-case")
)]
pub struct ChorusParams {
    #[sync]
    pub voices: usize,

    #[sync]
    pub delay_factor: usize,
}

impl ChorusParams {
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

/// Schroeder reverb. Uses four parallel recirculating delay lines feeding into
/// a series of two all-pass delay lines.
#[derive(Control, Debug, Default, Uid)]
pub struct Chorus {
    uid: usize,
    params: ChorusParams,

    // what percentage of the output should be processed. 0.0 = all dry (no
    // effect). 1.0 = all wet (100% effect).
    //
    // TODO: maybe handle the wet/dry more centrally. It seems like it'll be
    // repeated a lot.
    #[controllable]
    wet_dry_mix: f32,

    delay: DelayLine,
}
impl IsEffect for Chorus {}
impl TransformsAudio for Chorus {
    fn transform_channel(&mut self, _channel: usize, input_sample: Sample) -> Sample {
        let index_offset = self.params.delay_factor / self.params.voices;
        let mut sum = self.delay.pop_output(input_sample);
        for i in 1..self.params.voices as isize {
            sum += self.delay.peek_indexed_output(i * index_offset as isize);
        }
        sum * self.wet_dry_mix as SampleType / self.params.voices as SampleType
            + input_sample * (1.0 - self.wet_dry_mix)
    }
}
impl Chorus {
    #[allow(dead_code)]
    fn new() -> Self {
        Self::default()
    }

    pub fn new_with(sample_rate: usize, params: ChorusParams, wet_dry_mix: f32) -> Self {
        // TODO: the delay_seconds param feels like a hack
        Self {
            uid: Default::default(),
            wet_dry_mix,
            params,
            delay: DelayLine::new_with(
                sample_rate,
                params.delay_factor as ParameterType / sample_rate as ParameterType,
                1.0,
            ),
        }
    }

    pub(crate) fn set_wet_dry_mix(&mut self, wet_pct: f32) {
        self.wet_dry_mix = wet_pct;
    }

    pub fn set_control_wet_dry_mix(&mut self, value: groove_core::control::F32ControlValue) {
        self.set_wet_dry_mix(value.0);
    }

    pub fn params(&self) -> ChorusParams {
        self.params
    }

    pub fn update(&mut self, message: ChorusParamsMessage) {
        self.params.update(message)
    }
}

#[cfg(test)]
mod tests {
    //TODO
}
