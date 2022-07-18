pub mod clock;
pub mod envelopes;
pub mod filter;
pub mod gain;
pub mod limiter;
pub mod mixer;
pub mod oscillators;
pub mod bitcrusher;

#[allow(unused_variables)]
pub trait AudioSourceTrait {
    fn process(&mut self, time_seconds: f32) -> f32 {
        0.0
    }
}

#[allow(unused_variables)]
pub trait EffectTrait {
    fn process(&mut self, input: f32, time_seconds: f32) -> f32 {
        input
    }
}

#[allow(unused_variables)]
pub trait ControllerTrait {
    fn process(&mut self, time_seconds: f32) {}
}

#[cfg(test)]
pub mod tests {
    use std::{cell::RefCell, rc::Rc};

    use crate::primitives::clock::Clock;

    use super::{AudioSourceTrait, EffectTrait, ControllerTrait};

    pub(crate) fn write_source_to_file(source: &mut dyn AudioSourceTrait, filename: &str) {
        let mut clock = Clock::new(44100, 4, 4, 128.);

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: clock.sample_rate(),
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        const AMPLITUDE: f32 = i16::MAX as f32;
        let mut writer = hound::WavWriter::create(filename, spec).unwrap();

        while clock.seconds < 2.0 {
            let source_sample = source.process(clock.seconds);
            let _ = writer.write_sample((source_sample * AMPLITUDE) as i16);
            clock.tick();
        }
    }

    pub(crate) fn write_effect_to_file(
        source: &mut dyn AudioSourceTrait,
        effect: Rc<RefCell<dyn EffectTrait>>,
        opt_controller: &mut Option<&mut dyn ControllerTrait>,
        filename: &str,
    ) {
        let mut clock = Clock::new(44100, 4, 4, 128.);

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: clock.sample_rate(),
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        const AMPLITUDE: f32 = i16::MAX as f32;
        let mut writer = hound::WavWriter::create(filename, spec).unwrap();


        while clock.seconds < 2.0 {
            if opt_controller.is_some() {
                opt_controller.as_mut().unwrap().process(clock.seconds);
            }
            let source_sample = source.process(clock.seconds);
            let effect_sample = effect.borrow_mut().process(source_sample, clock.seconds);
            let _ = writer.write_sample((effect_sample * AMPLITUDE) as i16);
            clock.tick();
        }
    }

    pub struct TestAlwaysTooLoudDevice {}
    impl TestAlwaysTooLoudDevice {
        pub fn new() -> Self {
            Self {}
        }
        pub fn get_audio_sample(&self) -> f32 {
            1.1
        }
    }

    pub struct TestAlwaysLoudDevice {}
    impl TestAlwaysLoudDevice {
        pub fn new() -> Self {
            Self {}
        }
        pub fn get_audio_sample(&self) -> f32 {
            1.
        }
    }

    pub struct TestAlwaysSilentDevice {}
    impl TestAlwaysSilentDevice {
        pub fn new() -> Self {
            Self {}
        }
        pub fn get_audio_sample(&self) -> f32 {
            0.
        }
    }
}
