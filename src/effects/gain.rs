use crate::{
    common::{rrc, rrc_downgrade, MonoSample, Rrc, Ww},
    traits::{IsEffect, IsMutable, SinksAudio, SourcesAudio, TransformsAudio},
};


#[derive(Debug, Default)]
pub struct Gain {
    pub(crate) me: Ww<Self>,
    sources: Vec<Ww<dyn SourcesAudio>>,
    is_muted: bool,
    ceiling: f32,
}
impl IsEffect for Gain {}

impl Gain {
    pub(crate) const CONTROL_PARAM_CEILING: &str = "ceiling";

    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            ceiling: 1.0,
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    pub fn new_wrapped() -> Rrc<Self> {
        // TODO: Rc::new_cyclic() should make this easier, but I couldn't get the syntax right.
        // https://doc.rust-lang.org/std/rc/struct.Rc.html#method.new_cyclic

        let wrapped = rrc(Self::new());
        wrapped.borrow_mut().me = rrc_downgrade(&wrapped);
        wrapped
    }

    pub fn new_with(ceiling: f32) -> Self {
        Self {
            ceiling,
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    pub fn new_wrapped_with(ceiling: f32) -> Rrc<Self> {
        // TODO: Rc::new_cyclic() should make this easier, but I couldn't get the syntax right.
        // https://doc.rust-lang.org/std/rc/struct.Rc.html#method.new_cyclic

        let wrapped = rrc(Self::new_with(ceiling));
        wrapped.borrow_mut().me = rrc_downgrade(&wrapped);
        wrapped
    }

    #[allow(dead_code)]
    pub fn level(&self) -> f32 {
        self.ceiling
    }

    #[allow(dead_code)]
    pub fn set_level(&mut self, level: f32) {
        self.ceiling = level;
    }
}
impl SinksAudio for Gain {
    fn sources(&self) -> &[Ww<dyn SourcesAudio>] {
        &self.sources
    }
    fn sources_mut(&mut self) -> &mut Vec<Ww<dyn SourcesAudio>> {
        &mut self.sources
    }
}
impl TransformsAudio for Gain {
    fn transform_audio(&mut self, input_sample: MonoSample) -> MonoSample {
        input_sample * self.ceiling
    }
}
impl IsMutable for Gain {
    fn is_muted(&self) -> bool {
        self.is_muted
    }

    fn set_muted(&mut self, is_muted: bool) {
        self.is_muted = is_muted;
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{
        clock::Clock,
        utils::tests::{TestAudioSourceAlwaysLoud, TestAudioSourceAlwaysSameLevel},
    };

    #[test]
    fn test_gain_mainline() {
        let mut gain = Gain::new_with(1.1);
        let source = rrc(TestAudioSourceAlwaysLoud::new());
        let source = rrc_downgrade(&source);
        gain.add_audio_source(source);
        assert_eq!(gain.source_audio(&Clock::new()), 1.1);
    }

    #[test]
    fn test_gain_pola() {
        // principle of least astonishment: does a default instance adhere?
        let mut gain = Gain::new();
        let source = rrc(TestAudioSourceAlwaysSameLevel::new(0.888));
        let source = rrc_downgrade(&source);
        gain.add_audio_source(source);
        assert_eq!(gain.source_audio(&Clock::new()), 0.888);
    }
}
