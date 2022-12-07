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
    #[strum(serialize = "delay", serialize = "delay-seconds")]
    Attenuation,
}

/// Schroeder reverb. Uses four parallel recirculating delay lines feeding into
/// two serial all-pass delay lines.
#[derive(Debug, Default)]
pub(crate) struct Reverb {
    uid: usize,

    // what percentage should be unprocessed. 0.0 = all effect. 0.0 = all
    // unchanged.
    //
    // TODO: maybe handle the wet/dry more centrally. It seems like it'll be
    // repeated a lot.
    dry_pct: f32,
    attenuation: f32,
    recirc_delay_lines: Vec<RecirculatingDelayLine>,
    #[allow(dead_code)]
    allpass_delay_lines: Vec<AllPassDelayLine>,
}
impl IsEffect for Reverb {}
impl TransformsAudio for Reverb {
    fn transform_audio(&mut self, _clock: &Clock, input: MonoSample) -> MonoSample {
        let input = input * self.attenuation;
        let recirc_output = self.recirc_delay_lines[0].pop_output(input)
            + self.recirc_delay_lines[1].pop_output(input)
            + self.recirc_delay_lines[2].pop_output(input)
            + self.recirc_delay_lines[3].pop_output(input);
        // let adl_0_out = self.allpass_delay_lines[0].pop_output(recirc_output);
        // self.allpass_delay_lines[1].pop_output(adl_0_out);
        //
        // TODO: these lines are deadening the sound
        (1.0 - self.dry_pct) * recirc_output + self.dry_pct * input
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

    // pub fn attenuation(&self) -> f32 {
    //     self.attenuation
    // }

    pub fn set_attenuation(&mut self, attenuation: f32) {
        if attenuation != self.attenuation {
            self.attenuation = attenuation;
            // TODO regen
        }
    }
}

#[cfg(test)]
mod tests {

    // TODO
}
