use super::delay::{AllPassDelayLine, Delays, RecirculatingDelayLine};
use crate::{
    clock::Clock,
    common::F32ControlValue,
    common::Sample,
    traits::{Controllable, HasUid, IsEffect, TransformsAudio},
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
    fn transform_channel(
        &mut self,
        _clock: &Clock,
        _channel: usize,
        input_sample: crate::common::Sample,
    ) -> crate::common::Sample {
        let input_attenuated = input_sample * self.attenuation;
        let recirc_output = self.recirc_delay_lines[0].pop_output(input_attenuated)
            + self.recirc_delay_lines[1].pop_output(input_attenuated)
            + self.recirc_delay_lines[2].pop_output(input_attenuated)
            + self.recirc_delay_lines[3].pop_output(input_attenuated);
        let adl_0_out = self.allpass_delay_lines[0].pop_output(recirc_output);
        Sample::from(
            self.allpass_delay_lines[1].pop_output(adl_0_out) * self.wet_dry_mix
                + input_sample * (1.0 - self.wet_dry_mix),
        )
    }
}
impl Reverb {
    #[allow(dead_code)]
    fn new() -> Self {
        Self::default()
    }

    pub(crate) fn new_with(
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

    pub(crate) fn set_control_attenuation(&mut self, attenuation: F32ControlValue) {
        self.set_attenuation(attenuation.0);
    }

    pub fn set_wet_dry_mix(&mut self, mix: f32) {
        self.wet_dry_mix = mix;
    }

    pub(crate) fn set_control_wet_dry_mix(&mut self, mix: F32ControlValue) {
        self.set_wet_dry_mix(mix.0);
    }
}

#[cfg(test)]
mod tests {
    use super::Reverb;
    use crate::{common::Sample, traits::TransformsAudio, Clock};

    #[test]
    fn reverb_dry_works() {
        let mut clock = Clock::default();
        let mut fx = Reverb::new_with(clock.sample_rate(), 0.0, 0.5, 1.5);
        assert_eq!(
            fx.transform_channel(&clock, 0, Sample::from(0.8f32)),
            Sample::from(0.8f32)
        );
        clock.tick();
        assert_eq!(
            fx.transform_channel(&clock, 0, Sample::from(0.7f32)),
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
        let mut clock = Clock::default();
        let mut fx = Reverb::new_with(clock.sample_rate(), 1.0, 0.9, 0.5);
        assert_eq!(
            fx.transform_channel(&clock, 0, Sample::from(0.8)),
            Sample::SILENCE
        );
        clock.debug_set_seconds(0.5);
        let mut s = Sample::default();
        for _ in 0..44100 {
            s += fx.transform_channel(&clock, 0, Sample::SILENCE);
            clock.tick();
        }
        assert!(s != Sample::SILENCE);
    }
}
