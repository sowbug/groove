use crate::{
    common::Normal,
    controllers::F32ControlValue,
    traits::{Controllable, HasUid, IsEffect, TransformsAudio},
};
use groove_core::Sample;
use groove_macros::{Control, Uid};
use std::str::FromStr;
use strum_macros::{Display, EnumString, FromRepr};

#[derive(Control, Debug, Default, Uid)]
pub struct Gain {
    uid: usize,

    #[controllable]
    ceiling: Normal,
}
impl IsEffect for Gain {}
impl TransformsAudio for Gain {
    fn transform_channel(&mut self, _channel: usize, input_sample: Sample) -> Sample {
        Sample(input_sample.0 * self.ceiling.value())
    }
}
impl Gain {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            ceiling: Normal::new(1.0),
            ..Default::default()
        }
    }

    pub fn new_with(ceiling: Normal) -> Self {
        Self {
            ceiling,
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    pub fn ceiling(&self) -> Normal {
        self.ceiling
    }

    pub fn set_ceiling(&mut self, ceiling: Normal) {
        self.ceiling = ceiling;
    }

    pub fn set_control_ceiling(&mut self, value: F32ControlValue) {
        self.set_ceiling(Normal::new_from_f32(value.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{instruments::AudioSource, traits::Generates};
    use groove_core::StereoSample;

    #[test]
    fn test_gain_mainline() {
        let mut gain = Gain::new_with(Normal::new(0.5));
        assert_eq!(
            gain.transform_audio(AudioSource::new_with(AudioSource::LOUD).value()),
            StereoSample::from(0.5)
        );
    }
}
