// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::delay::{AllPassDelayLine, Delays, RecirculatingDelayLine};
use groove_core::{
    traits::{IsEffect, TransformsAudio},
    Sample,
};
use groove_macros::{Control, Uid};
use std::str::FromStr;
use strum_macros::{Display, EnumString, FromRepr};

/// Schroeder reverb. Uses four parallel recirculating delay lines feeding into
/// a series of two all-pass delay lines.
#[derive(Control, Debug, Default, Uid)]
pub struct Reverb {
    uid: usize,

    // How much the effect should attenuate the input.
    #[controllable]
    attenuation: f32,

    // what percentage should be unprocessed. 0.0 = all effect. 0.0 = all
    // unchanged.
    //
    // TODO: maybe handle the wet/dry more centrally. It seems like it'll be
    // repeated a lot.
    #[controllable]
    wet_dry_mix: f32,
    recirc_delay_lines: Vec<RecirculatingDelayLine>,
    allpass_delay_lines: Vec<AllPassDelayLine>,
}
impl IsEffect for Reverb {}
impl TransformsAudio for Reverb {
    fn transform_channel(&mut self, _channel: usize, input_sample: Sample) -> Sample {
        let input_attenuated = input_sample * self.attenuation;
        let recirc_output = self.recirc_delay_lines[0].pop_output(input_attenuated)
            + self.recirc_delay_lines[1].pop_output(input_attenuated)
            + self.recirc_delay_lines[2].pop_output(input_attenuated)
            + self.recirc_delay_lines[3].pop_output(input_attenuated);
        let adl_0_out = self.allpass_delay_lines[0].pop_output(recirc_output);
        self.allpass_delay_lines[1].pop_output(adl_0_out) * self.wet_dry_mix
            + input_sample * (1.0 - self.wet_dry_mix)
    }
}
impl Reverb {
    #[allow(dead_code)]
    fn new() -> Self {
        Self::default()
    }

    pub fn new_with(
        sample_rate: usize,
        wet_dry_mix: f32,
        attenuation: f32,
        reverb_seconds: f32,
    ) -> Self {
        // Thanks to https://basicsynth.com/ (page 133 of paperback) for
        // constants.
        Self {
            uid: Default::default(),
            wet_dry_mix,
            attenuation,
            recirc_delay_lines: vec![
                RecirculatingDelayLine::new_with(sample_rate, 0.0297, reverb_seconds, 0.001, 1.0),
                RecirculatingDelayLine::new_with(sample_rate, 0.0371, reverb_seconds, 0.001, 1.0),
                RecirculatingDelayLine::new_with(sample_rate, 0.0411, reverb_seconds, 0.001, 1.0),
                RecirculatingDelayLine::new_with(sample_rate, 0.0437, reverb_seconds, 0.001, 1.0),
            ],
            allpass_delay_lines: vec![
                AllPassDelayLine::new_with(sample_rate, 0.09683, 0.0050, 0.001, 1.0),
                AllPassDelayLine::new_with(sample_rate, 0.03292, 0.0017, 0.001, 1.0),
            ],
        }
    }

    pub fn set_attenuation(&mut self, attenuation: f32) {
        self.attenuation = attenuation;
    }

    pub fn set_control_attenuation(&mut self, attenuation: groove_core::control::F32ControlValue) {
        self.set_attenuation(attenuation.0);
    }

    pub fn set_wet_dry_mix(&mut self, mix: f32) {
        self.wet_dry_mix = mix;
    }

    pub fn set_control_wet_dry_mix(&mut self, mix: groove_core::control::F32ControlValue) {
        self.set_wet_dry_mix(mix.0);
    }
}

#[cfg(test)]
mod tests {
    use super::Reverb;
    use crate::tests::DEFAULT_SAMPLE_RATE;
    use groove_core::{traits::TransformsAudio, Sample};

    #[test]
    fn reverb_dry_works() {
        let mut fx = Reverb::new_with(DEFAULT_SAMPLE_RATE, 0.0, 0.5, 1.5);
        assert_eq!(
            fx.transform_channel(0, Sample::from(0.8f32)),
            Sample::from(0.8f32)
        );
        assert_eq!(
            fx.transform_channel(0, Sample::from(0.7f32)),
            Sample::from(0.7f32)
        );
    }

    #[test]
    fn reverb_wet_works() {
        // This test is lame, because I can't think of a programmatic way to
        // test that reverb works. I observed that with the Schroeder reverb set
        // to 0.5 seconds, we start getting back nonzero samples (first
        // 0.47767496) at samples: 29079, seconds: 0.65938777. This doesn't look
        // wrong, but I couldn't have predicted that exact number.
        let mut fx = Reverb::new_with(DEFAULT_SAMPLE_RATE, 1.0, 0.9, 0.5);
        assert_eq!(fx.transform_channel(0, Sample::from(0.8)), Sample::SILENCE);
        let mut s = Sample::default();
        for _ in 0..44100 {
            s += fx.transform_channel(0, Sample::SILENCE);
        }
        assert!(s != Sample::SILENCE);

        // TODO: this test might not do anything. I refactored it in a hurry and
        // took something that looked critical (skipping the clock to 0.5
        // seconds) out of it, but it still passed. I might not actually be
        // testing anything useful.
    }
}