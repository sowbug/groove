use crate::{
    clock::Clock,
    common::F32ControlValue,
    common::MonoSample,
    instruments::{envelopes::AdsrEnvelope, oscillators::Oscillator},
    messages::{EntityMessage, MessageBounds},
    settings::patches::EnvelopeSettings,
    traits::{
        Controllable, HasUid, IsController, IsInstrument, Response, SourcesAudio, Terminates,
        Updateable,
    },
};
use core::fmt::Debug;
use groove_macros::{Control, Uid};
use std::{
    env::{current_dir, current_exe},
    marker::PhantomData,
    path::PathBuf,
    str::FromStr,
};
use strum_macros::{Display, EnumString, FromRepr};

/// Timer returns true to Terminates::is_finished() after a specified amount of time.
#[derive(Debug, Default, Uid)]
pub struct Timer<M: MessageBounds> {
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

    pub fn time_to_run_seconds(&self) -> f32 {
        self.time_to_run_seconds
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
    ) -> Response<Self::Message> {
        Response::none()
    }
}
impl Updateable for Timer<EntityMessage> {
    type Message = EntityMessage;

    fn update(&mut self, clock: &Clock, message: Self::Message) -> Response<Self::Message> {
        if let Self::Message::Tick = message {
            self.has_more_work = clock.seconds() < self.time_to_run_seconds;
        }
        Response::none()
    }
}

/// Trigger issues a ControlF32 message after a specified amount of time.
#[derive(Debug, Default, Uid)]
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
    ) -> Response<Self::Message> {
        Response::none()
    }
}
impl<M: MessageBounds> Terminates for Trigger<M> {
    fn is_finished(&self) -> bool {
        self.has_triggered
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

    fn update(&mut self, clock: &Clock, message: Self::Message) -> Response<Self::Message> {
        if let Self::Message::Tick = message {
            return if !self.has_triggered && clock.seconds() >= self.time_to_trigger_seconds {
                self.has_triggered = true;
                Response::single(Self::Message::ControlF32(self.value))
            } else {
                Response::none()
            };
        }
        Response::none()
    }
}

#[derive(Display, Debug, EnumString)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum TestAudioSourceSetLevelControlParams {
    Level,
}

#[derive(Control, Debug, Default, Uid)]
pub struct AudioSource<M: MessageBounds> {
    uid: usize,
    level: MonoSample,
    _phantom: PhantomData<M>,
}
impl<M: MessageBounds> IsInstrument for AudioSource<M> {}
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

pub struct Paths {}
impl Paths {
    const ASSETS: &str = "assets";
    const PROJECTS: &str = "projects";
    const TEST_DATA: &str = "test-data";

    pub fn asset_path() -> PathBuf {
        let mut path_buf = Paths::cwd();
        path_buf.push(Self::ASSETS);
        path_buf
    }

    pub fn project_path() -> PathBuf {
        let mut path_buf = Paths::cwd();
        path_buf.push(Self::PROJECTS);
        path_buf
    }

    pub fn test_data_path() -> PathBuf {
        let mut path_buf = Paths::cwd();
        path_buf.push(Self::TEST_DATA);
        path_buf
    }

    pub(crate) fn cwd() -> PathBuf {
        PathBuf::from(
            current_dir()
                .ok()
                .map(PathBuf::into_os_string)
                .and_then(|exe| exe.into_string().ok())
                .unwrap(),
        )
    }

    #[allow(dead_code)]
    pub(crate) fn exe_path() -> PathBuf {
        PathBuf::from(
            current_exe()
                .ok()
                .map(PathBuf::into_os_string)
                .and_then(|exe| exe.into_string().ok())
                .unwrap(),
        )
    }
}

#[derive(Control, Debug, Uid)]
pub struct TestSynth<M: MessageBounds> {
    uid: usize,

    #[controllable]
    oscillator_modulation: f32,

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

    pub fn oscillator_modulation(&self) -> f32 {
        self.oscillator.frequency_modulation()
    }

    pub fn set_oscillator_modulation(&mut self, oscillator_modulation: f32) {
        self.oscillator_modulation = oscillator_modulation;
        self.oscillator
            .set_frequency_modulation(oscillator_modulation);
    }

