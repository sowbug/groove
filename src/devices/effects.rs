use super::traits::{AutomationMessage, AutomationSink};
use crate::{
    common::MonoSample,
    primitives::{
        self,
        clock::Clock,
        filter::{MiniFilter2, MiniFilter2Type},
        gain::MiniGain,
        limiter::MiniLimiter,
        SinksAudio, SourcesAudio, TransformsAudio, WatchesClock,
    },
};
use std::{cell::RefCell, rc::Rc};

pub struct Limiter {
    sources: Vec<Rc<RefCell<dyn SourcesAudio>>>,
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
impl Default for Limiter {
    fn default() -> Self {
        Self::new()
    }
}
impl SinksAudio for Limiter {
    fn sources(&mut self) -> &mut Vec<Rc<RefCell<dyn SourcesAudio>>> {
        &mut self.sources
    }
}
impl SourcesAudio for Limiter {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        let input = self.gather_source_audio(clock);
        self.effect.transform_audio(input)
    }
}
impl AutomationSink for Limiter {
    fn handle_automation_message(&mut self, _message: &AutomationMessage) {
        todo!()
    }
}
impl WatchesClock for Limiter {
    fn tick(&mut self, _clock: &Clock) -> bool {
        true
    }
}

#[derive(Default)]
pub struct Gain {
    sources: Vec<Rc<RefCell<dyn SourcesAudio>>>,
    effect: MiniGain,
}

impl Gain {
    pub fn new_with(amount: f32) -> Self {
        Self {
            sources: Vec::new(),
            effect: MiniGain::new_with(amount),
        }
    }

    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::new_with(1.0)
    }
}

impl SinksAudio for Gain {
    fn sources(&mut self) -> &mut Vec<Rc<RefCell<dyn SourcesAudio>>> {
        &mut self.sources
    }
}

impl SourcesAudio for Gain {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        let input = self.gather_source_audio(clock);
        self.effect.transform_audio(input)
    }
}

impl AutomationSink for Gain {
    fn handle_automation_message(&mut self, _message: &AutomationMessage) {
        todo!()
    }
}
impl WatchesClock for Gain {
    fn tick(&mut self, _clock: &Clock) -> bool {
        true
    }
}

pub struct Bitcrusher {
    sources: Vec<Rc<RefCell<dyn SourcesAudio>>>,
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

    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::new_with_params(8)
    }
}

impl Default for Bitcrusher {
    fn default() -> Self {
        Self::new()
    }
}

impl SinksAudio for Bitcrusher {
    fn sources(&mut self) -> &mut Vec<Rc<RefCell<dyn SourcesAudio>>> {
        &mut self.sources
    }
}

impl WatchesClock for Bitcrusher {
    fn tick(&mut self, clock: &Clock) -> bool {
        self.time_seconds = clock.seconds;
        true
    }
}

impl SourcesAudio for Bitcrusher {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        let input = self.gather_source_audio(clock);
        self.effect.transform_audio(input)
    }
}

impl AutomationSink for Bitcrusher {
    fn handle_automation_message(&mut self, _message: &AutomationMessage) {
        todo!()
    }
}

#[allow(dead_code)]
pub struct Filter {
    sources: Vec<Rc<RefCell<dyn SourcesAudio>>>,
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

impl SinksAudio for Filter {
    fn sources(&mut self) -> &mut Vec<Rc<RefCell<dyn SourcesAudio>>> {
        &mut self.sources
    }
}

impl SourcesAudio for Filter {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        let input = self.gather_source_audio(clock);
        self.effect.transform_audio(input)
    }
}

impl AutomationSink for Filter {
    fn handle_automation_message(&mut self, message: &AutomationMessage) {
        match message {
            AutomationMessage::UpdatePrimaryValue { value } => {
                let unscaled_cutoff = MiniFilter2::percent_to_frequency(value * 2.0 - 1.0);
                self.effect.set_cutoff(unscaled_cutoff);
            }
            _ => todo!(),
        }
    }
}

impl WatchesClock for Filter {
    fn tick(&mut self, _clock: &Clock) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;

    use crate::{
        common::MonoSample,
        devices::tests::SingleLevelDevice,
        primitives::{clock::Clock, SourcesAudio, SinksAudio},
    };

    use super::Limiter;
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn test_limiter() {
        const MIN: MonoSample = -0.75;
        const MAX: MonoSample = -MIN;
        let clock = Clock::new_test();
        {
            let mut limiter = Limiter::new_with_params(MIN, MAX);
            limiter.add_audio_source(Rc::new(RefCell::new(SingleLevelDevice::new(0.5))));
            assert_eq!(limiter.source_audio(&clock), 0.5);
        }
        {
            let mut limiter = Limiter::new_with_params(MIN, MAX);
            limiter.add_audio_source(Rc::new(RefCell::new(SingleLevelDevice::new(-0.8))));
            assert_eq!(limiter.source_audio(&clock), MIN);
        }
        {
            let mut limiter = Limiter::new_with_params(MIN, MAX);
            limiter.add_audio_source(Rc::new(RefCell::new(SingleLevelDevice::new(0.8))));
            assert_eq!(limiter.source_audio(&clock), MAX);
        }

        // multiple sources
        {
            let mut limiter = Limiter::new_with_params(MIN, MAX);
            limiter.add_audio_source(Rc::new(RefCell::new(SingleLevelDevice::new(0.2))));
            limiter.add_audio_source(Rc::new(RefCell::new(SingleLevelDevice::new(0.6))));
            assert_eq!(limiter.source_audio(&clock), MAX);
            limiter.add_audio_source(Rc::new(RefCell::new(SingleLevelDevice::new(-1.0))));
            assert_approx_eq!(limiter.source_audio(&clock), -0.2);
            limiter.add_audio_source(Rc::new(RefCell::new(SingleLevelDevice::new(-1.0))));
            assert_eq!(limiter.source_audio(&clock), MIN);
        }
    }

    // TODO: test multiple sources for all effects. follow lead of limiter
}
