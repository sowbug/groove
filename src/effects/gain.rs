use crate::backend::devices::DeviceTrait;
use std::{cell::RefCell, rc::Rc};

pub struct Gain {
    source: Rc<RefCell<dyn DeviceTrait>>,
    amount: f32,
}
impl Gain {
    pub fn new(source: Rc<RefCell<dyn DeviceTrait>>, amount: f32) -> Self {
        Self { source, amount }
    }
}
impl DeviceTrait for Gain {
    fn sources_audio(&self) -> bool {
        true
    }
    fn sinks_audio(&self) -> bool {
        true
    }
    fn add_audio_source(&mut self, source: Rc<RefCell<dyn DeviceTrait>>) {
        self.source = source;
    }
    fn get_audio_sample(&self) -> f32 {
        self.source.borrow().get_audio_sample() * self.amount
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::tests::TestAlwaysLoudDevice;

    #[test]
    fn test_gain_mainline() {
        let loud = Rc::new(RefCell::new(TestAlwaysLoudDevice {}));
        let gain = Gain::new(loud, 1.1);
        assert_eq!(gain.get_audio_sample(), 1.1);
    }
}
