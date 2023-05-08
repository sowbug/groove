// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::delay::{DelayLine, Delays};
use groove_core::{
    traits::{IsEffect, Resets, TransformsAudio},
    ParameterType, Sample, SampleType,
};
use groove_proc_macros::{Control, Params, Uid};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

/// Schroeder reverb. Uses four parallel recirculating delay lines feeding into
/// a series of two all-pass delay lines.
#[derive(Debug, Default, Control, Params, Uid)]
pub struct Chorus {
    uid: usize,

    #[control]
    #[params]
    voices: usize,

    #[control]
    #[params]
    delay_seconds: ParameterType,

    // what percentage of the output should be processed. 0.0 = all dry (no
    // effect). 1.0 = all wet (100% effect).
    //
    // TODO: maybe handle the wet/dry more centrally. It seems like it'll be
    // repeated a lot.
    #[control]
    #[params]
    wet_dry_mix: f32,

    delay: DelayLine,
}
impl IsEffect for Chorus {}
impl TransformsAudio for Chorus {
    fn transform_channel(&mut self, _channel: usize, input_sample: Sample) -> Sample {
        let index_offset = self.delay_seconds / self.voices as ParameterType;
        let mut sum = self.delay.pop_output(input_sample);
        for i in 1..self.voices as isize {
            sum += self.delay.peek_indexed_output(i * index_offset as isize);
        }
        sum * self.wet_dry_mix as SampleType / self.voices as SampleType
            + input_sample * (1.0 - self.wet_dry_mix)
    }
}
impl Resets for Chorus {
    fn reset(&mut self, sample_rate: usize) {
        self.delay.reset(sample_rate);
    }
}
impl Chorus {
    #[allow(dead_code)]
    fn new() -> Self {
        Self::default()
    }

    pub fn new_with(params: &ChorusParams) -> Self {
        // TODO: the delay_seconds param feels like a hack
        Self {
            uid: Default::default(),
            voices: params.voices(),
            delay_seconds: params.delay_seconds(),
            wet_dry_mix: params.wet_dry_mix(),
            delay: DelayLine::new_with(params.delay_seconds(), 1.0),
        }
    }

    pub fn set_wet_dry_mix(&mut self, wet_pct: f32) {
        self.wet_dry_mix = wet_pct;
    }

    #[cfg(feature = "iced-framework")]
    pub fn update(&mut self, message: ChorusMessage) {
        match message {
            ChorusMessage::Chorus(s) => *self = Self::new_with(s),
            _ => self.derived_update(message),
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

    pub fn delay_seconds(&self) -> f64 {
        self.delay_seconds
    }

    pub fn set_delay_seconds(&mut self, delay_seconds: ParameterType) {
        self.delay_seconds = delay_seconds;
    }
}

#[cfg(test)]
mod tests {
    //TODO
}
