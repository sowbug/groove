use crate::{
    clock::Clock,
    common::MonoSample,
    messages::GrooveMessage,
    traits::{HasUid, IsEffect, Updateable, TransformsAudio},
};

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
        let clock = Clock::new();
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
