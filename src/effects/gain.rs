use crate::{
    clock::Clock,
    common::MonoSample,
    messages::GrooveMessage,
    traits::{HasUid, IsEffect, TransformsAudio, Updateable},
};
use strum_macros::{Display, EnumString, FromRepr};

#[derive(Display, Debug, EnumString, FromRepr)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum GainControlParams {
    Ceiling,
}

#[derive(Debug, Default)]
pub(crate) struct Gain {
    uid: usize,

    ceiling: f32,
}
impl IsEffect for Gain {}
impl TransformsAudio for Gain {
    fn transform_audio(&mut self, _clock: &Clock, input_sample: MonoSample) -> MonoSample {
        input_sample * self.ceiling
    }
}
impl Updateable for Gain {
    type Message = GrooveMessage;

    fn update(
        &mut self,
        _clock: &Clock,
        message: Self::Message,
    ) -> crate::traits::EvenNewerCommand<Self::Message> {
        match message {
            Self::Message::UpdateF32(param_id, value) => {
                if let Some(param) = GainControlParams::from_repr(param_id) {
                    match param {
                        GainControlParams::Ceiling => self.set_ceiling(value),
                    }
                }
            }
            _ => todo!(),
        }
        crate::traits::EvenNewerCommand::none()
    }
}
impl HasUid for Gain {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}

impl Gain {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            ceiling: 1.0,
            ..Default::default()
        }
    }

    pub fn new_with(ceiling: f32) -> Self {
        Self {
            ceiling,
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    pub fn ceiling(&self) -> f32 {
        self.ceiling
    }

    #[allow(dead_code)]
    pub fn set_ceiling(&mut self, pct: f32) {
        self.ceiling = pct;
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{
        clock::Clock, messages::tests::TestMessage, traits::SourcesAudio, utils::AudioSource,
    };

    #[test]
    fn test_gain_mainline() {
        let mut gain = Gain::new_with(1.1);
        let clock = Clock::default();
        assert_eq!(
            gain.transform_audio(
                &clock,
                AudioSource::<TestMessage>::new_with(AudioSource::<TestMessage>::LOUD)
                    .source_audio(&clock)
            ),
            1.1
        );
    }
}
