use crate::{
    clock::Clock,
    controllers::{BigMessage, SmallMessageGenerator},
    messages::GrooveMessage,
    traits::{
        EvenNewerCommand, HasUid, MessageBounds, NewIsController, NewUpdateable, SourcesUpdates,
        Terminates, WatchesClock,
    },
};

use core::fmt::Debug;

use std::marker::PhantomData;

#[derive(Debug, Default)]
pub(crate) struct Timer<M: MessageBounds> {
    uid: usize,
    has_more_work: bool,
    time_to_run_seconds: f32,

    _phantom: PhantomData<M>,
}
impl<M: MessageBounds> Timer<M> {
    #[allow(dead_code)]
    pub fn new_with(time_to_run_seconds: f32) -> Self {
        Self {
            time_to_run_seconds,
            ..Default::default()
        }
    }
}
impl<M: MessageBounds> Terminates for Timer<M> {
    fn is_finished(&self) -> bool {
        !self.has_more_work
    }
}
impl<M: MessageBounds> NewIsController for Timer<M> {}
impl<M: MessageBounds> NewUpdateable for Timer<M> {
    default type Message = M;

    default fn update(
        &mut self,
        _clock: &Clock,
        _message: Self::Message,
    ) -> EvenNewerCommand<Self::Message> {
        EvenNewerCommand::none()
    }
}
impl NewUpdateable for Timer<GrooveMessage> {
    type Message = GrooveMessage;

    fn update(&mut self, clock: &Clock, message: Self::Message) -> EvenNewerCommand<Self::Message> {
        match message {
            GrooveMessage::Tick => {
                self.has_more_work = clock.seconds() < self.time_to_run_seconds;
            }
            _ => {}
        }
        EvenNewerCommand::none()
    }
}
impl<M: MessageBounds> HasUid for Timer<M> {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}
impl<M: MessageBounds> WatchesClock for Timer<M> {
    fn tick(&mut self, clock: &Clock) -> Vec<BigMessage> {
        self.has_more_work = clock.seconds() < self.time_to_run_seconds;
        Vec::new()
    }
}

#[derive(Debug, Default)]
pub(crate) struct Trigger<M: MessageBounds> {
    uid: usize,
    time_to_trigger_seconds: f32,
    value: f32,
    has_triggered: bool,

    target_uids: Vec<usize>,
    target_messages: Vec<SmallMessageGenerator>,

    _phantom: PhantomData<M>,
}
impl<M: MessageBounds> NewIsController for Trigger<M> {}
impl<M: MessageBounds> NewUpdateable for Trigger<M> {
    default type Message = M;

    default fn update(
        &mut self,
        _clock: &Clock,
        _message: Self::Message,
    ) -> EvenNewerCommand<Self::Message> {
        EvenNewerCommand::none()
    }
}
impl<M: MessageBounds> Terminates for Trigger<M> {
    fn is_finished(&self) -> bool {
        self.has_triggered
    }
}
impl<M: MessageBounds> HasUid for Trigger<M> {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}
impl<M: MessageBounds> Trigger<M> {
    pub fn new(time_to_trigger_seconds: f32, value: f32) -> Self {
        Self {
            time_to_trigger_seconds,
            value,
            ..Default::default()
        }
    }
}
impl<M: MessageBounds> SourcesUpdates for Trigger<M> {
    fn target_uids(&self) -> &[usize] {
        &self.target_uids
    }

    fn target_uids_mut(&mut self) -> &mut Vec<usize> {
        &mut self.target_uids
    }

    fn target_messages(&self) -> &[SmallMessageGenerator] {
        &self.target_messages
    }

    fn target_messages_mut(&mut self) -> &mut Vec<SmallMessageGenerator> {
        &mut self.target_messages
    }
}
impl<M: MessageBounds> WatchesClock for Trigger<M> {
    fn tick(&mut self, clock: &Clock) -> Vec<BigMessage> {
        if !self.has_triggered && clock.seconds() >= self.time_to_trigger_seconds {
            self.has_triggered = true;
            let value = self.value;
            self.post_message(value)
        } else {
            Vec::default()
        }
    }
}
impl NewUpdateable for Trigger<GrooveMessage> {
    type Message = GrooveMessage;

    fn update(&mut self, clock: &Clock, message: Self::Message) -> EvenNewerCommand<Self::Message> {
        match message {
            GrooveMessage::Tick => {
                return if !self.has_triggered && clock.seconds() >= self.time_to_trigger_seconds {
                    self.has_triggered = true;
                    EvenNewerCommand::single(GrooveMessage::ControlF32(self.uid, self.value))
                } else {
                    EvenNewerCommand::none()
                };
            }
            _ => {}
        }
        EvenNewerCommand::none()
    }
}

#[cfg(test)]
pub mod tests {
    use crate::{
        clock::{Clock, ClockTimeUnit, WatchedClock},
        common::{
            rrc, rrc_clone, rrc_downgrade, MonoSample, Rrc, Ww, MONO_SAMPLE_MAX, MONO_SAMPLE_MIN,
            MONO_SAMPLE_SILENCE,
        },
        controllers::{BigMessage, SmallMessage, SmallMessageGenerator},
        effects::mixer::Mixer,
        envelopes::AdsrEnvelope,
        messages::{tests::TestMessage, GrooveMessage},
        midi::{MidiChannel, MidiMessage, MidiUtils},
        orchestrator::{tests::Runner, GrooveRunner, Orchestrator},
        oscillators::Oscillator,
        settings::patches::EnvelopeSettings,
        settings::patches::WaveformType,
        settings::ClockSettings,
        traits::{
            BoxedEntity, EvenNewerCommand, HasOverhead, HasUid, IsEffect, MessageBounds,
            NewIsController, NewIsEffect, NewIsInstrument, NewUpdateable, Overhead, SinksAudio,
            SinksMidi, SinksUpdates, SourcesAudio, SourcesMidi, Terminates, TransformsAudio,
            WatchesClock,
        },
    };
    use assert_approx_eq::assert_approx_eq;
    use convert_case::{Case, Casing};
    use strum_macros::FromRepr;
    // use plotters::prelude::*;
    use super::{Timer, Trigger};
    use spectrum_analyzer::{
        samples_fft_to_spectrum, scaling::divide_by_N, windows::hann_window, FrequencyLimit,
    };
    use std::{
        collections::{HashMap, VecDeque},
        str::FromStr,
    };
    use std::{fs, marker::PhantomData};
    use strum_macros::{Display, EnumString};

    pub fn canonicalize_filename(filename: &str) -> String {
        const OUT_DIR: &str = "out";
        let result = fs::create_dir_all(OUT_DIR);
        if result.is_err() {
            panic!();
        }
        let snake_filename = filename.to_case(Case::Snake);
        format!("{OUT_DIR}/{snake_filename}.wav")
    }

    pub fn canonicalize_fft_filename(filename: &str) -> String {
        const OUT_DIR: &str = "out";
        let snake_filename = filename.to_case(Case::Snake);
        format!("{OUT_DIR}/{snake_filename}-spectrum")
    }

