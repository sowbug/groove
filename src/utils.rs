use crate::{
    clock::Clock,
    common::MonoSample,
    messages::{GrooveMessage, MessageBounds},
    traits::{
        EvenNewerCommand, HasUid, NewIsController, NewIsInstrument, NewUpdateable, SourcesAudio,
        Terminates,
    },
};
use core::fmt::Debug;
use std::marker::PhantomData;
use strum_macros::{Display, EnumString};

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

#[derive(Debug, Default)]
pub(crate) struct Trigger<M: MessageBounds> {
    uid: usize,
    time_to_trigger_seconds: f32,
    value: f32,
    has_triggered: bool,

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
    #[allow(dead_code)]
    pub fn new(time_to_trigger_seconds: f32, value: f32) -> Self {
        Self {
            time_to_trigger_seconds,
            value,
            ..Default::default()
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

#[derive(Display, Debug, EnumString)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum TestAudioSourceSetLevelControlParams {
    Level,
}

#[derive(Debug, Default)]
pub struct AudioSource<M: MessageBounds> {
    uid: usize,
    level: MonoSample,
    _phantom: PhantomData<M>,
}
impl<M: MessageBounds> NewIsInstrument for AudioSource<M> {}
impl<M: MessageBounds> HasUid for AudioSource<M> {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}
impl<M: MessageBounds> NewUpdateable for AudioSource<M> {
    type Message = M;
}
#[allow(dead_code)]
impl<M: MessageBounds> AudioSource<M> {
    pub const TOO_LOUD: MonoSample = 1.1;
    pub const LOUD: MonoSample = 1.0;
    pub const SILENT: MonoSample = 0.0;
    pub const QUIET: MonoSample = -1.0;
    pub const TOO_QUIET: MonoSample = -1.1;

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
impl<M: MessageBounds> SourcesAudio for AudioSource<M> {
    fn source_audio(&mut self, _clock: &Clock) -> MonoSample {
        self.level
    }
}

#[cfg(test)]
pub mod tests {
    use crate::{
        clock::{Clock, ClockTimeUnit},
        common::{MonoSample, MONO_SAMPLE_SILENCE},
        controllers::orchestrator::{tests::Runner, GrooveRunner, Orchestrator},
        instruments::{envelopes::AdsrEnvelope, oscillators::Oscillator},
        messages::MessageBounds,
        messages::{tests::TestMessage, GrooveMessage},
        midi::{MidiChannel, MidiMessage},
        settings::{patches::EnvelopeSettings, ClockSettings},
        traits::{
            tests::{TestEffect, TestInstrument},
            BoxedEntity, EvenNewerCommand, HasUid, NewIsController, NewIsEffect, NewIsInstrument,
            NewUpdateable, SourcesAudio, Terminates, TransformsAudio,
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
    use std::{collections::VecDeque, str::FromStr};
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

    #[allow(dead_code)]
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

    // pub(crate) fn write_orchestration_to_file<M: MessageBounds>(
    //     basename: &str,
    //     waveform_type: WaveformType,
    //     orchestrator: &mut Orchestrator<M>,
    // ) {
    //     let osc = Oscillator::new_with(waveform_type);
    //     if let Some(effect) = effect_opt {
    //         effect
    //             .borrow_mut()
    //             .add_audio_source(rrc_downgrade::<Oscillator>(&osc));
    //         o.add_audio_source(rrc_downgrade::<dyn IsEffect>(&effect));
    //     }
    //     c.add_watcher(rrc(Timer::<TestMessage>::new_with(2.0)));
    //     if let Some(control) = control_opt {
    //         c.add_watcher(rrc_clone::<dyn WatchesClock>(&control));
    //     }
    //     let samples_out = o.run_until_completion(&mut c);
    //     write_samples_to_wav_file(basename, sample_rate, &samples_out);
    // }

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

    // ```rust
    // #[allow(dead_code)]
    // pub(crate) fn write_orchestration_to_file(
    //     orchestrator: &mut OldTestOrchestrator,
    //     clock: &mut WatchedClock,
    //     basename: &str,
    // ) {
    //     let samples = orchestrator.run_until_completion(clock);
    //     let spec = hound::WavSpec {
    //         channels: 1,
    //         sample_rate: clock.inner_clock().sample_rate() as u32,
    //         bits_per_sample: 16,
    //         sample_format: hound::SampleFormat::Int,
    //     };
    //     const AMPLITUDE: MonoSample = i16::MAX as MonoSample;
    //     let mut writer = hound::WavWriter::create(canonicalize_filename(basename), spec).unwrap();
    //     for sample in samples.iter() {
    //         let _ = writer.write_sample((sample * AMPLITUDE) as i16);
    //     }
    //     generate_fft_for_samples(
    //         clock.inner_clock().settings(),
    //         &samples,
    //         &canonicalize_fft_filename(basename),
    //     );
    // }
    // ```

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

    #[derive(Display, Debug, EnumString, FromRepr)]
    #[strum(serialize_all = "kebab_case")]
    pub enum TestSynthControlParams {
        OscillatorModulation,
    }

    #[derive(Debug)]
    pub struct TestSynth<M: MessageBounds> {
        uid: usize,

        oscillator: Box<Oscillator>,
        envelope: Box<dyn SourcesAudio>,
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
                Box::new(AdsrEnvelope::new_with(&EnvelopeSettings::default())),
            )
        }
        pub fn new_with(oscillator: Box<Oscillator>, envelope: Box<dyn SourcesAudio>) -> Self {
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
                envelope: Box::new(AdsrEnvelope::new_with(&EnvelopeSettings::default())),
                _phantom: Default::default(),
            }
        }
    }

    impl<M: MessageBounds> SourcesAudio for TestSynth<M> {
        fn source_audio(&mut self, clock: &Clock) -> MonoSample {
            self.oscillator.source_audio(clock) * self.envelope.source_audio(clock)
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
    pub(crate) enum TestArpeggiatorControlParams {
        Tempo,
    }

    enum TestArpeggiatorAction {
        Nothing,
        NoteOn,
        NoteOff,
    }

    #[derive(Debug, Default)]
    pub struct TestController<M: MessageBounds> {
        uid: usize,
        midi_channel_out: MidiChannel,
        pub tempo: f32,
        is_enabled: bool,
        is_playing: bool,

        _phantom: PhantomData<M>,
    }
    impl<M: MessageBounds> Terminates for TestController<M> {
        fn is_finished(&self) -> bool {
            true
        }
    }
    impl<M: MessageBounds> NewIsController for TestController<M> {}
    impl<M: MessageBounds> NewUpdateable for TestController<M> {
        default type Message = M;

        default fn update(
            &mut self,
            _clock: &Clock,
            _message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            EvenNewerCommand::none()
        }
    }
    impl NewUpdateable for TestController<TestMessage> {
        type Message = TestMessage;

        fn update(
            &mut self,
            clock: &Clock,
            message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            match message {
                TestMessage::Tick => {
                    return match self.what_to_do(clock) {
                        TestArpeggiatorAction::Nothing => EvenNewerCommand::none(),
                        TestArpeggiatorAction::NoteOn => {
                            // This is elegant, I hope. If the arpeggiator is
                            // disabled during play, and we were playing a note,
                            // then we still send the off note,
                            if self.is_enabled {
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
                            }
                        }
                        TestArpeggiatorAction::NoteOff => {
                            if self.is_playing {
                                EvenNewerCommand::single(TestMessage::Midi(
                                    self.midi_channel_out,
                                    MidiMessage::NoteOff {
                                        key: 60.into(),
                                        vel: 0.into(),
                                    },
                                ))
                            } else {
                                EvenNewerCommand::none()
                            }
                        }
                    };
                }
                TestMessage::Enable(enabled) => {
                    self.is_enabled = enabled;
                    EvenNewerCommand::none()
                }
                _ => todo!(),
            }
        }
    }
    impl<M: MessageBounds> HasUid for TestController<M> {
        fn uid(&self) -> usize {
            self.uid
        }

        fn set_uid(&mut self, uid: usize) {
            self.uid = uid
        }
    }
    impl<M: MessageBounds> TestController<M> {
        pub fn new_with(midi_channel_out: MidiChannel) -> Self {
            Self {
                midi_channel_out,
                ..Default::default()
            }
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
        #[allow(dead_code)]
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
            BoxedEntity::Effect(Box::new(TestEffect::<TestMessage>::default())),
        );

        // Connect the audio's output to the effect's input.
        assert!(o.patch(synth_uid, effect_uid).is_ok());

        // And patch the effect into the main mixer.
        let _ = o.connect_to_main_mixer(effect_uid);

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
        let _ = o.connect_to_main_mixer(synth_1_uid);

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
            BoxedEntity::Controller(Box::new(TestController::<TestMessage>::new_with(
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
            BoxedEntity::Effect(Box::new(TestEffect::<GrooveMessage>::default())),
        );

        // Connect the audio's output to the effect's input.
        assert!(o.patch(synth_uid, effect_uid).is_ok());

        // And patch the effect into the main mixer.
        let _ = o.connect_to_main_mixer(effect_uid);

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
