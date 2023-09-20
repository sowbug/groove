// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::delay::{AllPassDelayLine, Delays, RecirculatingDelayLine};
use ensnare::prelude::*;
use groove_core::{
    time::SampleRate,
    traits::{Configurable, Serializable, TransformsAudio},
};
use groove_proc_macros::{Control, IsEffect, Params, Uid};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

/// Schroeder reverb. Uses four parallel recirculating delay lines feeding into
/// a series of two all-pass delay lines.
#[derive(Debug, Control, IsEffect, Params, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Reverb {
    uid: groove_core::Uid,

    #[cfg_attr(feature = "serialization", serde(skip))]
    sample_rate: SampleRate,

    /// How much the effect should attenuate the input.
    #[control]
    #[params]
    attenuation: Normal,

    #[control]
    #[params]
    seconds: ParameterType,

    #[cfg_attr(feature = "serialization", serde(skip))]
    channels: [ReverbChannel; 2],
}
impl Serializable for Reverb {}
impl Configurable for Reverb {
    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.channels[0].update_sample_rate(sample_rate);
        self.channels[1].update_sample_rate(sample_rate);
    }
}
impl TransformsAudio for Reverb {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        self.channels[channel].transform_channel(channel, input_sample)
    }
}
impl Reverb {
    pub fn new_with(params: &ReverbParams) -> Self {
        // Thanks to https://basicsynth.com/ (page 133 of paperback) for
        // constants.
        Self {
            uid: Default::default(),
            sample_rate: Default::default(),
            attenuation: params.attenuation(),
            seconds: params.seconds(),
            channels: [
                ReverbChannel::new_with(params),
                ReverbChannel::new_with(params),
            ],
        }
    }

    pub fn attenuation(&self) -> Normal {
        self.attenuation
    }

    pub fn set_attenuation(&mut self, attenuation: Normal) {
        self.attenuation = attenuation;
        self.channels
            .iter_mut()
            .for_each(|c| c.set_attenuation(attenuation));
    }

    pub fn seconds(&self) -> f64 {
        self.seconds
    }

    pub fn set_seconds(&mut self, seconds: ParameterType) {
        self.seconds = seconds;
        self.channels
            .iter_mut()
            .for_each(|c| c.set_seconds(seconds));
    }
}

#[derive(Debug, Default)]
struct ReverbChannel {
    attenuation: Normal,

    recirc_delay_lines: Vec<RecirculatingDelayLine>,
    allpass_delay_lines: Vec<AllPassDelayLine>,
}
impl TransformsAudio for ReverbChannel {
    fn transform_channel(&mut self, _channel: usize, input_sample: Sample) -> Sample {
        let input_attenuated = input_sample * self.attenuation.value();
        let recirc_output = self.recirc_delay_lines[0].pop_output(input_attenuated)
            + self.recirc_delay_lines[1].pop_output(input_attenuated)
            + self.recirc_delay_lines[2].pop_output(input_attenuated)
            + self.recirc_delay_lines[3].pop_output(input_attenuated);
        let adl_0_out = self.allpass_delay_lines[0].pop_output(recirc_output);
        self.allpass_delay_lines[1].pop_output(adl_0_out)
    }
}
impl Serializable for ReverbChannel {}
impl Configurable for ReverbChannel {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.recirc_delay_lines
            .iter_mut()
            .for_each(|r| r.update_sample_rate(sample_rate));
        self.allpass_delay_lines
            .iter_mut()
            .for_each(|r| r.update_sample_rate(sample_rate));
    }
}
impl ReverbChannel {
    pub fn new_with(params: &ReverbParams) -> Self {
        // Thanks to https://basicsynth.com/ (page 133 of paperback) for
        // constants.
        Self {
            attenuation: params.attenuation(),
            recirc_delay_lines: Self::instantiate_recirc_delay_lines(params.seconds()),
            allpass_delay_lines: Self::instantiate_allpass_delay_lines(),
        }
    }

    fn set_attenuation(&mut self, attenuation: Normal) {
        self.attenuation = attenuation;
    }

    fn set_seconds(&mut self, seconds: ParameterType) {
        self.recirc_delay_lines = Self::instantiate_recirc_delay_lines(seconds);
    }

    fn instantiate_recirc_delay_lines(seconds: ParameterType) -> Vec<RecirculatingDelayLine> {
        vec![
            RecirculatingDelayLine::new_with(
                0.0297,
                seconds,
                Normal::from(0.001),
                Normal::from(1.0),
            ),
            RecirculatingDelayLine::new_with(
                0.0371,
                seconds,
                Normal::from(0.001),
                Normal::from(1.0),
            ),
            RecirculatingDelayLine::new_with(
                0.0411,
                seconds,
                Normal::from(0.001),
                Normal::from(1.0),
            ),
            RecirculatingDelayLine::new_with(
                0.0437,
                seconds,
                Normal::from(0.001),
                Normal::from(1.0),
            ),
        ]
    }

    fn instantiate_allpass_delay_lines() -> Vec<AllPassDelayLine> {
        vec![
            AllPassDelayLine::new_with(0.09683, 0.0050, Normal::from(0.001), Normal::from(1.0)),
            AllPassDelayLine::new_with(0.03292, 0.0017, Normal::from(0.001), Normal::from(1.0)),
        ]
    }
}

#[cfg(feature = "egui-framework")]
mod gui {
    use super::Reverb;
    use eframe::egui::Ui;
    use groove_core::traits::{gui::Displays, HasUid};

    impl Displays for Reverb {
        fn ui(&mut self, ui: &mut Ui) -> eframe::egui::Response {
            ui.label(self.name())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Reverb;
    use crate::{effects::ReverbParams, tests::DEFAULT_SAMPLE_RATE};
    use ensnare::core::{Normal, Sample};
    use groove_core::{
        time::SampleRate,
        traits::{Configurable, TransformsAudio},
    };

    #[test]
    fn reverb_does_anything_at_all() {
        // This test is lame, because I can't think of a programmatic way to
        // test that reverb works. I observed that with the Schroeder reverb set
        // to 0.5 seconds, we start getting back nonzero samples (first
        // 0.47767496) at samples: 29079, seconds: 0.65938777. This doesn't look
        // wrong, but I couldn't have predicted that exact number.
        let mut fx = Reverb::new_with(&ReverbParams {
            attenuation: Normal::from(0.9),
            seconds: 0.5,
        });
        fx.update_sample_rate(SampleRate::DEFAULT);
        assert_eq!(fx.transform_channel(0, Sample::from(0.8)), Sample::SILENCE);
        let mut s = Sample::default();
        for _ in 0..DEFAULT_SAMPLE_RATE {
            s += fx.transform_channel(0, Sample::SILENCE);
        }
        assert!(s != Sample::SILENCE);

        // TODO: this test might not do anything. I refactored it in a hurry and
        // took something that looked critical (skipping the clock to 0.5
        // seconds) out of it, but it still passed. I might not actually be
        // testing anything useful.
    }
}
