// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::delay::{AllPassDelayLine, Delays, RecirculatingDelayLine};
use groove_core::{
    traits::{IsEffect, TransformsAudio},
    Normal, ParameterType, Sample,
};
use groove_macros::{Control, Synchronization, Uid};
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
    serde(rename = "reverb", rename_all = "kebab-case")
)]
pub struct ReverbParams {
    // How much the effect should attenuate the input.
    #[sync]
    pub attenuation: Normal,

    #[sync]
    pub seconds: ParameterType,
}
impl ReverbParams {
    pub fn attenuation(&self) -> &Normal {
        &self.attenuation
    }

    pub fn set_attenuation(&mut self, attenuation: Normal) {
        self.attenuation = attenuation;
    }

    pub fn seconds(&self) -> f64 {
        self.seconds
    }

    pub fn set_seconds(&mut self, seconds: ParameterType) {
        self.seconds = seconds;
    }
}

/// Schroeder reverb. Uses four parallel recirculating delay lines feeding into
/// a series of two all-pass delay lines.
#[derive(Control, Debug, Uid)]
pub struct Reverb {
    uid: usize,
    params: ReverbParams,

    // what percentage should be unprocessed. 0.0 = all effect. 0.0 = all
    // unchanged.
    //
    // TODO: maybe handle the wet/dry more centrally. It seems like it'll be
    // repeated a lot.
    #[controllable]
    wet_dry_mix: f32,

    left: ReverbChannel,
    right: ReverbChannel,
}

impl IsEffect for Reverb {}
impl TransformsAudio for Reverb {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        if channel == 0 {
            self.left.transform_channel(channel, input_sample)
        } else {
            self.right.transform_channel(channel, input_sample)
        }
    }
}
impl Reverb {
    pub fn new_with(sample_rate: usize, params: ReverbParams, wet_dry_mix: f32) -> Self {
        // Thanks to https://basicsynth.com/ (page 133 of paperback) for
        // constants.
        Self {
            uid: Default::default(),
            params,
            wet_dry_mix,
            left: ReverbChannel::new_with(sample_rate, params, wet_dry_mix),
            right: ReverbChannel::new_with(sample_rate, params, wet_dry_mix),
        }
    }

    pub fn set_wet_dry_mix(&mut self, mix: f32) {
        self.wet_dry_mix = mix;
        self.left.set_wet_dry_mix(mix);
        self.right.set_wet_dry_mix(mix);
    }

    pub fn set_control_wet_dry_mix(&mut self, mix: groove_core::control::F32ControlValue) {
        self.set_wet_dry_mix(mix.0);
    }

    pub fn params(&self) -> ReverbParams {
        self.params
    }

    pub fn update(&mut self, message: ReverbParamsMessage) {
        self.params.update(message)
    }
}

#[derive(Debug)]
struct ReverbChannel {
    params: ReverbParams,

    // what percentage should be unprocessed. 0.0 = all effect. 0.0 = all
    // unchanged.
    //
    // TODO: maybe handle the wet/dry more centrally. It seems like it'll be
    // repeated a lot.
    wet_dry_mix: f32,

    recirc_delay_lines: Vec<RecirculatingDelayLine>,
    allpass_delay_lines: Vec<AllPassDelayLine>,
}
impl TransformsAudio for ReverbChannel {
    fn transform_channel(&mut self, _channel: usize, input_sample: Sample) -> Sample {
        let input_attenuated = input_sample * self.params.attenuation.value();
        let recirc_output = self.recirc_delay_lines[0].pop_output(input_attenuated)
            + self.recirc_delay_lines[1].pop_output(input_attenuated)
            + self.recirc_delay_lines[2].pop_output(input_attenuated)
            + self.recirc_delay_lines[3].pop_output(input_attenuated);
        let adl_0_out = self.allpass_delay_lines[0].pop_output(recirc_output);
        self.allpass_delay_lines[1].pop_output(adl_0_out) * self.wet_dry_mix
            + input_sample * (1.0 - self.wet_dry_mix)
    }
}
impl ReverbChannel {
    pub fn new_with(sample_rate: usize, params: ReverbParams, wet_dry_mix: f32) -> Self {
        // Thanks to https://basicsynth.com/ (page 133 of paperback) for
        // constants.
        Self {
            params,
            wet_dry_mix,
            recirc_delay_lines: vec![
                RecirculatingDelayLine::new_with(
                    sample_rate,
                    0.0297,
                    params.seconds(),
                    Normal::from(0.001),
                    Normal::from(1.0),
                ),
                RecirculatingDelayLine::new_with(
                    sample_rate,
                    0.0371,
                    params.seconds(),
                    Normal::from(0.001),
                    Normal::from(1.0),
                ),
                RecirculatingDelayLine::new_with(
                    sample_rate,
                    0.0411,
                    params.seconds(),
                    Normal::from(0.001),
                    Normal::from(1.0),
                ),
                RecirculatingDelayLine::new_with(
                    sample_rate,
                    0.0437,
                    params.seconds(),
                    Normal::from(0.001),
                    Normal::from(1.0),
                ),
            ],
            allpass_delay_lines: vec![
                AllPassDelayLine::new_with(
                    sample_rate,
                    0.09683,
                    0.0050,
                    Normal::from(0.001),
                    Normal::from(1.0),
                ),
                AllPassDelayLine::new_with(
                    sample_rate,
                    0.03292,
                    0.0017,
                    Normal::from(0.001),
                    Normal::from(1.0),
                ),
            ],
        }
    }

    pub fn set_wet_dry_mix(&mut self, mix: f32) {
        self.wet_dry_mix = mix;
    }
}

#[cfg(test)]
mod tests {
    use super::Reverb;
    use crate::{effects::ReverbParams, tests::DEFAULT_SAMPLE_RATE};
    use groove_core::{traits::TransformsAudio, Normal, Sample};

    #[test]
    fn reverb_dry_works() {
        let mut fx = Reverb::new_with(
            DEFAULT_SAMPLE_RATE,
            crate::effects::ReverbParams {
                attenuation: Normal::from(0.5),
                seconds: 1.5,
            },
            0.0,
        );
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
        let mut fx = Reverb::new_with(
            DEFAULT_SAMPLE_RATE,
            ReverbParams {
                attenuation: Normal::from(0.9),
                seconds: 0.5,
            },
            1.0,
        );
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
