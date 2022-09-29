use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use crate::common::{MonoSample, MONO_SAMPLE_SILENCE};

pub mod bitcrusher;
pub mod clock;
pub mod envelopes;
pub mod filter;
pub mod gain;
pub mod limiter;
pub mod mixer;
pub mod oscillators;

pub trait Wrappable: Default {
    fn new() -> Self;
}

pub fn wrapped_new<T: Wrappable>() -> Rc<RefCell<T>> {
    Rc::new(RefCell::new(T::new()))
}

pub trait SourcesAudio {
    // Lots of implementers don't care about time_seconds here,
    // but some do (oscillators, LFOs), and it's a lot cleaner
    // to pass a bit of extra information here than to either
    // create a separate optional method supplying it (which
    // everyone would have to call anyway), or define a whole
    // new trait that breaks a bunch of simple paths elsewhere.
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
    }
}

pub trait SourcesControl {
    fn control_sinks(&self) -> &[Weak<RefCell<dyn SinksControl>>];
    fn control_sinks_mut(&mut self) -> &mut Vec<Weak<RefCell<dyn SinksControl>>>;

    fn add_control_sink(&mut self, sink: Weak<RefCell<dyn SinksControl>>) {
        self.control_sinks_mut().push(sink);
    }
    fn issue_control(&mut self, time_seconds: f32, param: &SinksControlParamType) {
        for sink in self.control_sinks_mut() {
            if let Some(sink_up) = sink.upgrade() {
                sink_up.borrow_mut().handle_control(time_seconds, param);
            }
        }
    }
}

pub trait SinksControl {
    fn handle_control(&mut self, time_seconds: f32, param: &SinksControlParamType);
}

pub enum SinksControlParamType {
    Primary { value: f32 },
    Secondary { value: f32 },
}

pub trait SourcesFakeMidi {
    fn add_midi_sink(&mut self, sink: Box<dyn SinksFakeMidi>);
}

pub trait SinksFakeMidi {
    fn handle_midi(&mut self, midi: f32, time_seconds: f32);
}

pub trait WatchesClock {
    /// returns true if we had a finite amount of known work that has finished.
    /// TODO: if we return true, then do we still expect to be called for the
    /// result of our work during this cycle? E.g., source_audio()
    ///
    /// If you're not sure what you should return, you should return true.
    /// This is because false prevents outer loops from ending.
    fn tick(&mut self, time_seconds: f32) -> bool;
}

pub trait TransformsAudio {
    fn transform_audio(&mut self, input_sample: MonoSample) -> MonoSample;
}

pub trait IsEffect: SourcesAudio + SinksAudio + TransformsAudio {}
pub trait IsController: SourcesControl + WatchesClock {}

pub trait TransformsControlToAudio /*  SinksControl + SourcesAudio */ {}
impl<T: SinksAudio + TransformsAudio> SourcesAudio for T {
    fn source_audio(&mut self, time_seconds: f32) -> MonoSample {
        let input = self.gather_source_audio(time_seconds);
        self.transform_audio(input)
    }
}

#[cfg(test)]
pub mod tests {

    use convert_case::{Case, Casing};
    use plotters::prelude::*;
    use std::cell::RefCell;
    use std::fs;
    use std::rc::{Rc, Weak};

    use crate::common::{MidiMessage, MONO_SAMPLE_MAX, MONO_SAMPLE_SILENCE};
    use crate::primitives::wrapped_new;
    use crate::{common::MonoSample, primitives::clock::Clock, settings::ClockSettings};

