use std::marker::PhantomData;

use crate::{
    clock::Clock,
    common::MonoSample,
    messages::MessageBounds,
    traits::{HasUid, IsEffect, Updateable, TransformsAudio},
};

#[derive(Clone, Debug, Default)]
pub struct Mixer<M: MessageBounds> {
    uid: usize,

    _phantom: PhantomData<M>,
}
impl<M: MessageBounds> IsEffect for Mixer<M> {}
impl<M: MessageBounds> TransformsAudio for Mixer<M> {
    fn transform_audio(&mut self, _clock: &Clock, input_sample: MonoSample) -> MonoSample {
        input_sample
    }
}
impl<M: MessageBounds> Updateable for Mixer<M> {
    type Message = M;
}
impl<M: MessageBounds> HasUid for Mixer<M> {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}
impl<M: MessageBounds> Mixer<M> {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_mixer_mainline() {
        // This could be replaced with a test, elsewhere, showing that
        // Orchestrator's gather_audio() method can gather audio.
    }
}
