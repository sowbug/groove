use std::marker::PhantomData;

use crate::{
    clock::Clock,
    common::MonoSample,
    messages::{EntityMessage, MessageBounds},
    traits::{HasUid, IsEffect, TransformsAudio, Updateable},
};
use strum_macros::{Display, EnumString, FromRepr};

#[derive(Display, Debug, EnumString, FromRepr)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum GainControlParams {
    Ceiling,
}

#[derive(Debug, Default)]
pub(crate) struct Gain<M: MessageBounds> {
    uid: usize,
    ceiling: f32,
    _phantom: PhantomData<M>,
}
impl<M: MessageBounds> IsEffect for Gain<M> {}
impl<M: MessageBounds> TransformsAudio for Gain<M> {
    fn transform_audio(&mut self, _clock: &Clock, input_sample: MonoSample) -> MonoSample {
        input_sample * self.ceiling
    }
}
impl<M: MessageBounds> Updateable for Gain<M> {
    default type Message = M;

    #[allow(unused_variables)]
    default fn update(
        &mut self,
        clock: &Clock,
        message: Self::Message,
    ) -> crate::traits::EvenNewerCommand<Self::Message> {
        crate::traits::EvenNewerCommand::none()
    }

    fn set_indexed_param_f32(&mut self, index: usize, value: f32) {
        if let Some(param) = GainControlParams::from_repr(index) {
            match param {
                GainControlParams::Ceiling => self.set_ceiling(value),
            }
        } else {
            todo!()
        }
    }
}
impl Updateable for Gain<EntityMessage> {
    type Message = EntityMessage;

    fn update(
        &mut self,
        _clock: &Clock,
        message: Self::Message,
    ) -> crate::traits::EvenNewerCommand<Self::Message> {
        match message {
            Self::Message::UpdateF32(param_id, value) => {
                self.set_indexed_param_f32(param_id, value);
            }
            Self::Message::UpdateParam0F32(value) => {
                self.set_indexed_param_f32(GainControlParams::Ceiling as usize, value);
            }
            Self::Message::UpdateParam0U8(value) => {
                self.set_indexed_param_f32(
                    GainControlParams::Ceiling as usize,
                    value as f32 / 100.0,
                );
            }
            _ => todo!(),
        }
        crate::traits::EvenNewerCommand::none()
    }
}
impl<M: MessageBounds> HasUid for Gain<M> {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}

impl<M: MessageBounds> Gain<M> {
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
        let mut gain = Gain::<EntityMessage>::new_with(1.1);
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
