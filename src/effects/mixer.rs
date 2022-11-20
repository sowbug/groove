use crate::{
    clock::Clock,
    common::{rrc, rrc_downgrade, MonoSample, Rrc, Ww},
    traits::{
        HasOverhead, HasUid, MessageBounds, NewIsEffect, NewUpdateable, Overhead, SourcesAudio,
        TransformsAudio,
    },
};

#[derive(Clone, Debug, Default)]
pub struct Mixer<M: MessageBounds> {
    uid: usize,
    pub(crate) me: Ww<Self>,
    overhead: Overhead,
}
impl<M: MessageBounds> NewIsEffect for Mixer<M> {}
impl<M: MessageBounds> TransformsAudio for Mixer<M> {
    fn transform_audio(&mut self, _clock: &Clock, input_sample: MonoSample) -> MonoSample {
        input_sample
    }
}
impl<M: MessageBounds> NewUpdateable for Mixer<M> {
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
    pub fn new_wrapped() -> Rrc<Self> {
        let wrapped = rrc(Self::new());
        wrapped.borrow_mut().me = rrc_downgrade(&wrapped);
        wrapped
    }
}
impl<M: MessageBounds> HasOverhead for Mixer<M> {
    fn overhead(&self) -> &Overhead {
        &self.overhead
    }

    fn overhead_mut(&mut self) -> &mut Overhead {
        &mut self.overhead
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
