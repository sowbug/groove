use crate::{
    clock::Clock,
    common::{F32ControlValue, OldMonoSample},
    messages::EntityMessage,
    traits::{Controllable, HasUid, IsEffect, TransformsAudio, Updateable},
};
use groove_macros::{Control, Uid};
use std::str::FromStr;
use strum_macros::{Display, EnumString, FromRepr};

#[derive(Control, Debug, Default, Uid)]
pub struct Compressor {
    uid: usize,

    /// The level above which compression takes effect. Range is 0.0..=1.0, 0.0
    /// corresponds to quietest, and 1.0 corresponds to 0dB.
    #[controllable]
    threshold: f32,

    /// How much to compress the audio above the threshold. For example, 2:1
    /// means that a 2dB input increase leads to a 1dB output increase. Note
    /// that this value is actually the inverted ratio, so that 2:1 is 0.5 (1
    /// divided by 2), and 1:4 is 0.25 (1 divided by 4). Thus, 1.0 means no
    /// compression, and 0.0 is infinite compression (the output remains a
    /// constant amplitude no matter what).
    #[controllable]
    ratio: f32,

    /// How soon the compressor activates after the level exceeds the threshold.
    /// Time in seconds.
    #[controllable]
    attack: f32,

    /// How soon the compressor deactivates after the level drops below the
    /// threshold. Time in seconds.
    #[controllable]
    release: f32,

    // TODO
    #[allow(dead_code)]
    current_gain: f32,
}
impl IsEffect for Compressor {}
impl Updateable for Compressor {
    type Message = EntityMessage;
}
impl TransformsAudio for Compressor {
    fn transform_audio(&mut self, _clock: &Clock, input_sample: OldMonoSample) -> OldMonoSample {
        let input_sample_positive = input_sample.abs();
        if input_sample_positive > self.threshold {
            (self.threshold + (input_sample_positive - self.threshold) * self.ratio)
                * input_sample.signum()
        } else {
            input_sample
        }
    }
}

impl Compressor {
    pub fn new_with(threshold: f32, ratio: f32, attack: f32, release: f32) -> Self {
        Self {
            threshold,
            ratio,
            attack,
            release,
            ..Default::default()
        }
    }

    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    pub fn ratio(&self) -> f32 {
        self.ratio
    }

    pub fn attack(&self) -> f32 {
        self.attack
    }

    pub fn release(&self) -> f32 {
        self.release
    }

    pub fn set_threshold(&mut self, threshold: f32) {
        self.threshold = threshold;
    }

    pub fn set_ratio(&mut self, ratio: f32) {
        self.ratio = ratio;
    }

    pub fn set_attack(&mut self, attack: f32) {
        self.attack = attack;
    }

    pub fn set_release(&mut self, release: f32) {
        self.release = release;
    }

    pub fn set_control_threshold(&mut self, threshold: F32ControlValue) {
        self.threshold = threshold.0;
    }

    pub fn set_control_ratio(&mut self, ratio: F32ControlValue) {
        self.ratio = ratio.0;
    }

    pub fn set_control_attack(&mut self, attack: F32ControlValue) {
        self.attack = attack.0;
    }

    pub fn set_control_release(&mut self, release: F32ControlValue) {
        self.release = release.0;
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        common::Sample, effects::compressor::Compressor, traits::TransformsAudio, BoxedEntity,
        Clock,
    };

    #[test]
    fn compressor_exists() {
        let fx = Compressor::default();
        let entity = BoxedEntity::Compressor(Box::new(fx));
        assert!(entity.as_controllable().is_some());
        assert!(entity.as_is_effect().is_some());
    }

    #[test]
    fn basic_compressor() {
        let clock = Clock::default();
        const THRESHOLD: f32 = 0.25;
        let mut fx = Compressor::new_with(THRESHOLD, 0.5, 0.0, 0.0);
        assert_eq!(
            fx.transform_channel(&clock, 0, Sample::from(0.35)),
            Sample::from((0.35 - THRESHOLD) * 0.5 + THRESHOLD)
        );
    }

    #[test]
    fn nothing_compressor() {
        let clock = Clock::default();
        let mut fx = Compressor::new_with(0.25, 1.0, 0.0, 0.0);
        assert_eq!(
            fx.transform_channel(&clock, 0, Sample::from(0.35f32)),
            Sample::from(0.35f32)
        );
    }

    #[test]
    fn infinite_compressor() {
        let clock = Clock::default();
        let mut fx = Compressor::new_with(0.25, 0.0, 0.0, 0.0);
        assert_eq!(
            fx.transform_channel(&clock, 0, Sample::from(0.35)),
            Sample::from(0.25)
        );
    }
}