    use super::clock::WatchedClock;
    use super::envelopes::MiniEnvelope;
    use super::mixer::Mixer;
    use super::oscillators::MiniOscillator;
    use super::{
        IsController, IsEffect, SinksControl, SinksControlParamType, SourcesAudio, SourcesControl,
        WatchesClock,
    };

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
        let mut clock = Clock::new_with(&clock_settings);
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
        opt_controller: &mut dyn IsController,
        basename: &str,
    ) {
        let clock_settings = ClockSettings::new_defaults();
        let mut clock = Clock::new_with(&clock_settings);
        let mut samples = Vec::<MonoSample>::new();
        while clock.seconds < 2.0 {
            opt_controller.tick(clock.seconds);

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

    #[derive(Default)]
    pub struct TestAlwaysSameLevelDevice {
        level: MonoSample,
    }
    impl TestAlwaysSameLevelDevice {
        pub fn new(level: MonoSample) -> Self {
            Self {
                level,
                ..Default::default()
            }
        }
    }
    impl SourcesAudio for TestAlwaysSameLevelDevice {
        fn source_audio(&mut self, _time_seconds: f32) -> MonoSample {
            self.level
        }
    }

    #[derive(Default)]
    pub struct TestAlwaysTooLoudDevice {}
    impl TestAlwaysTooLoudDevice {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }
    impl SourcesAudio for TestAlwaysTooLoudDevice {
        fn source_audio(&mut self, _time_seconds: f32) -> MonoSample {
            MONO_SAMPLE_MAX + 0.1
        }
    }

    #[derive(Default)]
    pub struct TestAlwaysLoudDevice {}
    impl TestAlwaysLoudDevice {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }
    impl SourcesAudio for TestAlwaysLoudDevice {
        fn source_audio(&mut self, _time_seconds: f32) -> MonoSample {
            MONO_SAMPLE_MAX
        }
    }

    #[derive(Default)]
    pub struct TestAlwaysSilentDevice {}
    impl TestAlwaysSilentDevice {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }
    impl SourcesAudio for TestAlwaysSilentDevice {
        fn source_audio(&mut self, _time_seconds: f32) -> MonoSample {
            MONO_SAMPLE_SILENCE
        }
    }

    pub struct SimpleOrchestrator {
        main_mixer: Box<dyn IsEffect>,
    }

    impl Default for SimpleOrchestrator {
        fn default() -> Self {
            Self::new()
        }
    }

    impl SimpleOrchestrator {
        fn new() -> Self {
            Self {
                main_mixer: Box::new(Mixer::new()),
            }
        }

        // TODO: I like "new_with"
        #[allow(dead_code)]
        fn new_with(main_mixer: Box<dyn IsEffect>) -> Self {
            Self { main_mixer }
        }

        fn add_audio_source(&mut self, source: Box<dyn SourcesAudio>) {
            self.main_mixer.add_audio_source(source);
        }

        fn start(&mut self, clock: &mut WatchedClock, samples_out: &mut Vec<f32>) {
            loop {
                if clock.visit_watchers() {
                    break;
                }
                samples_out.push(self.main_mixer.source_audio(clock.seconds()));
                clock.tick();
            }
        }
    }

    pub struct SimpleSynth {
        oscillator: Rc<RefCell<dyn SourcesAudio>>,
        envelope: Rc<RefCell<dyn SourcesAudio>>,
    }

    impl SimpleSynth {
        fn new() -> Self {
            Self {
                oscillator: wrapped_new::<MiniOscillator>(),
                envelope: wrapped_new::<MiniEnvelope>(),
            }
        }
        fn new_with(
            oscillator: Rc<RefCell<dyn SourcesAudio>>,
            envelope: Rc<RefCell<dyn SourcesAudio>>,
        ) -> Self {
            Self {
                oscillator,
                envelope,
            }
        }
    }

    impl Default for SimpleSynth {
        fn default() -> Self {
            Self::new()
        }
    }

    impl SourcesAudio for SimpleSynth {
        fn source_audio(&mut self, time_seconds: f32) -> MonoSample {
            self.oscillator.borrow_mut().source_audio(time_seconds)
                * self.envelope.borrow_mut().source_audio(time_seconds)
        }
    }

    #[derive(Default)]
    pub struct SimpleTimer {
        time_to_run_seconds: f32,
    }
    impl SimpleTimer {
        fn new(time_to_run_seconds: f32) -> Self {
            Self {
                time_to_run_seconds,
            }
        }
    }
    impl WatchesClock for SimpleTimer {
        fn tick(&mut self, time_seconds: f32) -> bool {
            time_seconds >= self.time_to_run_seconds
        }
    }

    #[derive(Default)]
    pub struct SimpleTrigger {
        control_sinks: Vec<Weak<RefCell<dyn SinksControl>>>,
        time_to_trigger_seconds: f32,
        value: f32,
        has_triggered: bool,
    }
    impl SimpleTrigger {
        fn new(time_to_trigger_seconds: f32, value: f32) -> Self {
            Self {
                time_to_trigger_seconds,
                value,
                ..Default::default()
            }
        }
    }
    impl WatchesClock for SimpleTrigger {
        fn tick(&mut self, time_seconds: f32) -> bool {
            if !self.has_triggered && time_seconds >= self.time_to_trigger_seconds {
                self.has_triggered = true;
                let value = self.value;
                self.issue_control(time_seconds, &SinksControlParamType::Primary { value });
            }
            time_seconds >= self.time_to_trigger_seconds
        }
    }
    impl SourcesControl for SimpleTrigger {
        fn control_sinks(&self) -> &[Weak<RefCell<dyn SinksControl>>] {
            &self.control_sinks
        }

        fn control_sinks_mut(&mut self) -> &mut Vec<Weak<RefCell<dyn SinksControl>>> {
            &mut self.control_sinks
        }
    }

    #[test]
    fn test_simple_orchestrator() {
        let mut orchestrator = SimpleOrchestrator::new();
        let envelope = wrapped_new::<MiniEnvelope>();
        let oscillator = Rc::new(RefCell::new(MiniOscillator::new_with(
            crate::common::WaveformType::Sine,
        )));
        oscillator
            .borrow_mut()
            .set_frequency(MidiMessage::note_to_frequency(60));
        let synth = SimpleSynth::new_with(oscillator, envelope.clone());

        let mut clock = WatchedClock::new();

        orchestrator.add_audio_source(Box::new(synth));
        let timer = SimpleTimer::new(2.0);
        clock.add_watcher(Box::new(timer));

        let mut trigger_on = SimpleTrigger::new(1.0, 1.0);
        let weak_envelope_on = Rc::downgrade(&envelope);
        trigger_on.add_control_sink(weak_envelope_on);
        clock.add_watcher(Box::new(trigger_on));

        let mut trigger_off = SimpleTrigger::new(1.5, 0.0);
        let weak_envelope_off = Rc::downgrade(&envelope);
        trigger_off.add_control_sink(weak_envelope_off);
        clock.add_watcher(Box::new(trigger_off));

        let mut samples = Vec::<MonoSample>::new();
        orchestrator.start(&mut clock, &mut samples);
        assert_eq!(samples.len(), 2 * 44100);
        assert_eq!(samples[0], 0.0); // because the envelope hasn't been triggered yet
        assert!(samples[44100] != 0.0); // envelope should be triggered at 1-second mark

        // envelope should be triggered at 1-second mark. We check two consecutive samples just in
        // case the oscillator happens to cross over between negative and positive right at that moment.
        assert!(samples[44100] != 0.0 || samples[44100 + 1] != 0.0);
    }
}
