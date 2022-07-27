use super::traits::DeviceTrait;
use crate::primitives::{
    self,
    filter::{MiniFilter2, MiniFilter2Type},
    gain::MiniGain,
    limiter::MiniLimiter,
    EffectTrait,
};
use std::{cell::RefCell, rc::Rc};

pub struct Limiter {
    source: Option<Rc<RefCell<dyn DeviceTrait>>>,
    effect: MiniLimiter,
}

impl Limiter {
    pub fn new_with_params(min: f32, max: f32) -> Self {
        Self {
            source: None,
            effect: MiniLimiter::new(min, max),
        }
    }

    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            source: None,
            effect: MiniLimiter::new(0.0, 1.0),
        }
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
        self.source = Some(source);
    }
    fn get_audio_sample(&mut self) -> f32 {
        if self.source.is_some() {
            let source_ref = self.source.as_ref().unwrap();
            self.effect
                .process(source_ref.borrow_mut().get_audio_sample())
        } else {
            0.0
        }
    }
}

#[derive(Default)]
pub struct Gain {
    source: Option<Rc<RefCell<dyn DeviceTrait>>>,
    effect: MiniGain,
}

impl Gain {
    pub fn new_with_params(amount: f32) -> Self {
        Self {
            source: None,
            effect: MiniGain::new(amount), // TODO: consider new_with_params() convention
        }
    }

    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            source: None,
            effect: MiniGain::new(1.0), // TODO: what's a neutral gain?
        }
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
        self.source = Some(source);
    }
    fn get_audio_sample(&mut self) -> f32 {
        if self.source.is_some() {
            let source_ref = self.source.as_ref().unwrap();
            self.effect
                .process(source_ref.borrow_mut().get_audio_sample())
        } else {
            0.0
        }
    }
}

pub struct Bitcrusher {
    source: Option<Rc<RefCell<dyn DeviceTrait>>>,
    effect: primitives::bitcrusher::Bitcrusher,
    time_seconds: f32,
}

impl Bitcrusher {
    pub fn new_with_params(bits_to_crush: u8) -> Self {
        Self {
            source: None,
            effect: primitives::bitcrusher::Bitcrusher::new(bits_to_crush),
            time_seconds: 0.0,
        }
    }
    pub fn new() -> Self {
        Self {
            source: None,
            effect: primitives::bitcrusher::Bitcrusher::new(8),
            time_seconds: 0.0,
        }
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
        self.source = Some(source);
    }

    fn tick(&mut self, clock: &primitives::clock::Clock) -> bool {
        self.time_seconds = clock.seconds;
        true
    }

    fn get_audio_sample(&mut self) -> f32 {
        if self.source.is_some() {
            let source_ref = self.source.as_ref().unwrap();
            self.effect.process(
                source_ref.borrow_mut().get_audio_sample(),
                self.time_seconds,
            )
        } else {
            0.0
        }
    }
}

#[allow(dead_code)]
pub struct Filter {
    source: Option<Rc<RefCell<dyn DeviceTrait>>>,
    effect: MiniFilter2,

    filter_type: MiniFilter2Type,
}

impl Filter {
    fn inner_new_filter(ft: &MiniFilter2Type) -> Self {
        Self {
            source: None,
            effect: MiniFilter2::new(ft),
            filter_type: *ft,
        }
    }
    pub fn new_low_pass_12db(sample_rate: u32, cutoff: f32, q: f32) -> Self {
        Self::inner_new_filter(&MiniFilter2Type::LowPass {
            sample_rate,
            cutoff,
            q,
        })
    }
    pub fn new_high_pass_12db(sample_rate: u32, cutoff: f32, q: f32) -> Self {
        Self::inner_new_filter(&MiniFilter2Type::HighPass {
            sample_rate,
            cutoff,
            q,
        })
    }
    pub fn new_band_pass_12db(sample_rate: u32, cutoff: f32, bandwidth: f32) -> Self {
        Self::inner_new_filter(&MiniFilter2Type::BandPass {
            sample_rate,
            cutoff,
            bandwidth,
        })
    }
    pub fn new_band_stop_12db(sample_rate: u32, cutoff: f32, bandwidth: f32) -> Self {
        Self::inner_new_filter(&MiniFilter2Type::BandStop {
            sample_rate,
            cutoff,
            bandwidth,
        })
    }
    pub fn new_all_pass_12db(sample_rate: u32, cutoff: f32, q: f32) -> Self {
        Self::inner_new_filter(&MiniFilter2Type::AllPass {
            sample_rate,
            cutoff,
            q,
        })
    }
    pub fn new_peaking_eq_12db(sample_rate: u32, cutoff: f32, db_gain: f32) -> Self {
        Self::inner_new_filter(&MiniFilter2Type::PeakingEq {
            sample_rate,
            cutoff,
            db_gain,
        })
    }
    pub fn new_low_shelf_12db(sample_rate: u32, cutoff: f32, db_gain: f32) -> Self {
        Self::inner_new_filter(&MiniFilter2Type::LowShelf {
            sample_rate,
            cutoff,
            db_gain,
        })
    }
    pub fn new_high_shelf_12db(sample_rate: u32, cutoff: f32, db_gain: f32) -> Self {
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
        self.source = Some(source);
    }

    fn tick(&mut self, _clock: &primitives::clock::Clock) -> bool {
        true
    }

    fn get_audio_sample(&mut self) -> f32 {
        if self.source.is_some() {
            let source_ref = self.source.as_ref().unwrap();
            self.effect
                .process(source_ref.borrow_mut().get_audio_sample(), 0.0)
        } else {
            0.0
        }
    }
}
