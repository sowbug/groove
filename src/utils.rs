use crate::{
    clock::Clock,
    common::MonoSample,
    messages::{EntityMessage, MessageBounds},
    traits::{
        EvenNewerCommand, HasUid, IsController, IsInstrument, SourcesAudio, Terminates, Updateable,
    },
};
use core::fmt::Debug;
use std::marker::PhantomData;
use strum_macros::{Display, EnumString};

/// Timer returns true to Terminates::is_finished() after a specified amount of time.
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
impl<M: MessageBounds> IsController for Timer<M> {}
impl<M: MessageBounds> Updateable for Timer<M> {
    default type Message = M;

    default fn update(
        &mut self,
        _clock: &Clock,
        _message: Self::Message,
    ) -> EvenNewerCommand<Self::Message> {
        EvenNewerCommand::none()
    }
}
impl Updateable for Timer<EntityMessage> {
    type Message = EntityMessage;

    fn update(&mut self, clock: &Clock, message: Self::Message) -> EvenNewerCommand<Self::Message> {
        match message {
            Self::Message::Tick => {
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

/// Trigger issues a ControlF32 message after a specified amount of time.
#[derive(Debug, Default)]
pub(crate) struct Trigger<M: MessageBounds> {
    uid: usize,
    time_to_trigger_seconds: f32,
    value: f32,
    has_triggered: bool,

    _phantom: PhantomData<M>,
}
impl<M: MessageBounds> IsController for Trigger<M> {}
impl<M: MessageBounds> Updateable for Trigger<M> {
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
impl Updateable for Trigger<EntityMessage> {
    type Message = EntityMessage;

    fn update(&mut self, clock: &Clock, message: Self::Message) -> EvenNewerCommand<Self::Message> {
        match message {
            Self::Message::Tick => {
                return if !self.has_triggered && clock.seconds() >= self.time_to_trigger_seconds {
                    self.has_triggered = true;
                    EvenNewerCommand::single(Self::Message::ControlF32(self.value))
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
impl<M: MessageBounds> IsInstrument for AudioSource<M> {}
impl<M: MessageBounds> HasUid for AudioSource<M> {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}
impl<M: MessageBounds> Updateable for AudioSource<M> {
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
        clock::Clock,
        common::{MonoSample, MONO_SAMPLE_SILENCE},
        controllers::orchestrator::{tests::Runner, GrooveRunner, Orchestrator},
        instruments::{envelopes::AdsrEnvelope, oscillators::Oscillator},
        messages::{tests::TestMessage, EntityMessage},
        messages::{GrooveMessage, MessageBounds},
        midi::MidiChannel,
        settings::{patches::EnvelopeSettings, ClockSettings},
        traits::{
            BoxedEntity, EvenNewerCommand, HasUid, IsController, IsEffect, IsInstrument,
            SourcesAudio, Terminates, TestController, TestEffect, TestInstrument, TransformsAudio,
            Updateable,
        },
    };
    use convert_case::{Case, Casing};
    use strum_macros::FromRepr;
    // use plotters::prelude::*;
    use super::Timer;
    use spectrum_analyzer::{
        samples_fft_to_spectrum, scaling::divide_by_N, windows::hann_window, FrequencyLimit,
    };
    use std::str::FromStr;
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

    impl Updateable for Timer<TestMessage> {
        type Message = TestMessage;

        fn update(
            &mut self,
            clock: &Clock,
            message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            match message {
                Self::Message::Tick => {
                    self.has_more_work = clock.seconds() < self.time_to_run_seconds;
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
    impl<M: MessageBounds> IsEffect for TestMixer<M> {}
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
    impl<M: MessageBounds> Updateable for TestMixer<M> {
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
    impl<M: MessageBounds> IsController for TestLfo<M> {}
    impl<M: MessageBounds> HasUid for TestLfo<M> {
        fn uid(&self) -> usize {
            self.uid
        }

        fn set_uid(&mut self, uid: usize) {
            self.uid = uid;
        }
    }
    impl<M: MessageBounds> Updateable for TestLfo<M> {
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
    impl Updateable for TestLfo<EntityMessage> {
        type Message = EntityMessage;

        fn update(
            &mut self,
            clock: &Clock,
            message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            if let Self::Message::Tick = message {
                let value = self.oscillator.source_audio(&clock);
                EvenNewerCommand::single(Self::Message::ControlF32(value))
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
                Box::new(Oscillator::default()),
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
                oscillator: Box::new(Oscillator::default()),
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

    impl<M: MessageBounds> IsInstrument for TestSynth<M> {}
    impl<M: MessageBounds> Updateable for TestSynth<M> {
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
    impl Updateable for TestSynth<EntityMessage> {
        type Message = EntityMessage;

        fn update(
            &mut self,
            _clock: &Clock,
            message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            match message {
                Self::Message::UpdateF32(param_index, value) => {
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
    impl<M: MessageBounds> IsController for TestControlSourceContinuous<M> {}
    impl<M: MessageBounds> Updateable for TestControlSourceContinuous<M> {
        default type Message = M;

        default fn update(
            &mut self,
            _clock: &Clock,
            _message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            EvenNewerCommand::none()
        }
    }
    impl Updateable for TestControlSourceContinuous<EntityMessage> {
        type Message = EntityMessage;

        fn update(
            &mut self,
            clock: &Clock,
            message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            match message {
                Self::Message::Tick => {
                    let value = self.source.source_audio(&clock).abs();
                    EvenNewerCommand::single(Self::Message::ControlF32(value))
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

    #[test]
    fn test_audio_routing() {
        let mut o = Box::new(Orchestrator::<TestMessage>::default());

        // A simple audio source.
        let synth_uid = o.add(
            None,
            BoxedEntity::Instrument(Box::new(TestSynth::default())),
        );

        // A simple effect.
        let effect_uid = o.add(
            None,
            BoxedEntity::Effect(Box::new(TestEffect::<EntityMessage>::default())),
        );

        // Connect the audio's output to the effect's input.
        assert!(o.patch(synth_uid, effect_uid).is_ok());

        // And patch the effect into the main mixer.
        let _ = o.connect_to_main_mixer(effect_uid);

        // Run the main loop for a while.
        const SECONDS: usize = 1;
        let _ = o.add(
            None,
            BoxedEntity::Controller(Box::new(Timer::new_with(SECONDS as f32))),
        );

        // Gather the audio output.
        let mut runner = Runner::default();
        let mut clock = Clock::default();
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
            BoxedEntity::Instrument(Box::new(TestSynth::default())),
        );
        let mut lfo = TestLfo::default();
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
            BoxedEntity::Controller(Box::new(Timer::new_with(SECONDS as f32))),
        );

        // Gather the audio output.
        let mut runner = Runner::default();
        let mut clock = Clock::default();
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
        const TEST_MIDI_CHANNEL: MidiChannel = 7;
        let mut o = Box::new(Orchestrator::<TestMessage>::default());

        // We have a regular MIDI instrument, and an arpeggiator that emits MIDI note messages.
        let instrument_uid = o.add(
            None,
            BoxedEntity::Instrument(Box::new(TestInstrument::default())),
        );
        let arpeggiator_uid = o.add(
            None,
            BoxedEntity::Controller(Box::new(TestController::new_with(TEST_MIDI_CHANNEL))),
        );

        // We'll hear the instrument.
        assert!(o.connect_to_main_mixer(instrument_uid).is_ok());

        // This might not be necessary. Orchestrator will automatically get
        // every MIDI message sent.
        o.connect_midi_upstream(arpeggiator_uid);
        o.connect_midi_downstream(instrument_uid, TEST_MIDI_CHANNEL);

        const SECONDS: usize = 1;
        let _ = o.add(
            None,
            BoxedEntity::Controller(Box::new(Timer::new_with(SECONDS as f32))),
        );

        // Everything is hooked up. Let's run it and hear what we got.
        let mut runner = Runner::default();
        let mut clock = Clock::default();
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
        o.disconnect_midi_downstream(instrument_uid, TEST_MIDI_CHANNEL);
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
        let entity_groove = BoxedEntity::Instrument(Box::new(TestSynth::default()));
        let synth_uid = o.add(None, entity_groove);

        // A simple effect.
        let effect_uid = o.add(None, BoxedEntity::Effect(Box::new(TestEffect::default())));

        // Connect the audio's output to the effect's input.
        assert!(o.patch(synth_uid, effect_uid).is_ok());

        // And patch the effect into the main mixer.
        let _ = o.connect_to_main_mixer(effect_uid);

        // Run the main loop for a while.
        const SECONDS: usize = 1;
        let _ = o.add(
            None,
            BoxedEntity::Controller(Box::new(Timer::new_with(SECONDS as f32))),
        );

        // Gather the audio output.
        let mut runner = GrooveRunner::default();
        let mut clock = Clock::default();
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
