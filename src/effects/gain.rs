use crate::{
    clock::Clock,
    common::F32ControlValue,
    common::MonoSample,
    messages::{EntityMessage, MessageBounds},
    traits::{Controllable, HasUid, IsEffect, Response, TransformsAudio, Updateable},
};
use groove_macros::{Control, Uid};
use std::{marker::PhantomData, str::FromStr};
use strum_macros::{Display, EnumString, FromRepr};

#[derive(Control, Debug, Default, Uid)]
pub struct Gain<M: MessageBounds> {
    uid: usize,

    #[controllable]
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
    default fn update(&mut self, clock: &Clock, message: Self::Message) -> Response<Self::Message> {
        Response::none()
    }
}
impl Updateable for Gain<EntityMessage> {
    type Message = EntityMessage;

    fn update(&mut self, _clock: &Clock, message: Self::Message) -> Response<Self::Message> {
        match message {
            EntityMessage::HSliderInt(value) => {
                self.set_control_ceiling(F32ControlValue(value.as_f32()));
            }
            _ => todo!(),
        }
        Response::none()
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

    pub fn set_ceiling(&mut self, pct: f32) {
        self.ceiling = pct;
    }

    pub fn set_control_ceiling(&mut self, value: F32ControlValue) {
        self.set_ceiling(value.0);
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