    pub fn set_control_oscillator_modulation(&mut self, oscillator_modulation: F32ControlValue) {
        self.set_oscillator_modulation(oscillator_modulation.0);
    }
}
impl<M: MessageBounds> Default for TestSynth<M> {
    fn default() -> Self {
        Self {
            uid: 0,
            oscillator_modulation: Default::default(),
            oscillator: Box::new(Oscillator::default()),
            envelope: Box::new(AdsrEnvelope::new_with(&EnvelopeSettings::default())),
            _phantom: Default::default(),
        }
    }
}

impl<M: MessageBounds> SourcesAudio for TestSynth<M> {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        // TODO: I don't think this can play sounds, because I don't see how the
        // envelope ever gets triggered.
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
    ) -> Response<Self::Message> {
        Response::none()
    }
}
impl Updateable for TestSynth<EntityMessage> {
    type Message = EntityMessage;

    fn update(&mut self, _clock: &Clock, message: Self::Message) -> Response<Self::Message> {
        match message {
            _ => todo!(),
        }
    }
}

#[derive(Display, Debug, EnumString)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum TestLfoControlParams {
    Frequency,
}

#[derive(Debug, Default, Uid)]
pub struct TestLfo<M: MessageBounds> {
    uid: usize,
    value: MonoSample,
    oscillator: Oscillator,
    _phantom: PhantomData<M>,
}
impl<M: MessageBounds> IsController for TestLfo<M> {}
impl<M: MessageBounds> Updateable for TestLfo<M> {
    default type Message = M;

    default fn update(
        &mut self,
        _clock: &Clock,
        _message: Self::Message,
    ) -> Response<Self::Message> {
        Response::none()
    }
}
impl Updateable for TestLfo<EntityMessage> {
    type Message = EntityMessage;

    fn update(&mut self, clock: &Clock, message: Self::Message) -> Response<Self::Message> {
        if let Self::Message::Tick = message {
            self.value = self.oscillator.source_audio(clock);
            Response::single(Self::Message::ControlF32(self.value))
        } else {
            Response::none()
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
    pub fn frequency(&self) -> f32 {
        self.oscillator.frequency()
    }

    #[allow(dead_code)]
    pub fn set_frequency(&mut self, frequency_hz: f32) {
        self.oscillator.set_frequency(frequency_hz);
    }

    pub fn value(&self) -> f32 {
        self.value
    }
}

#[cfg(test)]
pub mod tests {
    use super::Timer;
    use crate::{
        clock::Clock,
        common::{MonoSample, MONO_SAMPLE_SILENCE},
        controllers::orchestrator::Orchestrator,
        entities::BoxedEntity,
        messages::{tests::TestMessage, EntityMessage, GrooveMessage, MessageBounds},
        midi::MidiChannel,
        traits::{
            Controllable, HasUid, IsController, IsEffect, Response, SourcesAudio, Terminates,
            TestController, TestEffect, TestInstrument, TransformsAudio, Updateable,
        },
        utils::{F32ControlValue, TestLfo, TestSynth, TestSynthControlParams},
    };
    use convert_case::{Case, Casing};
    use groove_macros::Control;
    use std::str::FromStr;
    use std::{fs, marker::PhantomData, path::PathBuf};
    use strum_macros::{Display, EnumString, FromRepr};

    fn read_samples_from_mono_wav_file(filename: &PathBuf) -> Vec<MonoSample> {
        let mut reader = hound::WavReader::open(filename).unwrap();
        let mut r = Vec::default();

        for sample in reader.samples::<i16>() {
            r.push(sample.unwrap() as MonoSample / i16::MAX as MonoSample);
        }
        r
    }

    pub fn samples_match_known_good_wav_file(
        samples: Vec<f32>,
        filename: &PathBuf,
        acceptable_deviation: f32,
    ) -> bool {
        let known_good_samples = read_samples_from_mono_wav_file(filename);
        if known_good_samples.len() != samples.len() {
            eprintln!("Provided samples of different length from known-good");
            return false;
        }
        for i in 0..samples.len() {
            if (samples[i] - known_good_samples[i]).abs() >= acceptable_deviation {
                eprintln!(
                    "Samples differed at position {i}: known-good {}, test {}",
                    known_good_samples[i], samples[i]
                );
                return false;
            }
        }
        true
    }

    pub fn canonicalize_filename(filename: &str) -> String {
        const OUT_DIR: &str = "out";
        let result = fs::create_dir_all(OUT_DIR);
        if result.is_err() {
            panic!();
        }
        let snake_filename = filename.to_case(Case::Snake);
        format!("{OUT_DIR}/{snake_filename}.wav")
    }

    impl Updateable for Timer<TestMessage> {
        type Message = TestMessage;

        fn update(&mut self, clock: &Clock, message: Self::Message) -> Response<Self::Message> {
            if let Self::Message::Tick = message {
                self.has_more_work = clock.seconds() < self.time_to_run_seconds;
            }
            Response::none()
        }
    }

    #[derive(Control, Debug, Default)]
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
        ) -> Response<Self::Message> {
            Response::none()
        }
    }
    impl Updateable for TestControlSourceContinuous<EntityMessage> {
        type Message = EntityMessage;

