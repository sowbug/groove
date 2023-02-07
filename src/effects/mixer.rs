use crate::{
    clock::Clock,
    common::F32ControlValue,
    traits::{Controllable, HasUid, IsEffect, TransformsAudio},
};
use groove_macros::{Control, Uid};
use std::str::FromStr;
use strum_macros::{Display, EnumString, FromRepr};

#[derive(Clone, Control, Debug, Default, Uid)]
pub struct Mixer {
    uid: usize,
}
impl IsEffect for Mixer {}
impl TransformsAudio for Mixer {
    fn transform_channel(
        &mut self,
        _clock: &Clock,
        _channel: usize,
        input_sample: crate::common::Sample,
    ) -> crate::common::Sample {
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
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_mixer_mainline() {
        // This could be replaced with a test, elsewhere, showing that
        // Orchestrator's gather_audio() method can gather audio.
    }
}
