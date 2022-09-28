use crate::common::{MonoSample, MONO_SAMPLE_SILENCE};

pub mod bitcrusher;
pub mod clock;
pub mod envelopes;
pub mod filter;
pub mod gain;
pub mod limiter;
pub mod mixer;
pub mod oscillators;

pub trait SourcesAudio {
    fn source_audio(&mut self, time_seconds: f32) -> MonoSample;
}

pub trait SinksAudio {
    fn sources(&mut self) -> &mut Vec<Box<dyn SourcesAudio>>;

    fn add_audio_source(&mut self, source: Box<dyn SourcesAudio>) {
        self.sources().push(source);
    }

    fn gather_source_audio(&mut self, time_seconds: f32) -> MonoSample {
        if self.sources().is_empty() {
            return MONO_SAMPLE_SILENCE;
        }
        self.sources()
            .iter_mut()
            .map(|source| source.source_audio(time_seconds))
            .sum::<f32>()
            / self.sources().len() as f32
    }
}

pub trait TransformsAudio {
    fn transform_audio(&mut self, input_sample: MonoSample) -> MonoSample;
}

pub trait SourcesControl {
    fn control_sinks(&mut self) -> &mut Vec<Box<dyn SinksControl>>;

    fn add_control_sink(&mut self, sink: Box<dyn SinksControl>) {
        self.control_sinks().push(sink);
    }

    fn control(&mut self, time_seconds: f32);
}

pub enum SinksControlParamType {
    Primary,
    Secondary,
}
pub trait SinksControl {
    fn handle_control(
        &mut self,
        time_seconds: f32,
        param_type: SinksControlParamType,
        new_value: f32,
    );
}

pub trait SourcesFakeMidi {
    fn add_midi_sink(&mut self, sink: Box<dyn SinksFakeMidi>);
}

pub trait SinksFakeMidi {
    fn handle_midi(&mut self, midi: f32, time_seconds: f32);
}

#[cfg(test)]
pub mod tests {

    use convert_case::{Case, Casing};
    use plotters::prelude::*;
    use std::fs;

    use crate::common::{MONO_SAMPLE_MAX, MONO_SAMPLE_SILENCE};
    use crate::{common::MonoSample, primitives::clock::Clock, settings::ClockSettings};

    use super::{SourcesAudio, SourcesControl};

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
        format!("{}/{}-spectrum", OUT_DIR, snake_filename)
    }

    pub(crate) fn write_source_to_file(source: &mut dyn SourcesAudio, basename: &str) {
        let clock_settings = ClockSettings::new_defaults();
        let mut samples = Vec::<MonoSample>::new();
        let mut clock = Clock::new(&clock_settings);
        while clock.seconds < 2.0 {
            samples.push(source.source_audio(clock.seconds));
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
        effect: &mut dyn SourcesAudio,
        opt_controller: &mut dyn SourcesControl,
        basename: &str,
    ) {
        let clock_settings = ClockSettings::new_defaults();
        let mut clock = Clock::new(&clock_settings);
        let mut samples = Vec::<MonoSample>::new();
        while clock.seconds < 2.0 {
            opt_controller.control(clock.seconds);

            let effect_sample = effect.source_audio(clock.seconds);
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

    use std::error::Error;
    fn generate_chart(
        data: &Vec<(f32, f32)>,
        min_domain: f32,
        max_domain: f32,
        min_range: f32,
        max_range: f32,
        filename: &str,
    ) -> Result<(), Box<dyn Error>> {
        let out_filename = format!("{}.png", filename);
        let root = BitMapBackend::new(out_filename.as_str(), (640, 360)).into_drawing_area();
        root.fill(&WHITE)?;

        let mut chart = ChartBuilder::on(&root)
            .margin(0)
            .x_label_area_size(20)
            .y_label_area_size(0)
            .build_cartesian_2d(
                IntoLogRange::log_scale(min_domain..max_domain),
                IntoLogRange::log_scale(min_range..max_range),
            )?;
        chart.configure_mesh().disable_mesh().draw()?;
        chart.draw_series(LineSeries::new(data.iter().map(|t| (t.0, t.1)), &BLUE))?;

        root.present()?;

        Ok(())
    }

    pub(crate) fn generate_fft_for_samples(
        clock_settings: &ClockSettings,
        samples: &Vec<f32>,
        filename: &str,
    ) {
        const HANN_WINDOW_LENGTH: usize = 2048;
        assert!(samples.len() >= HANN_WINDOW_LENGTH);
        let hann_window = hann_window(&samples[0..HANN_WINDOW_LENGTH]);
        let spectrum_hann_window = samples_fft_to_spectrum(
            &hann_window,
            clock_settings.sample_rate() as u32,
            FrequencyLimit::All,
            Some(&divide_by_N),
        )
        .unwrap();

        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        let mut data = Vec::<(f32, f32)>::new();
        for hwd in spectrum_hann_window.data().iter() {
            let mut y = hwd.1.val();
            if y == 0.0 {
                y = f32::EPSILON;
            }
            data.push((hwd.0.val(), y));
            if y < min_y {
                min_y = y;
            }
            if y > max_y {
                max_y = y;
            }
        }

        let _ = generate_chart(
            &data,
            0.0,
            clock_settings.sample_rate() as f32 / 2.0,
            min_y,
            max_y,
            filename,
        );
    }

    pub struct TestAlwaysSameLevelDevice {
        level: MonoSample,
    }
    impl TestAlwaysSameLevelDevice {
        pub fn new(level: MonoSample) -> Self {
            Self { level }
        }
    }
    impl SourcesAudio for TestAlwaysSameLevelDevice {
        fn source_audio(&mut self, _time_seconds: f32) -> MonoSample {
            self.level
        }
    }

    pub struct TestAlwaysTooLoudDevice {}
    impl TestAlwaysTooLoudDevice {
        pub fn new() -> Self {
            Self {}
        }
    }
    impl SourcesAudio for TestAlwaysTooLoudDevice {
        fn source_audio(&mut self, _time_seconds: f32) -> MonoSample {
            MONO_SAMPLE_MAX + 0.1
        }
    }

    pub struct TestAlwaysLoudDevice {}
    impl TestAlwaysLoudDevice {
        pub fn new() -> Self {
            Self {}
        }
    }
    impl SourcesAudio for TestAlwaysLoudDevice {
        fn source_audio(&mut self, _time_seconds: f32) -> MonoSample {
            MONO_SAMPLE_MAX
        }
    }

    pub struct TestAlwaysSilentDevice {}
    impl TestAlwaysSilentDevice {
        pub fn new() -> Self {
            Self {}
        }
    }
    impl SourcesAudio for TestAlwaysSilentDevice {
        fn source_audio(&mut self, _time_seconds: f32) -> MonoSample {
            MONO_SAMPLE_SILENCE
        }
    }
}
