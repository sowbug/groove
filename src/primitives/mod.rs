use crate::common::MonoSample;

pub mod bitcrusher;
pub mod clock;
pub mod envelopes;
pub mod filter;
pub mod gain;
pub mod limiter;
pub mod mixer;
pub mod oscillators;

#[allow(unused_variables)]
pub trait AudioSourceTrait__ {
    fn process(&mut self, time_seconds: f32) -> MonoSample {
        0.0
    }
}

#[allow(unused_variables)]
pub trait EffectTrait__ {
    fn process(&mut self, input: MonoSample, time_seconds: f32) -> MonoSample {
        input
    }
}

#[allow(unused_variables)]
pub trait ControllerTrait__ {
    fn process(&mut self, time_seconds: f32) {}
}

#[cfg(test)]
pub mod tests {
    use std::{cell::RefCell, fs, rc::Rc};

    use convert_case::{Case, Casing};

    use crate::{common::MonoSample, primitives::clock::Clock, settings::ClockSettings};

    use super::{AudioSourceTrait__, ControllerTrait__, EffectTrait__};

    pub fn canonicalize_filename(filename: &str) -> String {
        const OUT_DIR: &str = "out";
        let result = fs::create_dir_all(OUT_DIR);
        if result.is_err() {
            panic!();
        }
        let snake_filename = filename.to_case(Case::Snake);
        format!("{}/{}.wav", OUT_DIR, snake_filename)
    }

    pub fn canonicalize_fft_filename(filename: &str) -> String {
        const OUT_DIR: &str = "out";
        let snake_filename = filename.to_case(Case::Snake);
        format!("{}/{}-fft.csv", OUT_DIR, snake_filename)
    }

    pub(crate) fn write_source_to_file(source: &mut dyn AudioSourceTrait__, basename: &str) {
        let clock_settings = ClockSettings::new_defaults();
        let mut samples = Vec::<MonoSample>::new();
        let mut clock = Clock::new(&clock_settings);
        while clock.seconds < 2.0 {
            samples.push(source.process(clock.seconds));
            clock.tick();
        }
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: clock_settings.sample_rate() as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        const AMPLITUDE: MonoSample = i16::MAX as MonoSample;
        let mut writer = hound::WavWriter::create(canonicalize_filename(basename), spec).unwrap();
        for sample in samples.clone() {
            let _ = writer.write_sample((sample * AMPLITUDE) as i16);
        }
        generate_fft_for_samples(
            &clock_settings,
            &samples,
            &canonicalize_fft_filename(basename),
        );
    }

    pub(crate) fn write_effect_to_file(
        source: &mut dyn AudioSourceTrait__,
        effect: Rc<RefCell<dyn EffectTrait__>>,
        opt_controller: &mut Option<&mut dyn ControllerTrait__>,
        basename: &str,
    ) {
        let clock_settings = ClockSettings::new_defaults();
        let mut clock = Clock::new(&clock_settings);
        let mut samples = Vec::<MonoSample>::new();
        while clock.seconds < 2.0 {
            if opt_controller.is_some() {
                opt_controller.as_mut().unwrap().process(clock.seconds);
            }
            let source_sample = source.process(clock.seconds);
            let effect_sample = effect.borrow_mut().process(source_sample, clock.seconds);
            samples.push(effect_sample);
            clock.tick();
        }

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: clock.settings().sample_rate() as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        const AMPLITUDE: MonoSample = i16::MAX as MonoSample;
        let mut writer = hound::WavWriter::create(canonicalize_filename(basename), spec).unwrap();
        for sample in samples.clone() {
            let _ = writer.write_sample((sample * AMPLITUDE) as i16);
        }
        generate_fft_for_samples(
            &clock_settings,
            &samples,
            &canonicalize_fft_filename(basename),
        );
    }

    use spectrum_analyzer::scaling::divide_by_N;
    use spectrum_analyzer::windows::hann_window;
    use spectrum_analyzer::{samples_fft_to_spectrum, FrequencyLimit};

    pub(crate) fn generate_fft_for_samples(
        clock_settings: &ClockSettings,
        samples: &Vec<f32>,
        filename: &str,
    ) {
        const HANN_WINDOW_LENGTH: usize = 1024;
        assert!(samples.len() >= HANN_WINDOW_LENGTH);
        let hann_window = hann_window(&samples[0..HANN_WINDOW_LENGTH]);
        let spectrum_hann_window = samples_fft_to_spectrum(
            &hann_window,
            clock_settings.sample_rate() as u32,
            FrequencyLimit::All,
            Some(&divide_by_N),
        )
        .unwrap();

        let mut output_text = String::new();
        for i in 0..spectrum_hann_window.data().len() {
            let d = spectrum_hann_window.data()[i];
            let s = format!("{}, {}\n", d.0, d.1);
            output_text.push_str(s.as_str());
        }
        match fs::write(filename, output_text) {
            Ok(_) => (),
            Err(e) => panic!("{:?}", e),
        }
    }

    pub struct TestAlwaysTooLoudDevice {}
    impl TestAlwaysTooLoudDevice {
        pub fn new() -> Self {
            Self {}
        }
        pub fn get_audio_sample(&self) -> MonoSample {
            1.1
        }
    }

    pub struct TestAlwaysLoudDevice {}
    impl TestAlwaysLoudDevice {
        pub fn new() -> Self {
            Self {}
        }
        pub fn get_audio_sample(&self) -> MonoSample {
            1.
        }
    }

    pub struct TestAlwaysSilentDevice {}
    impl TestAlwaysSilentDevice {
        pub fn new() -> Self {
            Self {}
        }
        pub fn get_audio_sample(&self) -> MonoSample {
            0.
        }
    }
}
