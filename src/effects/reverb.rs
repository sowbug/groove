use super::delay::{AllPassDelayLine, Delays, RecirculatingDelayLine};
use crate::{
    clock::Clock,
    common::MonoSample,
    messages::EntityMessage,
    traits::{HasUid, IsEffect, Response, TransformsAudio, Updateable},
};
use strum_macros::{Display, EnumString, FromRepr};

#[derive(Display, Debug, EnumString, FromRepr)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum ReverbControlParams {
    #[strum(serialize = "attenuation")]
    Attenuation,

    #[strum(serialize = "dry", serialize = "dry-pct")]
    DryPct,
}

/// Schroeder reverb. Uses four parallel recirculating delay lines feeding into
/// a series of two all-pass delay lines.
#[derive(Debug, Default)]
pub(crate) struct Reverb {
    uid: usize,

    // How much the effect should attenuate the input.
    attenuation: f32,

    // what percentage should be unprocessed. 0.0 = all effect. 0.0 = all
    // unchanged.
    //
    // TODO: maybe handle the wet/dry more centrally. It seems like it'll be
    // repeated a lot.
    dry_pct: f32,
    recirc_delay_lines: Vec<RecirculatingDelayLine>,
    allpass_delay_lines: Vec<AllPassDelayLine>,
}
impl IsEffect for Reverb {}
impl TransformsAudio for Reverb {
    fn transform_audio(&mut self, _clock: &Clock, input: MonoSample) -> MonoSample {
        let input_attenuated = input * self.attenuation;
        let recirc_output = self.recirc_delay_lines[0].pop_output(input_attenuated)
            + self.recirc_delay_lines[1].pop_output(input_attenuated)
            + self.recirc_delay_lines[2].pop_output(input_attenuated)
            + self.recirc_delay_lines[3].pop_output(input_attenuated);
        let adl_0_out = self.allpass_delay_lines[0].pop_output(recirc_output);
        (1.0 - self.dry_pct) * self.allpass_delay_lines[1].pop_output(adl_0_out)
            + self.dry_pct * input
    }
}
impl Updateable for Reverb {
    type Message = EntityMessage;

    #[allow(unused_variables)]
    fn update(&mut self, clock: &Clock, message: Self::Message) -> Response<Self::Message> {
        match message {
            Self::Message::UpdateF32(param_id, value) => {
                self.set_indexed_param_f32(param_id, value);
            }
            _ => todo!(),
        }
        Response::none()
    }

    fn set_indexed_param_f32(&mut self, index: usize, value: f32) {
        if let Some(param) = ReverbControlParams::from_repr(index) {
            match param {
                ReverbControlParams::Attenuation => self.set_attenuation(value),
                ReverbControlParams::DryPct => self.set_dry_pct(value),
            }
        } else {
            todo!()
        }
    }
}
impl HasUid for Reverb {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}

impl Reverb {
    #[allow(dead_code)]
    fn new() -> Self {
        Self::default()
    }

    pub(crate) fn new_with(
        sample_rate: usize,
        dry_pct: f32,
        attenuation: f32,
        reverb_seconds: f32,
    ) -> Self {
        // Thanks to https://basicsynth.com/ (page 133 of paperback) for
        // constants.
        Self {
            uid: Default::default(),
            dry_pct,
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

    pub(crate) fn set_dry_pct(&mut self, dry_pct: f32) {
        self.dry_pct = dry_pct;
    }
}

#[cfg(test)]
mod tests {
    use super::Reverb;
    use crate::{common::MONO_SAMPLE_SILENCE, traits::TransformsAudio, Clock};

    #[test]
    fn reverb_dry_works() {
        let mut clock = Clock::default();
        let mut fx = Reverb::new_with(clock.sample_rate(), 1.0, 0.5, 1.5);
        assert_eq!(fx.transform_audio(&clock, 0.8), 0.8);
        clock.tick();
        assert_eq!(fx.transform_audio(&clock, 0.7), 0.7);
    }

    #[test]
    fn reverb_wet_works() {
        // This test is lame, because I can't think of a programmatic way to
        // test that reverb works. I observed that with the Schroeder reverb set
        // to 0.5 seconds, we start getting back nonzero samples (first
        // 0.47767496) at samples: 29079, seconds: 0.65938777. This doesn't look
        // wrong, but I couldn't have predicted that exact number.
        let mut clock = Clock::default();
        let mut fx = Reverb::new_with(clock.sample_rate(), 0.0, 0.9, 0.5);
        assert_eq!(fx.transform_audio(&clock, 0.8), 0.0);
        clock.debug_set_seconds(0.5);
        let mut s = MONO_SAMPLE_SILENCE;
        for _ in 0..44100 {
            s += fx.transform_audio(&clock, 0.0);
            clock.tick();
        }
        assert!(s != 0.0);
    }
}
