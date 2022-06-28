use std::{rc::Rc, cell::RefCell};

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

pub struct Quietener {
    source: Rc<RefCell<dyn DeviceTrait>>,
}
impl Quietener {
    pub fn new(source: Rc<RefCell<dyn DeviceTrait>>) -> Quietener {
        Quietener { source }
    }
}
// TODO(miket): idea: ticks are called only if the entity was asked for its sample, as a power optimization
impl DeviceTrait for Quietener {
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
        self.source.borrow().get_audio_sample() * 0.8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
