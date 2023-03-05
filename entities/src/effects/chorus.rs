use super::delay::{DelayLine, Delays};
use groove_core::{
    control::F32ControlValue,
    traits::{Controllable, HasUid, IsEffect, TransformsAudio},
    Sample, SampleType,
};
use groove_macros::{Control, Uid};
use std::str::FromStr;
use strum_macros::{Display, EnumString, FromRepr};

/// Schroeder reverb. Uses four parallel recirculating delay lines feeding into
/// a series of two all-pass delay lines.
#[derive(Control, Debug, Default, Uid)]
pub struct Chorus {
    uid: usize,

    voices: usize,
    delay_factor: usize,

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

    pub fn new_with(
        sample_rate: usize,
        wet_dry_mix: f32,
        voices: usize,
        delay_factor: usize,
    ) -> Self {
        // TODO: the delay_seconds param feels like a hack
        Self {
            uid: Default::default(),
            wet_dry_mix,
            voices,
            delay_factor,
            delay: DelayLine::new_with(sample_rate, delay_factor as f32 / sample_rate as f32, 1.0),
        }
    }

    pub(crate) fn set_wet_dry_mix(&mut self, wet_pct: f32) {
        self.wet_dry_mix = wet_pct;
    }

    pub fn set_control_wet_dry_mix(&mut self, value: F32ControlValue) {
        self.set_wet_dry_mix(value.0);
    }
}

#[cfg(test)]
mod tests {
    //TODO
}
