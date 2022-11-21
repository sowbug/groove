use crate::{
    clock::Clock,
    common::{rrc, MonoSample, Rrc, Ww},
    messages::GrooveMessage,
    traits::{
        HasUid, NewIsEffect, NewUpdateable, SourcesAudio, TransformsAudio,
    },
};

#[derive(Debug, Default)]
pub(crate) struct Gain {
    uid: usize,



    ceiling: f32,
}
impl NewIsEffect for Gain {}
impl TransformsAudio for Gain {
    fn transform_audio(&mut self, _clock: &Clock, input_sample: MonoSample) -> MonoSample {
        input_sample * self.ceiling
    }
}
impl NewUpdateable for Gain {
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
        clock::Clock,
        messages::tests::TestMessage,
        utils::tests::{TestAudioSourceAlwaysLoud, TestAudioSourceOneLevel},
    };

    #[test]
    fn test_gain_mainline() {
        let mut gain = Gain::new_with(1.1);
        let clock = Clock::new();
        assert_eq!(
            gain.transform_audio(
                &clock,
                TestAudioSourceAlwaysLoud::<TestMessage>::new().source_audio(&clock)
            ),
            1.1
        );
    }
}
