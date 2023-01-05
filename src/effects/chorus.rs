use super::delay::{DelayLine, Delays};
use crate::{
    clock::Clock,
    common::MonoSample,
    messages::EntityMessage,
    traits::{
        Controllable, F32ControlValue, HasUid, IsEffect, Response, TransformsAudio, Updateable,
    },
};
use groove_macros::{Control, Uid};
use std::str::FromStr;
use strum_macros::{Display, EnumString, FromRepr};

/// Schroeder reverb. Uses four parallel recirculating delay lines feeding into
/// a series of two all-pass delay lines.
#[derive(Control, Debug, Default, Uid)]
pub struct Chorus {
    uid: usize,

    voice_count: usize,
    delay_factor: usize,

    // what percentage of the output should be processed. 0.0 = all dry (no
    // effect). 1.0 = all wet (100% effect).
    //
    // TODO: maybe handle the wet/dry more centrally. It seems like it'll be
    // repeated a lot.
    #[controllable]
    dry_wet_mix: f32,

    delay: DelayLine,
}
impl IsEffect for Chorus {}
impl TransformsAudio for Chorus {
    fn transform_audio(&mut self, _clock: &Clock, input: MonoSample) -> MonoSample {
        let index_offset = self.delay_factor / self.voice_count;
        let mut sum = self.delay.pop_output(input);
        for i in 1..self.voice_count as isize {
            sum += self.delay.peek_indexed_output(i * index_offset as isize);
        }

        self.dry_wet_mix * sum / self.voice_count as MonoSample + (1.0 - self.dry_wet_mix) * input
    }
}
impl Updateable for Chorus {
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

    fn param_id_for_name(&self, name: &str) -> usize {
        if let Ok(param) = ChorusControlParams::from_str(name) {
            param as usize
        } else {
            usize::MAX
        }
    }

    fn set_indexed_param_f32(&mut self, index: usize, value: f32) {
        if let Some(param) = ChorusControlParams::from_repr(index) {
            match param {
                ChorusControlParams::DryWetMix => self.set_dry_wet_mix(value),
            }
        } else {
            todo!()
        }
    }
}

impl Chorus {
    #[allow(dead_code)]
    fn new() -> Self {
        Self::default()
    }

    pub(crate) fn new_with(
        sample_rate: usize,
        dry_pct: f32,
        voice_count: usize,
        delay_factor: usize,
    ) -> Self {
        // TODO: the delay_seconds param feels like a hack
        Self {
            uid: Default::default(),
            dry_wet_mix: dry_pct,
            voice_count,
            delay_factor,
            delay: DelayLine::new_with(sample_rate, delay_factor as f32 / sample_rate as f32, 1.0),
        }
    }

    pub(crate) fn set_dry_wet_mix(&mut self, wet_pct: f32) {
        self.dry_wet_mix = wet_pct;
    }

    pub(crate) fn set_control_dry_wet_mix(&mut self, value: F32ControlValue) {
        self.set_dry_wet_mix(value.0);
    }
}

#[cfg(test)]
mod tests {
    //TODO
}
