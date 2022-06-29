use std::{cell::RefCell, rc::Rc};

use super::devices::DeviceTrait;

pub struct Mixer {
    sources: Vec<Rc<RefCell<dyn DeviceTrait>>>,
}

impl Mixer {
    pub fn new() -> Mixer {
        Mixer {
            sources: Vec::new(),
        }
    }
}
impl DeviceTrait for Mixer {
    fn sources_audio(&self) -> bool {
        true
    }
    fn sinks_audio(&self) -> bool {
        true
    }
    fn add_audio_source(&mut self, audio_instrument: Rc<RefCell<dyn DeviceTrait>>) {
        self.sources.push(audio_instrument);
    }
    fn get_audio_sample(&self) -> f32 {
        let mut sample: f32 = 0.;
        for i in self.sources.clone() {
            let weight: f32 = 1. / self.sources.len() as f32;
            sample += i.borrow().get_audio_sample() * weight;
        }
        sample
    }
}

pub struct Gain {
    source: Rc<RefCell<dyn DeviceTrait>>,
    amount: f32,
}
impl Gain {
    pub fn new(source: Rc<RefCell<dyn DeviceTrait>>, amount: f32) -> Gain {
        Gain { source, amount }
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

    struct TestAlwaysTooLoudDevice {}
    impl DeviceTrait for TestAlwaysTooLoudDevice {
        fn get_audio_sample(&self) -> f32 {
            1.1
        }
    }

    struct TestAlwaysLoudDevice {}
    impl DeviceTrait for TestAlwaysLoudDevice {
        fn get_audio_sample(&self) -> f32 {
            1.
        }
    }

    struct TestAlwaysSilentDevice {}
    impl DeviceTrait for TestAlwaysSilentDevice {
        fn get_audio_sample(&self) -> f32 {
            0.
        }
    }

    #[test]
    fn test_mixer_mainline() {
        let mut mixer = Mixer::new();

        // Nothing
        assert_eq!(mixer.get_audio_sample(), 0.);

        // One always-loud
        mixer.add_audio_source(Rc::new(RefCell::new(TestAlwaysLoudDevice {})));
        assert_eq!(mixer.get_audio_sample(), 1.);

        // One always-loud and one always-quiet
        mixer.add_audio_source(Rc::new(RefCell::new(TestAlwaysSilentDevice {})));
        assert_eq!(mixer.get_audio_sample(), 0.5);
    }

    #[test]
    fn test_gain_mainline() {
        let loud = Rc::new(RefCell::new(TestAlwaysLoudDevice {}));
        let gain = Gain::new(loud, 1.1);
        assert_eq!(gain.get_audio_sample(), 1.1);
    }

    #[test]
    fn test_limiter_mainline() {
        const MAX: f32 = 0.9;
        let too_loud = Rc::new(RefCell::new(TestAlwaysTooLoudDevice {}));
        let limiter = Limiter::new(too_loud.clone(), 0.0, MAX);

        assert_eq!(too_loud.borrow().get_audio_sample(), 1.1);
        assert_eq!(limiter.get_audio_sample(), MAX);
    }
}