        fn update(&mut self, clock: &Clock, message: Self::Message) -> Response<Self::Message> {
            match message {
                Self::Message::Tick => {
                    let value = self.source.source_audio(clock).abs();
                    Response::single(Self::Message::ControlF32(value))
                }
                _ => Response::none(),
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
        #[allow(dead_code)]
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
        let synth_uid = o.add(None, BoxedEntity::TestSynth(Box::new(TestSynth::default())));

        // A simple effect.
        let effect_uid = o.add(
            None,
            BoxedEntity::TestEffect(Box::new(TestEffect::<EntityMessage>::default())),
        );

        // Connect the audio's output to the effect's input.
        assert!(o.patch(synth_uid, effect_uid).is_ok());

        // And patch the effect into the main mixer.
        let _ = o.connect_to_main_mixer(effect_uid);

        // Run the main loop for a while.
        const SECONDS: usize = 1;
        let _ = o.add(
            None,
            BoxedEntity::Timer(Box::new(Timer::new_with(SECONDS as f32))),
        );

        // Gather the audio output.
        let mut clock = Clock::default();
        if let Ok(samples_1) = o.run(&mut clock) {
            // We should get exactly the right amount of audio.
            assert_eq!(samples_1.len(), SECONDS * clock.sample_rate());

            // It should not all be silence.
            assert!(!samples_1.iter().any(|&s| s != MONO_SAMPLE_SILENCE));

            // Run again but without the negating effect in the mix.
            assert!(o.unpatch(synth_uid, effect_uid).is_ok());
            clock.reset();
            if let Ok(samples_2) = o.run(&mut clock) {
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
        let synth_1_uid = o.add(None, BoxedEntity::TestSynth(Box::new(TestSynth::default())));
        let mut lfo = TestLfo::default();
        lfo.set_frequency(2.0);
        let lfo_uid = o.add(None, BoxedEntity::TestLfo(Box::new(lfo)));
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
            BoxedEntity::Timer(Box::new(Timer::new_with(SECONDS as f32))),
        );

        // Gather the audio output.
        let mut clock = Clock::default();
        if let Ok(samples_1) = o.run(&mut clock) {
            // We should get exactly the right amount of audio.
            assert_eq!(samples_1.len(), SECONDS * clock.sample_rate());

            // It should not all be silence.
            assert!(!samples_1.iter().any(|&s| s != MONO_SAMPLE_SILENCE));

            // Run again after disconnecting the LFO.
            o.unlink_control(lfo_uid, synth_1_uid);
            clock.reset();
            if let Ok(samples_2) = o.run(&mut clock) {
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
            BoxedEntity::TestInstrument(Box::new(TestInstrument::default())),
        );
        let arpeggiator_uid = o.add(
            None,
            BoxedEntity::TestController(Box::new(TestController::new_with(TEST_MIDI_CHANNEL))),
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
            BoxedEntity::Timer(Box::new(Timer::new_with(SECONDS as f32))),
        );

        // Everything is hooked up. Let's run it and hear what we got.
        let mut clock = Clock::default();
        if let Ok(samples) = o.run(&mut clock) {
            // We haven't asked the arpeggiator to start sending anything yet.
            assert!(
                samples.iter().all(|&s| s == MONO_SAMPLE_SILENCE),
                "Expected total silence because the arpeggiator is not turned on."
            );
        } else {
            panic!("impossible!");
        }

        // Let's turn on the arpeggiator.
        o.debug_send_msg_enable(&clock, arpeggiator_uid, true);
        clock.reset();
        if let Ok(samples) = o.run(&mut clock) {
            assert!(
                samples.iter().any(|&s| s != MONO_SAMPLE_SILENCE),
                "Expected some sound because the arpeggiator is now running."
            );
        } else {
            panic!("impossible!");
        }

        // The arpeggiator is still running. Let's disable it (taking advantage
        // of the fact that TestInstrument has zero release time, because
        // otherwise it would keep generating a bit of sound even after the
        // arpeggiator told it to stop).
        //
        // Note that we're implicitly testing that the arpeggiator will send a
        // note-off if necessary, even if it's disabled mid-note.
        o.debug_send_msg_enable(&clock, arpeggiator_uid, false);

        // It's actually immaterial to this test whether this has any sound in
        // it. We're just giving the arpeggiator a bit of time to clear out any
        // leftover note.
        clock.reset();
        if o.run(&mut clock).is_err() {
            panic!("impossible!");
        }

        // But by now it should be silent.
        clock.reset();
        if let Ok(samples) = o.run(&mut clock) {
            assert!(
                samples.iter().all(|&s| s == MONO_SAMPLE_SILENCE),
                "Expected total silence again after disabling the arpeggiator."
            );
        } else {
            panic!("impossible!");
        }

        // Re-enable the arpeggiator but disconnect the instrument's MIDI
        // connection.
        o.debug_send_msg_enable(&clock, arpeggiator_uid, true);
        o.disconnect_midi_downstream(instrument_uid, TEST_MIDI_CHANNEL);
        clock.reset();
        if let Ok(samples) = o.run(&mut clock) {
            assert!(
                samples.iter().all(|&s| s == MONO_SAMPLE_SILENCE),
                "Expected total silence after disconnecting the instrument from the MIDI bus."
            );
        } else {
            panic!("impossible!");
        }
    }

    #[test]
    fn test_groove_can_be_instantiated_in_new_generic_world() {
        let mut o = Box::new(Orchestrator::<GrooveMessage>::default());

        // A simple audio source.
        let entity_groove = BoxedEntity::TestSynth(Box::new(TestSynth::default()));
        let synth_uid = o.add(None, entity_groove);

        // A simple effect.
        let effect_uid = o.add(
            None,
            BoxedEntity::TestEffect(Box::new(TestEffect::default())),
        );

        // Connect the audio's output to the effect's input.
        assert!(o.patch(synth_uid, effect_uid).is_ok());

        // And patch the effect into the main mixer.
        let _ = o.connect_to_main_mixer(effect_uid);

        // Run the main loop for a while.
        const SECONDS: usize = 1;
        let _ = o.add(
            None,
            BoxedEntity::Timer(Box::new(Timer::new_with(SECONDS as f32))),
        );

        // Gather the audio output.
        let mut clock = Clock::default();
        if let Ok(samples_1) = o.run(&mut clock) {
            // We should get exactly the right amount of audio.
            assert_eq!(samples_1.len(), SECONDS * clock.sample_rate());

            // It should not all be silence.
            assert!(!samples_1.iter().any(|&s| s != MONO_SAMPLE_SILENCE));

            // Run again but without the negating effect in the mix.
            assert!(o.unpatch(synth_uid, effect_uid).is_ok());
            clock.reset();
            if let Ok(samples_2) = o.run(&mut clock) {
                // The sample pairs should cancel each other out.
                assert!(!samples_2.iter().any(|&s| s != MONO_SAMPLE_SILENCE));
                samples_1.iter().zip(samples_2.iter()).all(|(a, b)| {
                    *a + *b == MONO_SAMPLE_SILENCE && (*a == MONO_SAMPLE_SILENCE || *a != *b)
                });
            }
        }
    }
}
