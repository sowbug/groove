use super::traits::DeviceTrait;
use crate::{
    common::MonoSample,
    primitives::{
        self,
        filter::{MiniFilter2, MiniFilter2Type},
        gain::MiniGain,
        limiter::MiniLimiter,
        EffectTrait,
    },
};
use std::{cell::RefCell, rc::Rc};

fn add_sources(sources: &Vec<Rc<RefCell<dyn DeviceTrait>>>) -> MonoSample {
    sources
        .iter()
        .map(|s| s.borrow_mut().get_audio_sample())
        .sum()
}

pub struct Limiter {
    sources: Vec<Rc<RefCell<dyn DeviceTrait>>>,
    effect: MiniLimiter,
}

impl Limiter {
    pub fn new_with_params(min: MonoSample, max: MonoSample) -> Self {
        Self {
            sources: Vec::new(),
            effect: MiniLimiter::new(min, max),
        }
    }

    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::new_with_params(0.0, 1.0)
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
        self.sources.push(source);
    }
    fn get_audio_sample(&mut self) -> MonoSample {
        self.effect.process(add_sources(&self.sources))
    }
}

#[derive(Default)]
pub struct Gain {
    sources: Vec<Rc<RefCell<dyn DeviceTrait>>>,
    effect: MiniGain,
}

impl Gain {
    pub fn new_with_params(amount: f32) -> Self {
        Self {
            sources: Vec::new(),
            effect: MiniGain::new(amount), // TODO: consider new_with_params() convention
        }
    }

    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::new_with_params(1.0)
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
        self.sources.push(source);
    }
    fn get_audio_sample(&mut self) -> MonoSample {
        self.effect.process(add_sources(&self.sources))
    }
}

pub struct Bitcrusher {
    sources: Vec<Rc<RefCell<dyn DeviceTrait>>>,
    effect: primitives::bitcrusher::Bitcrusher,
    time_seconds: f32,
}

impl Bitcrusher {
    pub fn new_with_params(bits_to_crush: u8) -> Self {
        Self {
            sources: Vec::new(),
            effect: primitives::bitcrusher::Bitcrusher::new(bits_to_crush),
            time_seconds: 0.0,
        }
    }
    pub fn new() -> Self {
        Self::new_with_params(8)
    }
}

impl DeviceTrait for Bitcrusher {
    fn sources_audio(&self) -> bool {
        true
    }

    fn sinks_audio(&self) -> bool {
        true
    }

    fn add_audio_source(&mut self, source: Rc<RefCell<dyn DeviceTrait>>) {
        self.sources.push(source);
    }

    fn tick(&mut self, clock: &primitives::clock::Clock) -> bool {
        self.time_seconds = clock.seconds;
        true
    }

    fn get_audio_sample(&mut self) -> MonoSample {
        self.effect
            .process(add_sources(&self.sources), self.time_seconds)
    }
}

#[allow(dead_code)]
pub struct Filter {
    sources: Vec<Rc<RefCell<dyn DeviceTrait>>>,
    effect: MiniFilter2,

    filter_type: MiniFilter2Type,
}

