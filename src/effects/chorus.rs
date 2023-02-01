use super::delay::{DelayLine, Delays};
use crate::{
    clock::Clock,
    common::F32ControlValue,
    common::OldMonoSample,
    messages::EntityMessage,
    traits::{Controllable, HasUid, IsEffect, TransformsAudio, Updateable},
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
    fn transform_audio(&mut self, _clock: &Clock, input: OldMonoSample) -> OldMonoSample {
        let index_offset = self.delay_factor / self.voices;
        let mut sum = self.delay.pop_output(input);
        for i in 1..self.voices as isize {
            sum += self.delay.peek_indexed_output(i * index_offset as isize);
        }

        self.wet_dry_mix * sum / self.voices as OldMonoSample + (1.0 - self.wet_dry_mix) * input
    }
}
impl Updateable for Chorus {
    type Message = EntityMessage;
}

impl Chorus {
    #[allow(dead_code)]
    fn new() -> Self {
        Self::default()
    }

    pub(crate) fn new_with(
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

    pub(crate) fn set_control_wet_dry_mix(&mut self, value: F32ControlValue) {
        self.set_wet_dry_mix(value.0);
    }
}

#[cfg(test)]
mod tests {
    //TODO
}
