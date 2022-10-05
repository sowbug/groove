use std::{cell::RefCell, rc::Rc};

use crate::{
    clock::Clock,
    common::MonoSample,
    traits::{
        IsEffect, SinksAudio, SinksControl, SinksControlParam, SourcesAudio, TransformsAudio,
    },
};

#[derive(Debug)]
pub struct Gain {
    sources: Vec<Rc<RefCell<dyn SourcesAudio>>>,
    amount: f32,
}
impl IsEffect for Gain {}

impl Gain {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn new_with(amount: f32) -> Self {
        Self {
            amount,
            ..Default::default()
        }
    }
}
impl Default for Gain {
    fn default() -> Self {
        Self {
            sources: Vec::default(),
            amount: 1.0,
        }
    }
}
impl SinksAudio for Gain {
    fn sources(&self) -> &[Rc<RefCell<dyn SourcesAudio>>] {
        &self.sources
    }
    fn sources_mut(&mut self) -> &mut Vec<Rc<RefCell<dyn SourcesAudio>>> {
        &mut self.sources
    }
}
impl TransformsAudio for Gain {
    fn transform_audio(&mut self, input_sample: MonoSample) -> MonoSample {
        input_sample * self.amount
    }
}
impl SinksControl for Gain {
    fn handle_control(&mut self, _clock: &Clock, param: &SinksControlParam) {
        match param {
            SinksControlParam::Primary { value } => self.amount = *value,
            _ => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::traits::tests::{TestAlwaysLoudDevice, TestAlwaysSameLevelDevice};

    use super::*;

    #[test]
    fn test_gain_mainline() {
        let mut gain = Gain::new_with(1.1);
        gain.add_audio_source(Rc::new(RefCell::new(TestAlwaysLoudDevice::new())));
        assert_eq!(gain.source_audio(&Clock::new()), 1.1);
    }

    #[test]
    fn test_gain_pola() {
        // principle of least astonishment: does a default instance adhere?
        let mut gain = Gain::new();
        gain.add_audio_source(Rc::new(RefCell::new(TestAlwaysSameLevelDevice::new(0.888))));
        assert_eq!(gain.source_audio(&Clock::new()), 0.888);
    }
}