impl Filter {
    fn inner_new_filter(ft: &MiniFilter2Type) -> Self {
        Self {
            sources: Vec::new(),
            effect: MiniFilter2::new(ft),
            filter_type: *ft,
        }
    }
    pub fn new_low_pass_12db(sample_rate: usize, cutoff: f32, q: f32) -> Self {
        Self::inner_new_filter(&MiniFilter2Type::LowPass {
            sample_rate,
            cutoff,
            q,
        })
    }
    pub fn new_high_pass_12db(sample_rate: usize, cutoff: f32, q: f32) -> Self {
        Self::inner_new_filter(&MiniFilter2Type::HighPass {
            sample_rate,
            cutoff,
            q,
        })
    }
    pub fn new_band_pass_12db(sample_rate: usize, cutoff: f32, bandwidth: f32) -> Self {
        Self::inner_new_filter(&MiniFilter2Type::BandPass {
            sample_rate,
            cutoff,
            bandwidth,
        })
    }
    pub fn new_band_stop_12db(sample_rate: usize, cutoff: f32, bandwidth: f32) -> Self {
        Self::inner_new_filter(&MiniFilter2Type::BandStop {
            sample_rate,
            cutoff,
            bandwidth,
        })
    }
    pub fn new_all_pass_12db(sample_rate: usize, cutoff: f32, q: f32) -> Self {
        Self::inner_new_filter(&MiniFilter2Type::AllPass {
            sample_rate,
            cutoff,
            q,
        })
    }
    pub fn new_peaking_eq_12db(sample_rate: usize, cutoff: f32, db_gain: f32) -> Self {
        Self::inner_new_filter(&MiniFilter2Type::PeakingEq {
            sample_rate,
            cutoff,
            db_gain,
        })
    }
    pub fn new_low_shelf_12db(sample_rate: usize, cutoff: f32, db_gain: f32) -> Self {
        Self::inner_new_filter(&MiniFilter2Type::LowShelf {
            sample_rate,
            cutoff,
            db_gain,
        })
    }
    pub fn new_high_shelf_12db(sample_rate: usize, cutoff: f32, db_gain: f32) -> Self {
        Self::inner_new_filter(&MiniFilter2Type::HighShelf {
            sample_rate,
            cutoff,
            db_gain,
        })
    }
}

impl DeviceTrait for Filter {
    fn sources_audio(&self) -> bool {
        true
    }

    fn sinks_audio(&self) -> bool {
        true
    }

    fn add_audio_source(&mut self, source: Rc<RefCell<dyn DeviceTrait>>) {
        self.sources.push(source);
    }

    fn get_audio_sample(&mut self) -> MonoSample {
        self.effect.process(add_sources(&self.sources), -1.0)
    }

    fn handle_automation(&mut self, param_name: &String, param_value: f32) {
        let unscaled_cutoff = MiniFilter2::percent_to_frequency(param_value * 2.0 - 1.0);
        dbg!(param_name, param_value, unscaled_cutoff);
        self.effect.set_cutoff(unscaled_cutoff);
    }
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;

    use crate::{
        common::MonoSample,
        devices::{tests::SingleLevelDevice, traits::DeviceTrait},
    };

    use super::Limiter;
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn test_limiter() {
        const MIN: MonoSample = -0.75;
        const MAX: MonoSample = -MIN;
        {
            let mut limiter = Limiter::new_with_params(MIN, MAX);
            limiter.add_audio_source(Rc::new(RefCell::new(SingleLevelDevice::new(0.5))));
            assert_eq!(limiter.get_audio_sample(), 0.5);
        }
        {
            let mut limiter = Limiter::new_with_params(MIN, MAX);
            limiter.add_audio_source(Rc::new(RefCell::new(SingleLevelDevice::new(-0.8))));
            assert_eq!(limiter.get_audio_sample(), MIN);
        }
        {
            let mut limiter = Limiter::new_with_params(MIN, MAX);
            limiter.add_audio_source(Rc::new(RefCell::new(SingleLevelDevice::new(0.8))));
            assert_eq!(limiter.get_audio_sample(), MAX);
        }

        // multiple sources
        {
            let mut limiter = Limiter::new_with_params(MIN, MAX);
            limiter.add_audio_source(Rc::new(RefCell::new(SingleLevelDevice::new(0.2))));
            limiter.add_audio_source(Rc::new(RefCell::new(SingleLevelDevice::new(0.6))));
            assert_eq!(limiter.get_audio_sample(), MAX);
            limiter.add_audio_source(Rc::new(RefCell::new(SingleLevelDevice::new(-1.0))));
            assert_approx_eq!(limiter.get_audio_sample(), -0.2);
            limiter.add_audio_source(Rc::new(RefCell::new(SingleLevelDevice::new(-1.0))));
            assert_eq!(limiter.get_audio_sample(), MIN);
        }
    }

    // TODO: test multiple sources for all effects. follow lead of limiter
}