    fn write_samples_to_wav_file(basename: &str, sample_rate: usize, samples: &[MonoSample]) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: sample_rate as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        const AMPLITUDE: MonoSample = i16::MAX as MonoSample;
        let mut writer = hound::WavWriter::create(canonicalize_filename(basename), spec).unwrap();
        for sample in samples.iter() {
            let _ = writer.write_sample((sample * AMPLITUDE) as i16);
        }
    }

    pub(crate) fn write_source_and_controlled_effect(
        basename: &str,
        waveform_type: WaveformType,
        effect_opt: Option<Rrc<dyn IsEffect>>,
        control_opt: Option<Rrc<dyn WatchesClock>>,
    ) {
        let mut c = WatchedClock::new();
        let sample_rate = c.inner_clock().sample_rate();
        let mut o = OldTestOrchestrator::new();
        let osc = Oscillator::new_wrapped_with(waveform_type);
        if let Some(effect) = effect_opt {
            effect
                .borrow_mut()
                .add_audio_source(rrc_downgrade::<Oscillator>(&osc));
            o.add_audio_source(rrc_downgrade::<dyn IsEffect>(&effect));
        }
        c.add_watcher(rrc(Timer::<TestMessage>::new_with(2.0)));
        if let Some(control) = control_opt {
            c.add_watcher(rrc_clone::<dyn WatchesClock>(&control));
        }
        let samples_out = o.run_until_completion(&mut c);
        write_samples_to_wav_file(basename, sample_rate, &samples_out);
    }

    ///////////////////// DEDUPLICATE vvvvv
    pub(crate) fn write_source_to_file(source: &mut dyn SourcesAudio, basename: &str) {
        let clock_settings = ClockSettings::new_defaults();
        let mut samples = Vec::<MonoSample>::new();
        let mut clock = Clock::new_with(&clock_settings);
        while clock.seconds() < 2.0 {
            samples.push(source.source_audio(&clock));
            clock.tick();
        }
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: clock.sample_rate() as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        const AMPLITUDE: MonoSample = i16::MAX as MonoSample;
        let mut writer = hound::WavWriter::create(canonicalize_filename(basename), spec).unwrap();
        for sample in samples.iter() {
            let _ = writer.write_sample((sample * AMPLITUDE) as i16);
        }
        generate_fft_for_samples(
            &clock_settings,
            &samples,
            &canonicalize_fft_filename(basename),
        );
    }

    #[allow(dead_code)]
    pub(crate) fn write_orchestration_to_file(
        orchestrator: &mut OldTestOrchestrator,
        clock: &mut WatchedClock,
        basename: &str,
    ) {
        let samples = orchestrator.run_until_completion(clock);
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: clock.inner_clock().sample_rate() as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        const AMPLITUDE: MonoSample = i16::MAX as MonoSample;
        let mut writer = hound::WavWriter::create(canonicalize_filename(basename), spec).unwrap();
        for sample in samples.iter() {
            let _ = writer.write_sample((sample * AMPLITUDE) as i16);
        }
        generate_fft_for_samples(
            clock.inner_clock().settings(),
            &samples,
            &canonicalize_fft_filename(basename),
        );
    }

    // ```rust
    // use std::error::Error;
    // fn generate_chart(
    //     data: &Vec<(f32, f32)>,
    //     min_domain: f32,
    //     max_domain: f32,
    //     min_range: f32,
    //     max_range: f32,
    //     filename: &str,
    // ) -> Result<(), Box<dyn Error>> {
    //     let out_filename = format!("{}.png", filename);
    //     let root = BitMapBackend::new(out_filename.as_str(), (640, 360)).into_drawing_area();
    //     root.fill(&WHITE)?;

    //     let mut chart = ChartBuilder::on(&root) .margin(0)
    //         .x_label_area_size(20) .y_label_area_size(0) .build_cartesian_2d(
    //         IntoLogRange::log_scale(min_domain..max_domain),
    //         IntoLogRange::log_scale(min_range..max_range), )?;
    //         chart.configure_mesh().disable_mesh().draw()?;
    //             chart.draw_series(LineSeries::new(data.iter().map(|t| (t.0,
    //             t.1)), &BLUE))?;

    //     root.present()?;

    //     Ok(())
    // }
    // ```

    pub(crate) fn generate_fft_for_samples(
        clock_settings: &ClockSettings,
        samples: &Vec<f32>,
        _filename: &str,
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

        // ```rust
        // let _ = generate_chart(
        //     &data,
        //     0.0,
        //     clock_settings.sample_rate() as f32 / 2.0,
        //     min_y,
        //     max_y,
        //     filename,
        // );
        // ```
    }

    /// /////////////////

    impl NewUpdateable for Timer<TestMessage> {
        type Message = TestMessage;

        fn update(
            &mut self,
            clock: &Clock,
            message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            match message {
                TestMessage::Tick => {
                    self.has_more_work = clock.seconds() < self.time_to_run_seconds;
                }
                _ => {}
            }
            EvenNewerCommand::none()
        }
    }

    impl NewUpdateable for Trigger<TestMessage> {
        type Message = TestMessage;

        fn update(
            &mut self,
            clock: &Clock,
            message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            match message {
                TestMessage::Tick => {
                    return if !self.has_triggered && clock.seconds() >= self.time_to_trigger_seconds
                    {
                        self.has_triggered = true;
                        EvenNewerCommand::single(TestMessage::ControlF32(self.uid, self.value))
                    } else {
                        EvenNewerCommand::none()
                    };
                }
                _ => {}
            }
            EvenNewerCommand::none()
        }
    }

    #[derive(Display, Debug, EnumString)]
    #[strum(serialize_all = "kebab_case")]
    pub(crate) enum TestAudioSourceSetLevelControlParams {
        Level,
    }

    #[derive(Debug, Default)]
    pub struct TestAudioSourceOneLevel<M: MessageBounds> {
        uid: usize,
        level: MonoSample,
        _phantom: PhantomData<M>,
    }
    impl<M: MessageBounds> NewIsInstrument for TestAudioSourceOneLevel<M> {}
    impl<M: MessageBounds> HasUid for TestAudioSourceOneLevel<M> {
        fn uid(&self) -> usize {
            self.uid
        }

        fn set_uid(&mut self, uid: usize) {
            self.uid = uid;
        }
    }
    impl<M: MessageBounds> NewUpdateable for TestAudioSourceOneLevel<M> {
        type Message = M;
    }
    impl<M: MessageBounds> TestAudioSourceOneLevel<M> {
        pub fn new_with(level: MonoSample) -> Self {
            Self {
                level,
                ..Default::default()
            }
        }

        pub fn level(&self) -> f32 {
            self.level
        }

        pub fn set_level(&mut self, level: MonoSample) {
            self.level = level;
        }
    }
    impl<M: MessageBounds> SourcesAudio for TestAudioSourceOneLevel<M> {
        fn source_audio(&mut self, _clock: &Clock) -> MonoSample {
            self.level
        }
    }

    #[derive(Debug, Default)]
    pub struct TestAudioSourceAlwaysLoud<M: MessageBounds> {
        uid: usize,
        _phantom: PhantomData<M>,
    }
    impl<M: MessageBounds> NewIsInstrument for TestAudioSourceAlwaysLoud<M> {}
    impl<M: MessageBounds> TestAudioSourceAlwaysLoud<M> {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }
    impl<M: MessageBounds> SourcesAudio for TestAudioSourceAlwaysLoud<M> {
        fn source_audio(&mut self, _clock: &Clock) -> MonoSample {
            MONO_SAMPLE_MAX
        }
    }
    impl<M: MessageBounds> NewUpdateable for TestAudioSourceAlwaysLoud<M> {
        type Message = M;
    }
    impl<M: MessageBounds> HasUid for TestAudioSourceAlwaysLoud<M> {
        fn uid(&self) -> usize {
            self.uid
        }

        fn set_uid(&mut self, uid: usize) {
            self.uid = uid;
        }
    }

    #[derive(Debug, Default)]
    pub struct TestAudioSourceAlwaysTooLoud<M: MessageBounds> {
        uid: usize,
        _phantom: PhantomData<M>,
    }
    impl<M: MessageBounds> NewIsInstrument for TestAudioSourceAlwaysTooLoud<M> {}
    impl<M: MessageBounds> TestAudioSourceAlwaysTooLoud<M> {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }
    impl<M: MessageBounds> SourcesAudio for TestAudioSourceAlwaysTooLoud<M> {
        fn source_audio(&mut self, _clock: &Clock) -> MonoSample {
            MONO_SAMPLE_MAX + 0.1
        }
    }
    impl<M: MessageBounds> HasUid for TestAudioSourceAlwaysTooLoud<M> {
        fn uid(&self) -> usize {
            self.uid
        }

        fn set_uid(&mut self, uid: usize) {
            self.uid = uid
        }
    }
    impl<M: MessageBounds> NewUpdateable for TestAudioSourceAlwaysTooLoud<M> {
        type Message = M;
    }

    #[derive(Debug, Default)]
    pub struct TestAudioSourceAlwaysSilent<M: MessageBounds> {
        uid: usize,
        _phantom: PhantomData<M>,
    }
    impl<M: MessageBounds> NewIsInstrument for TestAudioSourceAlwaysSilent<M> {}
    impl<M: MessageBounds> TestAudioSourceAlwaysSilent<M> {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }
    impl<M: MessageBounds> SourcesAudio for TestAudioSourceAlwaysSilent<M> {
        fn source_audio(&mut self, _clock: &Clock) -> MonoSample {
            MONO_SAMPLE_SILENCE
        }
    }
    impl<M: MessageBounds> HasUid for TestAudioSourceAlwaysSilent<M> {
        fn uid(&self) -> usize {
            self.uid
        }

        fn set_uid(&mut self, uid: usize) {
            self.uid = uid
        }
    }
    impl<M: MessageBounds> NewUpdateable for TestAudioSourceAlwaysSilent<M> {
        type Message = M;
    }

    #[derive(Debug, Default)]
    pub struct TestAudioSourceAlwaysVeryQuiet<M: MessageBounds> {
        uid: usize,
        _phantom: PhantomData<M>,
    }
    impl<M: MessageBounds> NewIsInstrument for TestAudioSourceAlwaysVeryQuiet<M> {}
    impl<M: MessageBounds> TestAudioSourceAlwaysVeryQuiet<M> {
        #[allow(dead_code)]
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }
    impl<M: MessageBounds> SourcesAudio for TestAudioSourceAlwaysVeryQuiet<M> {
        fn source_audio(&mut self, _clock: &Clock) -> MonoSample {
            MONO_SAMPLE_MIN
        }
    }
    impl<M: MessageBounds> HasUid for TestAudioSourceAlwaysVeryQuiet<M> {
        fn uid(&self) -> usize {
            self.uid
        }

        fn set_uid(&mut self, uid: usize) {
            self.uid = uid;
        }
    }
    impl<M: MessageBounds> NewUpdateable for TestAudioSourceAlwaysVeryQuiet<M> {
        type Message = M;
    }

    #[derive(Debug)]
    pub struct OldTestOrchestrator {
        pub main_mixer: Box<dyn IsEffect>,
        pub updateables: HashMap<usize, Ww<dyn SinksUpdates>>,

        // The final clock watcher gets to run and test the state resulting from
        // all the prior clock watchers' ticks.
        pub final_clock_watcher: Option<Ww<dyn WatchesClock>>,
    }

    impl Default for OldTestOrchestrator {
        fn default() -> Self {
            Self {
                main_mixer: Box::new(Mixer::<TestMessage>::new()),
                updateables: Default::default(),
                final_clock_watcher: None,
            }
        }
    }

    impl OldTestOrchestrator {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }

        #[allow(dead_code)]
        pub fn new_with(main_mixer: Box<dyn IsEffect>) -> Self {
            Self {
                main_mixer,
                ..Default::default()
            }
        }

        pub fn add_audio_source(&mut self, source: Ww<dyn SourcesAudio>) {
            self.main_mixer.add_audio_source(source);
        }

        pub fn add_final_watcher(&mut self, watcher: Rrc<dyn WatchesClock>) {
            self.final_clock_watcher = Some(rrc_downgrade(&watcher));
        }

        pub fn run_until_completion(&mut self, clock: &mut WatchedClock) -> Vec<MonoSample> {
            let mut samples_out = Vec::new();
            loop {
                let (mut done, messages) = clock.visit_watchers();
                if let Some(watcher) = &self.final_clock_watcher {
                    if let Some(watcher) = watcher.upgrade() {
                        watcher.borrow_mut().tick(&clock.inner_clock());
                        done = done && watcher.borrow().is_finished();
                    }
                }
                if done {
                    break;
                }
                for message in messages {
                    match message {
                        crate::controllers::BigMessage::SmallMessage(uid, message) => {
                            if let Some(target) = self.updateables.get_mut(&uid) {
                                if let Some(target) = target.upgrade() {
                                    target.borrow_mut().update(clock.inner_clock(), message);
                                }
                            }
                        }
                    }
                }
                samples_out.push(self.main_mixer.source_audio(clock.inner_clock()));
                clock.tick();
            }
            samples_out
        }
    }

    #[derive(Debug, Default)]
    pub struct TestMixer<M: MessageBounds> {
        uid: usize,

        _phantom: PhantomData<M>,
    }
    impl<M: MessageBounds> NewIsEffect for TestMixer<M> {}
    impl<M: MessageBounds> HasUid for TestMixer<M> {
        fn uid(&self) -> usize {
            self.uid
        }

        fn set_uid(&mut self, uid: usize) {
            self.uid = uid;
        }
    }
    impl<M: MessageBounds> TransformsAudio for TestMixer<M> {
        fn transform_audio(&mut self, _clock: &Clock, input_sample: MonoSample) -> MonoSample {
            input_sample
        }
    }
    impl<M: MessageBounds> NewUpdateable for TestMixer<M> {
        type Message = M;
    }

    #[derive(Display, Debug, EnumString)]
    #[strum(serialize_all = "kebab_case")]
    pub(crate) enum TestLfoControlParams {
        Frequency,
    }

    #[derive(Debug, Default)]
    pub struct TestLfo<M: MessageBounds> {
        uid: usize,
        oscillator: Oscillator,
        _phantom: PhantomData<M>,
    }
    impl<M: MessageBounds> NewIsController for TestLfo<M> {}
    impl<M: MessageBounds> HasUid for TestLfo<M> {
        fn uid(&self) -> usize {
            self.uid
        }

        fn set_uid(&mut self, uid: usize) {
            self.uid = uid;
        }
    }
    impl<M: MessageBounds> NewUpdateable for TestLfo<M> {
        default type Message = M;

        default fn update(
            &mut self,
            _clock: &Clock,
            _message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            EvenNewerCommand::none()
        }

        default fn param_id_for_name(&self, _param_name: &str) -> usize {
            usize::MAX
        }
    }
    impl NewUpdateable for TestLfo<TestMessage> {
        type Message = TestMessage;

        fn update(
            &mut self,
            clock: &Clock,
            message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            if let TestMessage::Tick = message {
                let value = self.oscillator.source_audio(&clock);
                EvenNewerCommand::single(TestMessage::ControlF32(self.uid, value))
            } else {
                EvenNewerCommand::none()
            }
        }

        fn param_id_for_name(&self, param_name: &str) -> usize {
            if let Ok(param) = TestLfoControlParams::from_str(param_name) {
                param as usize
            } else {
                0
            }
        }
    }
    impl<M: MessageBounds> Terminates for TestLfo<M> {
        // This hardcoded value is OK because an LFO doesn't have a defined
        // beginning/end. It just keeps going. Yet it truly is a controller.
        fn is_finished(&self) -> bool {
            true
        }
    }
    impl<M: MessageBounds> TestLfo<M> {
        fn set_frequency(&mut self, frequency_hz: f32) {
            self.oscillator.set_frequency(frequency_hz);
        }
    }

    #[derive(Debug, Default)]
    pub struct TestNegatingEffect<M: MessageBounds> {
        uid: usize,
        _phantom: PhantomData<M>,
    }
    impl<M: MessageBounds> NewIsEffect for TestNegatingEffect<M> {}
    impl<M: MessageBounds> HasUid for TestNegatingEffect<M> {
        fn uid(&self) -> usize {
            self.uid
        }

        fn set_uid(&mut self, uid: usize) {
            self.uid = uid;
        }
    }
    impl<M: MessageBounds> NewUpdateable for TestNegatingEffect<M> {
        type Message = M;
    }
    impl<M: MessageBounds> TransformsAudio for TestNegatingEffect<M> {
        fn transform_audio(&mut self, _clock: &Clock, input_sample: MonoSample) -> MonoSample {
            -input_sample
        }
    }

    #[derive(Display, Debug, EnumString, FromRepr)]
    #[strum(serialize_all = "kebab_case")]
    pub enum TestSynthControlParams {
        OscillatorModulation,
    }

    #[derive(Debug)]
    pub struct TestSynth<M: MessageBounds> {
        uid: usize,

        oscillator: Box<Oscillator>,
        envelope: Rrc<dyn SourcesAudio>,
        _phantom: PhantomData<M>,
    }

    impl<M: MessageBounds> TestSynth<M> {
        /// You really don't want to call this, because you need a sample rate
        /// for it to do anything meaningful, and it's a bad practice to
        /// hardcode a 44.1KHz rate.
        #[deprecated]
        #[allow(dead_code)]
        fn new() -> Self {
            Self::new_with(
                Box::new(Oscillator::new()),
                rrc(AdsrEnvelope::new_with(&EnvelopeSettings::default())),
            )
        }
        pub fn new_with(oscillator: Box<Oscillator>, envelope: Rrc<dyn SourcesAudio>) -> Self {
            Self {
                oscillator,
                envelope,
                ..Default::default()
            }
        }
    }
    impl<M: MessageBounds> Default for TestSynth<M> {
        fn default() -> Self {
            Self {
                uid: 0,
                oscillator: Box::new(Oscillator::new()),
                envelope: rrc(AdsrEnvelope::new_with(&EnvelopeSettings::default())),
                _phantom: Default::default(),
            }
        }
    }

    impl<M: MessageBounds> SourcesAudio for TestSynth<M> {
        fn source_audio(&mut self, clock: &Clock) -> MonoSample {
            self.oscillator.source_audio(clock) * self.envelope.borrow_mut().source_audio(clock)
        }
    }

    impl<M: MessageBounds> NewIsInstrument for TestSynth<M> {}
    impl<M: MessageBounds> NewUpdateable for TestSynth<M> {
        default type Message = M;

        default fn update(
            &mut self,
            _clock: &Clock,
            _message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            EvenNewerCommand::none()
        }

        default fn handle_message(&mut self, _clock: &Clock, _message: Self::Message) {
            todo!()
        }

        default fn param_id_for_name(&self, _param_name: &str) -> usize {
            usize::MAX
        }
    }
    impl NewUpdateable for TestSynth<TestMessage> {
        type Message = TestMessage;

        fn update(
            &mut self,
            _clock: &Clock,
            message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            match message {
                TestMessage::UpdateF32(param_index, value) => {
                    if let Some(param) = TestSynthControlParams::from_repr(param_index) {
                        match param {
                            TestSynthControlParams::OscillatorModulation => {
                                self.oscillator.set_frequency_modulation(value);
                            }
                        }
                    }
                }
                _ => todo!(),
            }
            EvenNewerCommand::none()
        }

        fn param_id_for_name(&self, param_name: &str) -> usize {
            if let Ok(param) = TestSynthControlParams::from_str(param_name) {
                param as usize
            } else {
                0
            }
        }
    }
    impl NewUpdateable for TestSynth<GrooveMessage> {
        type Message = GrooveMessage;

        fn update(
            &mut self,
            _clock: &Clock,
            message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            match message {
                GrooveMessage::UpdateF32(param_index, value) => {
                    if let Some(param) = TestSynthControlParams::from_repr(param_index) {
                        match param {
                            TestSynthControlParams::OscillatorModulation => {
                                self.oscillator.set_frequency_modulation(value);
                            }
                        }
                    }
                }
                _ => todo!(),
            }
            EvenNewerCommand::none()
        }
        fn param_id_for_name(&self, param_name: &str) -> usize {
            if let Ok(param) = TestSynthControlParams::from_str(param_name) {
                param as usize
            } else {
                0
            }
        }
    }

    impl<M: MessageBounds> HasUid for TestSynth<M> {
        fn uid(&self) -> usize {
            self.uid
        }

        fn set_uid(&mut self, uid: usize) {
            self.uid = uid;
        }
    }

    /// Lets a SourcesAudio act like an IsController
    #[derive(Debug)]
    pub struct TestControlSourceContinuous<M: MessageBounds> {
        uid: usize,
        source: Box<dyn SourcesAudio>,

        _phantom: PhantomData<M>,
    }
    impl<M: MessageBounds> NewIsController for TestControlSourceContinuous<M> {}
    impl<M: MessageBounds> NewUpdateable for TestControlSourceContinuous<M> {
        default type Message = M;

        default fn update(
            &mut self,
            _clock: &Clock,
            _message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            EvenNewerCommand::none()
        }
    }
    impl NewUpdateable for TestControlSourceContinuous<TestMessage> {
        type Message = TestMessage;

        fn update(
            &mut self,
            clock: &Clock,
            message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            match message {
                TestMessage::Tick => {
                    let value = self.source.source_audio(&clock).abs();
                    EvenNewerCommand::single(TestMessage::ControlF32(self.uid, value))
                }
                _ => EvenNewerCommand::none(),
            }
        }
    }
    impl<M: MessageBounds> Terminates for TestControlSourceContinuous<M> {
        fn is_finished(&self) -> bool {
            true
        }
    }
    impl<M: MessageBounds> HasUid for TestControlSourceContinuous<M> {
        fn uid(&self) -> usize {
            self.uid
        }

        fn set_uid(&mut self, uid: usize) {
            self.uid = uid;
        }
    }
    impl<M: MessageBounds> TestControlSourceContinuous<M> {
        pub fn new_with(source: Box<dyn SourcesAudio>) -> Self {
            Self {
                uid: usize::default(),
                source,
                _phantom: PhantomData::default(),
            }
        }
    }
    #[derive(Display, Debug, EnumString)]
    #[strum(serialize_all = "kebab_case")]
    pub(crate) enum TestMidiSinkControlParams {
        Param,
    }

    /// Helper for testing SinksMidi
    #[derive(Debug, Default)]
    pub struct TestMidiSink<M: MessageBounds> {
        uid: usize,
        pub(crate) me: Ww<Self>,
        overhead: Overhead,

        pub is_playing: bool,
        midi_channel: MidiChannel,
        pub received_count: usize,
        pub handled_count: usize,
        pub value: f32,

        pub messages: Vec<(f32, MidiChannel, MidiMessage)>,
    }

    impl<M: MessageBounds> TestMidiSink<M> {
        pub const TEST_MIDI_CHANNEL: u8 = 42;

        pub fn new() -> Self {
            Self {
                midi_channel: Self::TEST_MIDI_CHANNEL,
                ..Default::default()
            }
        }
        pub fn new_wrapped() -> Rrc<Self> {
            let wrapped = rrc(Self::new());
            wrapped.borrow_mut().me = rrc_downgrade(&wrapped);
            wrapped
        }
        pub fn new_with(midi_channel: MidiChannel) -> Self {
            Self {
                midi_channel,
                ..Default::default()
            }
        }
        #[allow(dead_code)]
        pub fn new_wrapped_with(midi_channel: MidiChannel) -> Rrc<Self> {
            let wrapped = rrc(Self::new_with(midi_channel));
            wrapped.borrow_mut().me = rrc_downgrade(&wrapped);
            wrapped
        }
        #[allow(dead_code)]
        pub fn set_value(&mut self, value: f32) {
            self.value = value;
        }

        #[allow(dead_code)]
        pub fn dump_messages(&self) {
            dbg!(&self.messages);
        }
    }

    impl<M: MessageBounds> SinksMidi for TestMidiSink<M> {
        fn midi_channel(&self) -> MidiChannel {
            self.midi_channel
        }

        fn set_midi_channel(&mut self, midi_channel: MidiChannel) {
            self.midi_channel = midi_channel;
        }
        fn handle_midi_for_channel(
            &mut self,
            clock: &Clock,
            channel: &MidiChannel,
            message: &MidiMessage,
        ) {
            assert_eq!(self.midi_channel, *channel);
            self.messages.push((clock.beats(), *channel, *message));
            self.received_count += 1;

            #[allow(unused_variables)]
            match message {
                MidiMessage::NoteOff { key, vel } => {
                    self.is_playing = false;
                    self.handled_count += 1;
                }
                MidiMessage::NoteOn { key, vel } => {
                    self.is_playing = true;
                    self.handled_count += 1;
                }
                MidiMessage::Aftertouch { key, vel } => todo!(),
                MidiMessage::Controller { controller, value } => todo!(),
                MidiMessage::ProgramChange { program } => {
                    self.handled_count += 1;
                }
                MidiMessage::ChannelAftertouch { vel } => todo!(),
                MidiMessage::PitchBend { bend } => todo!(),
            }
        }
    }
    impl<M: MessageBounds> SourcesAudio for TestMidiSink<M> {
        fn source_audio(&mut self, _clock: &Clock) -> MonoSample {
            self.value
        }
    }
    impl<M: MessageBounds> HasOverhead for TestMidiSink<M> {
        fn overhead(&self) -> &Overhead {
            &self.overhead
        }

        fn overhead_mut(&mut self) -> &mut Overhead {
            &mut self.overhead
        }
    }
    impl<M: MessageBounds> SinksUpdates for TestMidiSink<M> {
        fn update(&mut self, _clock: &Clock, message: SmallMessage) {
            match message {
                SmallMessage::ValueChanged(value) => {
                    self.value = value;
                }
                _ => {
                    dbg!(&message);
                }
            }
        }

        fn message_for(&self, _param: &str) -> SmallMessageGenerator {
            todo!()
        }
    }

    #[derive(Debug)]
    pub struct TestInstrument<M: MessageBounds> {
        uid: usize,
        pub(crate) me: Ww<Self>,
        overhead: Overhead,

        sound_source: Oscillator,
        pub is_playing: bool,
        midi_channel: MidiChannel,
        pub received_count: usize,
        pub handled_count: usize,

        pub debug_messages: Vec<(f32, MidiChannel, MidiMessage)>,
    }
    impl<M: MessageBounds> NewIsInstrument for TestInstrument<M> {}
    impl<M: MessageBounds> NewUpdateable for TestInstrument<M> {
        default type Message = M;

        default fn update(
            &mut self,
            _clock: &Clock,
            _message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            EvenNewerCommand::none()
        }
    }
    impl NewUpdateable for TestInstrument<TestMessage> {
        type Message = TestMessage;

        fn update(
            &mut self,
            clock: &Clock,
            message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            match message {
                TestMessage::Midi(channel, message) => {
                    self.new_handle_midi(clock, channel, message);
                }
                _ => todo!(),
            }
            EvenNewerCommand::none()
        }
    }
    impl<M: MessageBounds> HasUid for TestInstrument<M> {
        fn uid(&self) -> usize {
            self.uid
        }

        fn set_uid(&mut self, uid: usize) {
            self.uid = uid;
        }
    }
    impl<M: MessageBounds> Default for TestInstrument<M> {
        fn default() -> Self {
            Self {
                uid: Default::default(),
                me: Default::default(),
                overhead: Default::default(),
                sound_source: Default::default(),
                is_playing: Default::default(),
                midi_channel: Self::TEST_MIDI_CHANNEL,
                received_count: Default::default(),
                handled_count: Default::default(),
                debug_messages: Default::default(),
            }
        }
    }
    impl<M: MessageBounds> TestInstrument<M> {
        pub const TEST_MIDI_CHANNEL: u8 = 42;

        pub fn new() -> Self {
            Self {
                midi_channel: Self::TEST_MIDI_CHANNEL,
                ..Default::default()
            }
        }
        pub fn new_wrapped() -> Rrc<Self> {
            let wrapped = rrc(Self::new());
            wrapped.borrow_mut().me = rrc_downgrade(&wrapped);
            wrapped
        }
        pub fn new_with(midi_channel: MidiChannel) -> Self {
            Self {
                midi_channel,
                ..Default::default()
            }
        }
        #[allow(dead_code)]
        pub fn new_wrapped_with(midi_channel: MidiChannel) -> Rrc<Self> {
            let wrapped = rrc(Self::new_with(midi_channel));
            wrapped.borrow_mut().me = rrc_downgrade(&wrapped);
            wrapped
        }

        #[allow(dead_code)]
        pub fn dump_messages(&self) {
            dbg!(&self.debug_messages);
        }

        fn new_handle_midi(&mut self, clock: &Clock, channel: MidiChannel, message: MidiMessage) {
            assert_eq!(self.midi_channel, channel);
            self.debug_messages.push((clock.beats(), channel, message));
            self.received_count += 1;

            match message {
                MidiMessage::NoteOn { key, vel: _ } => {
                    self.is_playing = true;
                    self.sound_source
                        .set_frequency(MidiUtils::note_to_frequency(key.as_int()));
                }
                MidiMessage::NoteOff { key: _, vel: _ } => {
                    self.is_playing = false;
                }
                _ => {}
            }
        }
    }

    impl<M: MessageBounds> SinksMidi for TestInstrument<M> {
        fn midi_channel(&self) -> MidiChannel {
            self.midi_channel
        }

        fn set_midi_channel(&mut self, midi_channel: MidiChannel) {
            self.midi_channel = midi_channel;
        }
        fn handle_midi_for_channel(
            &mut self,
            clock: &Clock,
            channel: &MidiChannel,
            message: &MidiMessage,
        ) {
            assert_eq!(self.midi_channel, *channel);
            self.debug_messages
                .push((clock.beats(), *channel, *message));
            self.received_count += 1;

            #[allow(unused_variables)]
            match message {
                MidiMessage::NoteOff { key, vel } => {
                    self.is_playing = false;
                    self.handled_count += 1;
                }
                MidiMessage::NoteOn { key, vel } => {
                    self.is_playing = true;
                    self.handled_count += 1;
                }
                MidiMessage::Aftertouch { key, vel } => todo!(),
                MidiMessage::Controller { controller, value } => todo!(),
                MidiMessage::ProgramChange { program } => {
                    self.handled_count += 1;
                }
                MidiMessage::ChannelAftertouch { vel } => todo!(),
                MidiMessage::PitchBend { bend } => todo!(),
            }
        }
    }
    impl<M: MessageBounds> SourcesAudio for TestInstrument<M> {
        fn source_audio(&mut self, clock: &Clock) -> MonoSample {
            if self.is_playing {
                self.sound_source.source_audio(clock)
            } else {
                MONO_SAMPLE_SILENCE
            }
        }
    }
    impl<M: MessageBounds> HasOverhead for TestInstrument<M> {
        fn overhead(&self) -> &Overhead {
            &self.overhead
        }

        fn overhead_mut(&mut self) -> &mut Overhead {
            &mut self.overhead
        }
    }

    /// Keeps asking for time slices until end of specified lifetime.
    #[derive(Debug, Default)]
    pub struct TestClockWatcher<M: MessageBounds> {
        has_more_work: bool,
        lifetime_seconds: f32,

        _phantom: PhantomData<M>,
    }

    impl<M: MessageBounds> WatchesClock for TestClockWatcher<M> {
        fn tick(&mut self, clock: &Clock) -> Vec<BigMessage> {
            self.has_more_work = clock.seconds() < self.lifetime_seconds;
            Vec::new()
        }
    }

    impl<M: MessageBounds> Terminates for TestClockWatcher<M> {
        fn is_finished(&self) -> bool {
            !self.has_more_work
        }
    }

    impl<M: MessageBounds> TestClockWatcher<M> {
        pub fn new(lifetime_seconds: f32) -> Self {
            Self {
                lifetime_seconds,
                ..Default::default()
            }
        }
    }

    #[derive(Display, Debug, EnumString)]
    #[strum(serialize_all = "kebab_case")]
    pub(crate) enum TestArpeggiatorControlParams {
        Tempo,
    }

    enum TestArpeggiatorAction {
        Nothing,
        NoteOn,
        NoteOff,
    }

    #[derive(Debug, Default)]
    pub struct TestArpeggiator<M: MessageBounds> {
        uid: usize,
        me: Ww<Self>,
        midi_channel_out: MidiChannel,
        pub tempo: f32,
        is_enabled: bool,
        is_playing: bool,
        channels_to_sink_vecs: HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>>,
    }
    impl<M: MessageBounds> SourcesMidi for TestArpeggiator<M> {
        fn midi_sinks(&self) -> &HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>> {
            &self.channels_to_sink_vecs
        }
        fn midi_sinks_mut(&mut self) -> &mut HashMap<MidiChannel, Vec<Ww<dyn SinksMidi>>> {
            &mut self.channels_to_sink_vecs
        }

        fn midi_output_channel(&self) -> MidiChannel {
            self.midi_channel_out
        }

        fn set_midi_output_channel(&mut self, midi_channel: MidiChannel) {
            self.midi_channel_out = midi_channel;
        }
    }
    impl<M: MessageBounds> WatchesClock for TestArpeggiator<M> {
        fn tick(&mut self, clock: &Clock) -> Vec<BigMessage> {
            // We don't actually pay any attention to self.tempo, but it's easy
            // enough to see that tempo could have influenced this MIDI message.
            self.issue_midi(
                clock,
                &self.midi_channel_out,
                &MidiUtils::new_note_on(60, 100),
            );
            Vec::new()
        }
    }
    impl<M: MessageBounds> Terminates for TestArpeggiator<M> {
        fn is_finished(&self) -> bool {
            true
        }
    }
    impl<M: MessageBounds> SinksUpdates for TestArpeggiator<M> {
        fn message_for(&self, param: &str) -> SmallMessageGenerator {
            assert_eq!(
                TestArpeggiatorControlParams::Tempo.to_string().as_str(),
                param
            );
            Box::new(SmallMessage::ValueChanged)
        }

        fn update(&mut self, _clock: &Clock, message: SmallMessage) {
            match message {
                SmallMessage::ValueChanged(value) => self.tempo = value,
                _ => todo!(),
            }
        }
    }
    impl<M: MessageBounds> NewIsController for TestArpeggiator<M> {}
    impl<M: MessageBounds> NewUpdateable for TestArpeggiator<M> {
        default type Message = M;

        default fn update(
            &mut self,
            _clock: &Clock,
            _message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            EvenNewerCommand::none()
        }
    }
    impl NewUpdateable for TestArpeggiator<TestMessage> {
        type Message = TestMessage;

        fn update(
            &mut self,
            clock: &Clock,
            message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            match message {
                TestMessage::Tick => match self.what_to_do(clock) {
                    TestArpeggiatorAction::Nothing => {
                        return EvenNewerCommand::none();
                    }
                    TestArpeggiatorAction::NoteOn => {
                        // This is elegant, I hope. If the arpeggiator is
                        // disabled during play, and we were playing a note,
                        // then we still send the off note,
                        return if self.is_enabled {
                            self.is_playing = true;
                            EvenNewerCommand::single(TestMessage::Midi(
                                self.midi_channel_out,
                                MidiMessage::NoteOn {
                                    key: 60.into(),
                                    vel: 127.into(),
                                },
                            ))
                        } else {
                            EvenNewerCommand::none()
                        };
                    }
                    TestArpeggiatorAction::NoteOff => {
                        return if self.is_playing {
                            EvenNewerCommand::single(TestMessage::Midi(
                                self.midi_channel_out,
                                MidiMessage::NoteOff {
                                    key: 60.into(),
                                    vel: 0.into(),
                                },
                            ))
                        } else {
                            EvenNewerCommand::none()
                        };
                    }
                },
                TestMessage::Enable(enabled) => {
                    self.is_enabled = enabled;
                    EvenNewerCommand::none()
                }
                _ => todo!(),
            }
        }
    }
    impl<M: MessageBounds> HasUid for TestArpeggiator<M> {
        fn uid(&self) -> usize {
            self.uid
        }

        fn set_uid(&mut self, uid: usize) {
            self.uid = uid
        }
    }
    impl<M: MessageBounds> TestArpeggiator<M> {
        pub fn new_with(midi_channel_out: MidiChannel) -> Self {
            Self {
                midi_channel_out,
                ..Default::default()
            }
        }
        pub fn new_wrapped_with(midi_channel_out: MidiChannel) -> Rrc<Self> {
            let wrapped = rrc(Self::new_with(midi_channel_out));
            wrapped.borrow_mut().me = rrc_downgrade(&wrapped);
            wrapped
        }

        fn what_to_do(&self, clock: &Clock) -> TestArpeggiatorAction {
            let beat_slice_start = clock.beats();
            let beat_slice_end = clock.next_slice_in_beats();
            let next_exact_beat = beat_slice_start.floor();
            let next_exact_half_beat = next_exact_beat + 0.5;
            if next_exact_beat >= beat_slice_start && next_exact_beat < beat_slice_end {
                return TestArpeggiatorAction::NoteOn;
            }
            if next_exact_half_beat >= beat_slice_start && next_exact_half_beat < beat_slice_end {
                return TestArpeggiatorAction::NoteOff;
            }
            return TestArpeggiatorAction::Nothing;
        }
    }

    #[derive(Debug, Default)]
    pub struct TestValueChecker<M: MessageBounds> {
        uid: usize,
        pub values: VecDeque<f32>,
        pub target_uid: usize,
        pub checkpoint: f32,
        pub checkpoint_delta: f32,
        pub time_unit: ClockTimeUnit,
        _phantom: PhantomData<M>,
    }
    impl<M: MessageBounds> NewIsEffect for TestValueChecker<M> {}
    impl<M: MessageBounds> TransformsAudio for TestValueChecker<M> {
        fn transform_audio(&mut self, _clock: &Clock, _input_sample: MonoSample) -> MonoSample {
            todo!()
        }
    }
    impl<M: MessageBounds> NewUpdateable for TestValueChecker<M> {
        default type Message = M;

        default fn update(
            &mut self,
            _clock: &Clock,
            _message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            EvenNewerCommand::none()
        }
    }
    impl NewUpdateable for TestValueChecker<TestMessage> {
        type Message = TestMessage;

        fn update(
            &mut self,
            clock: &Clock,
            message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            match message {
                TestMessage::Tick => {
                    if !self.values.is_empty() {
                        if clock.time_for(&self.time_unit) >= self.checkpoint {
                            const SAD_FLOAT_DIFF: f32 = 1.0e-4;
                            assert_approx_eq!(
                                1000.0, // TODO TODO
                                //      self.target_uid.source_audio(clock),
                                self.values[0],
                                SAD_FLOAT_DIFF
                            );
                            self.checkpoint += self.checkpoint_delta;
                            self.values.pop_front();
                        }
                    }
                }
                _ => todo!(),
            }
            EvenNewerCommand::none()
        }
    }
    impl<M: MessageBounds> HasUid for TestValueChecker<M> {
        fn uid(&self) -> usize {
            self.uid
        }

        fn set_uid(&mut self, uid: usize) {
            self.uid = uid;
        }
    }

    impl<M: MessageBounds> Terminates for TestValueChecker<M> {
        fn is_finished(&self) -> bool {
            self.values.is_empty()
        }
    }
    impl<M: MessageBounds> TestValueChecker<M> {
        pub(crate) fn new_with(
            values: &[f32],
            target_uid: usize,
            checkpoint: f32,
            checkpoint_delta: f32,
            time_unit: ClockTimeUnit,
        ) -> Self {
            Self {
                values: VecDeque::from(Vec::from(values)),
                target_uid,
                checkpoint,
                checkpoint_delta,
                time_unit,
                ..Default::default()
            }
        }
    }

    // GrooveMessage::Nop => {
    //     dbg!(clock, message);
    // }
    // GrooveMessage::Tick => panic!("GrooveMessage::Tick should be sent only by the system"),
    // GrooveMessage::ControlF32(uid, value) => ,
    // GrooveMessage::UpdateF32(param_id, value) => panic!(
    //     "GrooveMessage::UpdateF32({}, {}) should be dispatched by Orchestrator, not received by it",param_id,value            ),
    // GrooveMessage::Midi(channel, message) => self.send_midi_message(clock, channel, message),

    #[test]
    fn test_audio_routing() {
        let mut o = Box::new(Orchestrator::<TestMessage>::default());

        // A simple audio source.
        let synth_uid = o.add(
            None,
            BoxedEntity::Instrument(Box::new(TestSynth::<TestMessage>::default())),
        );

        // A simple effect.
        let effect_uid = o.add(
            None,
            BoxedEntity::Effect(Box::new(TestNegatingEffect::<TestMessage>::default())),
        );

        // Connect the audio's output to the effect's input.
        assert!(o.patch(synth_uid, effect_uid).is_ok());

        // And patch the effect into the main mixer.
        o.connect_to_main_mixer(effect_uid);

        // Run the main loop for a while.
        const SECONDS: usize = 1;
        let _ = o.add(
            None,
            BoxedEntity::Controller(Box::new(Timer::<TestMessage>::new_with(SECONDS as f32))),
        );

        // Gather the audio output.
        let mut runner = Runner::default();
        let mut clock = Clock::new();
        if let Ok(samples_1) = runner.run(&mut o, &mut clock) {
            // We should get exactly the right amount of audio.
            assert_eq!(samples_1.len(), SECONDS * clock.sample_rate());

            // It should not all be silence.
            assert!(!samples_1.iter().any(|&s| s != MONO_SAMPLE_SILENCE));

            // Run again but without the negating effect in the mix.
            o.unpatch(synth_uid, effect_uid);
            clock.reset();
            if let Ok(samples_2) = runner.run(&mut o, &mut clock) {
                // The sample pairs should cancel each other out.
                assert!(!samples_2.iter().any(|&s| s != MONO_SAMPLE_SILENCE));
                samples_1.iter().zip(samples_2.iter()).all(|(a, b)| {
                    *a + *b == MONO_SAMPLE_SILENCE && (*a == MONO_SAMPLE_SILENCE || *a != *b)
                });
            }
        }
    }

    // TODO: I had a bug for a day where I'd swapped the param_names for the
    // first and second audio inputs. In other words, the moment I got out
    // of the type system, I failed. Consider taking a more strongly typed
    // argument as an alternative to the (necessary) string argument.

    #[test]
    fn test_control_routing() {
        let mut o = Box::new(Orchestrator::<TestMessage>::default());

        // The synth's frequency is modulated by the LFO.
        let synth_1_uid = o.add(
            None,
            BoxedEntity::Instrument(Box::new(TestSynth::<TestMessage>::default())),
        );
        let mut lfo = TestLfo::<TestMessage>::default();
        lfo.set_frequency(2.0);
        let lfo_uid = o.add(None, BoxedEntity::Controller(Box::new(lfo)));
        o.link_control(
            lfo_uid,
            synth_1_uid,
            &TestSynthControlParams::OscillatorModulation.to_string(),
        );

        // We'll hear the synth's audio output.
        o.connect_to_main_mixer(synth_1_uid);

        const SECONDS: usize = 1;
        let _ = o.add(
            None,
            BoxedEntity::Controller(Box::new(Timer::<TestMessage>::new_with(SECONDS as f32))),
        );

        // Gather the audio output.
        let mut runner = Runner::default();
        let mut clock = Clock::new();
        if let Ok(samples_1) = runner.run(&mut o, &mut clock) {
            // We should get exactly the right amount of audio.
            assert_eq!(samples_1.len(), SECONDS * clock.sample_rate());

            // It should not all be silence.
            assert!(!samples_1.iter().any(|&s| s != MONO_SAMPLE_SILENCE));

            // Run again after disconnecting the LFO.
            o.unlink_control(lfo_uid, synth_1_uid);
            clock.reset();
            if let Ok(samples_2) = runner.run(&mut o, &mut clock) {
                // The two runs should be different. That's not a great test of what
                // we're doing here, but it will detect when things are broken.
                samples_1
                    .iter()
                    .zip(samples_2.iter())
                    .any(|(a, b)| *a != *b);
            }
        }
    }

    #[test]
    fn test_midi_routing() {
        let mut o = Box::new(Orchestrator::<TestMessage>::default());

        // We have a regular MIDI instrument, and an arpeggiator that emits MIDI note messages.
        let instrument_uid = o.add(
            None,
            BoxedEntity::Instrument(Box::new(TestInstrument::<TestMessage>::default())),
        );
        let arpeggiator_uid = o.add(
            None,
            BoxedEntity::Controller(Box::new(TestArpeggiator::<TestMessage>::new_with(
                TestInstrument::<TestMessage>::TEST_MIDI_CHANNEL,
            ))),
        );

        // We'll hear the instrument.
        assert!(o.connect_to_main_mixer(instrument_uid).is_ok());

        // This might not be necessary. We will automatically get every MIDI
        // message sent.
        o.connect_midi_upstream(arpeggiator_uid);
        o.connect_midi_downstream(
            instrument_uid,
            TestInstrument::<TestMessage>::TEST_MIDI_CHANNEL,
        );

        const SECONDS: usize = 1;
        let _ = o.add(
            None,
            BoxedEntity::Controller(Box::new(Timer::<TestMessage>::new_with(SECONDS as f32))),
        );

        // Everything is hooked up. Let's run it and hear what we got.
        let mut runner = Runner::default();
        let mut clock = Clock::new();
        if let Ok(samples) = runner.run(&mut o, &mut clock) {
            // We haven't asked the arpeggiator to start sending anything yet.
            assert!(
                samples.iter().all(|&s| s == MONO_SAMPLE_SILENCE),
                "Expected total silence because the arpeggiator is not turned on."
            );
        } else {
            assert!(false, "impossible!");
        }

        // Let's turn on the arpeggiator.
        runner.send_msg_enable(&mut o, &clock, arpeggiator_uid, true);
        clock.reset();
        if let Ok(samples) = runner.run(&mut o, &mut clock) {
            assert!(
                samples.iter().any(|&s| s != MONO_SAMPLE_SILENCE),
                "Expected some sound because the arpeggiator is now running."
            );
        } else {
            assert!(false, "impossible!");
        }

        // The arpeggiator is still running. Let's disable it (taking advantage
        // of the fact that TestInstrument has zero release time, because
        // otherwise it would keep generating a bit of sound even after the
        // arpeggiator told it to stop).
        //
        // Note that we're implicitly testing that the arpeggiator will send a
        // note-off if necessary, even if it's disabled mid-note.
        runner.send_msg_enable(&mut o, &clock, arpeggiator_uid, false);

        // It's actually immaterial to this test whether this has any sound in
        // it. We're just giving the arpeggiator a bit of time to clear out any
        // leftover note.
        clock.reset();
        if let Ok(_) = runner.run(&mut o, &mut clock) {
        } else {
            assert!(false, "impossible!");
        }

        // But by now it should be silent.
        clock.reset();
        if let Ok(samples) = runner.run(&mut o, &mut clock) {
            assert!(
                samples.iter().all(|&s| s == MONO_SAMPLE_SILENCE),
                "Expected total silence again after disabling the arpeggiator."
            );
        } else {
            assert!(false, "impossible!");
        }

        // Re-enable the arpeggiator but disconnect the instrument's MIDI
        // connection.
        runner.send_msg_enable(&mut o, &clock, arpeggiator_uid, true);
        o.disconnect_midi_downstream(
            instrument_uid,
            TestInstrument::<TestMessage>::TEST_MIDI_CHANNEL,
        );
        clock.reset();
        if let Ok(samples) = runner.run(&mut o, &mut clock) {
            assert!(
                samples.iter().all(|&s| s == MONO_SAMPLE_SILENCE),
                "Expected total silence after disconnecting the instrument from the MIDI bus."
            );
        } else {
            assert!(false, "impossible!");
        }
    }

    #[test]
    fn test_groove_can_be_instantiated_in_new_generic_world() {
        let mut o = Box::new(Orchestrator::<GrooveMessage>::default());

        // A simple audio source.
        let entity_groove =
            BoxedEntity::Instrument(Box::new(TestSynth::<GrooveMessage>::default()));
        let synth_uid = o.add(None, entity_groove);

        // A simple effect.
        let effect_uid = o.add(
            None,
            BoxedEntity::Effect(Box::new(TestNegatingEffect::<GrooveMessage>::default())),
        );

        // Connect the audio's output to the effect's input.
        assert!(o.patch(synth_uid, effect_uid).is_ok());

        // And patch the effect into the main mixer.
        o.connect_to_main_mixer(effect_uid);

        // Run the main loop for a while.
        const SECONDS: usize = 1;
        let _ = o.add(
            None,
            BoxedEntity::Controller(Box::new(Timer::<GrooveMessage>::new_with(SECONDS as f32))),
        );

        // Gather the audio output.
        let mut runner = GrooveRunner::default();
        let mut clock = Clock::new();
        if let Ok(samples_1) = runner.run(&mut o, &mut clock) {
            // We should get exactly the right amount of audio.
            assert_eq!(samples_1.len(), SECONDS * clock.sample_rate());

            // It should not all be silence.
            assert!(!samples_1.iter().any(|&s| s != MONO_SAMPLE_SILENCE));

            // Run again but without the negating effect in the mix.
            o.unpatch(synth_uid, effect_uid);
            clock.reset();
            if let Ok(samples_2) = runner.run(&mut o, &mut clock) {
                // The sample pairs should cancel each other out.
                assert!(!samples_2.iter().any(|&s| s != MONO_SAMPLE_SILENCE));
                samples_1.iter().zip(samples_2.iter()).all(|(a, b)| {
                    *a + *b == MONO_SAMPLE_SILENCE && (*a == MONO_SAMPLE_SILENCE || *a != *b)
                });
            }
        }
    }
}
