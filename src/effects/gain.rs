use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use crate::{
    common::{MonoSample, W, WW},
    traits::{IsEffect, SinksAudio, SourcesAudio, TransformsAudio},
};

#[derive(Debug)]
pub struct Gain {
    pub(crate) me: WW<Self>,
    sources: Vec<W<dyn SourcesAudio>>,
    ceiling: f32,
}
impl IsEffect for Gain {}

impl Gain {
    pub(crate) const CONTROL_PARAM_CEILING: &str = "ceiling";

    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    pub fn new_wrapped() -> W<Self> {
        // TODO: Rc::new_cyclic() should make this easier, but I couldn't get the syntax right.
        // https://doc.rust-lang.org/std/rc/struct.Rc.html#method.new_cyclic

        let wrapped = Rc::new(RefCell::new(Self::new()));
        wrapped.borrow_mut().me = Rc::downgrade(&wrapped);
        wrapped
    }

    pub fn new_with(ceiling: f32) -> Self {
        Self {
            ceiling,
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    pub fn new_wrapped_with(ceiling: f32) -> W<Self> {
        // TODO: Rc::new_cyclic() should make this easier, but I couldn't get the syntax right.
        // https://doc.rust-lang.org/std/rc/struct.Rc.html#method.new_cyclic

        let wrapped = Rc::new(RefCell::new(Self::new_with(ceiling)));
        wrapped.borrow_mut().me = Rc::downgrade(&wrapped);
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
impl Default for Gain {
    fn default() -> Self {
        Self {
            me: Weak::new(),
            sources: Vec::default(),
            ceiling: 1.0,
        }
    }
}
impl SinksAudio for Gain {
    fn sources(&self) -> &[W<dyn SourcesAudio>] {
        &self.sources
    }
    fn sources_mut(&mut self) -> &mut Vec<W<dyn SourcesAudio>> {
        &mut self.sources
    }
}
impl TransformsAudio for Gain {
    fn transform_audio(&mut self, input_sample: MonoSample) -> MonoSample {
        input_sample * self.ceiling
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        clock::Clock,
        utils::tests::{TestAudioSourceAlwaysLoud, TestAudioSourceAlwaysSameLevel},
    };

    use super::*;

    #[test]
    fn test_gain_mainline() {
        let mut gain = Gain::new_with(1.1);
        gain.add_audio_source(Rc::new(RefCell::new(TestAudioSourceAlwaysLoud::new())));
        assert_eq!(gain.source_audio(&Clock::new()), 1.1);
    }

    #[test]
    fn test_gain_pola() {
        // principle of least astonishment: does a default instance adhere?
        let mut gain = Gain::new();
        gain.add_audio_source(Rc::new(RefCell::new(TestAudioSourceAlwaysSameLevel::new(
            0.888,
        ))));
        assert_eq!(gain.source_audio(&Clock::new()), 0.888);
    }
}
