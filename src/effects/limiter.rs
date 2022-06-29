use crate::backend::devices::DeviceTrait;
use std::{cell::RefCell, rc::Rc};

pub struct Limiter {
    source: Rc<RefCell<dyn DeviceTrait>>,
    min: f32,
    max: f32,
}
impl Limiter {
    pub fn new(source: Rc<RefCell<dyn DeviceTrait>>, min: f32, max: f32) -> Limiter {
        Limiter { source, min, max }
    }
}
impl DeviceTrait for Limiter {
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
        self.source
            .borrow()
            .get_audio_sample()
            .clamp(self.min, self.max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::tests::TestAlwaysTooLoudDevice;

    #[test]
    fn test_limiter_mainline() {
        const MAX: f32 = 0.9;
        let too_loud = Rc::new(RefCell::new(TestAlwaysTooLoudDevice {}));
        let limiter = Limiter::new(too_loud.clone(), 0.0, MAX);

        assert_eq!(too_loud.borrow().get_audio_sample(), 1.1);
        assert_eq!(limiter.get_audio_sample(), MAX);
    }
}
