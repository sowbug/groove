use crate::{
    clock::Clock,
    common::F32ControlValue,
    common::{Normal, Sample},
    messages::MessageBounds,
    traits::{Controllable, HasUid, IsEffect, TransformsAudio, Updateable},
};
use groove_macros::{Control, Uid};
use std::{marker::PhantomData, str::FromStr};
use strum_macros::{Display, EnumString, FromRepr};

#[derive(Control, Debug, Default, Uid)]
pub struct Gain<M: MessageBounds> {
    uid: usize,

    #[controllable]
    ceiling: Normal,

    _phantom: PhantomData<M>,
}
impl<M: MessageBounds> IsEffect for Gain<M> {}
impl<M: MessageBounds> TransformsAudio for Gain<M> {
    fn transform_channel(
        &mut self,
        _clock: &Clock,
        _channel: usize,
        input_sample: crate::common::Sample,
    ) -> crate::common::Sample {
        Sample(input_sample.0 * self.ceiling.value())
    }
}
impl<M: MessageBounds> Updateable for Gain<M> {
    type Message = M;
}
impl<M: MessageBounds> Gain<M> {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            ceiling: Normal::new(1.0),
            ..Default::default()
        }
    }

    pub fn new_with(ceiling: Normal) -> Self {
        Self {
            ceiling,
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    pub fn ceiling(&self) -> Normal {
        self.ceiling
    }

    pub fn set_ceiling(&mut self, ceiling: Normal) {
        self.ceiling = ceiling;
    }

    pub fn set_control_ceiling(&mut self, value: F32ControlValue) {
        self.set_ceiling(Normal::new_from_f32(value.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        clock::Clock, messages::tests::TestMessage, traits::SourcesAudio, utils::AudioSource,
        EntityMessage, StereoSample,
    };

    #[test]
    fn test_gain_mainline() {
        let mut gain = Gain::<EntityMessage>::new_with(Normal::new(0.5));
        let clock = Clock::default();
        assert_eq!(
            gain.transform_audio(
                &clock,
                AudioSource::<TestMessage>::new_with(AudioSource::<TestMessage>::LOUD)
                    .source_stereo_audio(&clock)
            ),
            StereoSample::from(0.5)
        );
    }
}
