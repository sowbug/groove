#[cfg(test)]
pub mod tests {
    use std::{
        cell::RefCell,
        rc::{Rc, Weak},
    };

    use crate::{
        clock::{Clock, WatchedClock},
        common::{
            tests::{MONO_SAMPLE_MAX, MONO_SAMPLE_MIN},
            MonoSample, MONO_SAMPLE_SILENCE,
        },
        effects::mixer::Mixer,
        envelopes::AdsrEnvelope,
        midi::{MidiChannel, MidiMessage, MidiMessageType},
        oscillators::Oscillator,
        preset::EnvelopePreset,
        traits::{
            IsController, IsEffect, SinksControl, SinksControlParam, SinksMidi, SourcesAudio,
            SourcesControl, WatchesClock,
        },
    };

    #[derive(Debug, Default)]
    pub struct TestAudioSourceAlwaysSameLevel {
        level: MonoSample,
    }
    impl TestAudioSourceAlwaysSameLevel {
        pub fn new(level: MonoSample) -> Self {
            Self {
                level,
                ..Default::default()
            }
        }
    }
    impl SourcesAudio for TestAudioSourceAlwaysSameLevel {
        fn source_audio(&mut self, _clock: &Clock) -> MonoSample {
            self.level
        }
    }

    #[derive(Debug, Default)]
    pub struct TestAudioSourceAlwaysLoud {}
    impl TestAudioSourceAlwaysLoud {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }
    impl SourcesAudio for TestAudioSourceAlwaysLoud {
        fn source_audio(&mut self, _clock: &Clock) -> MonoSample {
            MONO_SAMPLE_MAX
        }
    }

    #[derive(Debug, Default)]
    pub struct TestAudioSourceAlwaysTooLoud {}
    impl TestAudioSourceAlwaysTooLoud {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }
    impl SourcesAudio for TestAudioSourceAlwaysTooLoud {
        fn source_audio(&mut self, _clock: &Clock) -> MonoSample {
            MONO_SAMPLE_MAX + 0.1
        }
    }

    #[derive(Debug, Default)]
    pub struct TestAudioSourceAlwaysSilent {}
    impl TestAudioSourceAlwaysSilent {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }
    impl SourcesAudio for TestAudioSourceAlwaysSilent {
        fn source_audio(&mut self, _clock: &Clock) -> MonoSample {
            MONO_SAMPLE_SILENCE
        }
    }

    #[derive(Debug, Default)]
    pub struct TestAudioSourceAlwaysVeryQuiet {}
    impl TestAudioSourceAlwaysVeryQuiet {
        #[allow(dead_code)]
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }
    impl SourcesAudio for TestAudioSourceAlwaysVeryQuiet {
        fn source_audio(&mut self, _clock: &Clock) -> MonoSample {
            MONO_SAMPLE_MIN
        }
    }

    #[derive(Debug)]
    pub struct TestOrchestrator {
        main_mixer: Box<dyn IsEffect>,
    }

    impl Default for TestOrchestrator {
        fn default() -> Self {
            Self::new()
        }
    }

    impl TestOrchestrator {
        pub fn new() -> Self {
            Self {
                main_mixer: Box::new(Mixer::new()),
            }
        }

        // TODO: I like "new_with"
        #[allow(dead_code)]
        pub fn new_with(main_mixer: Box<dyn IsEffect>) -> Self {
            Self { main_mixer }
        }

        pub fn add_audio_source(&mut self, source: Rc<RefCell<dyn SourcesAudio>>) {
            self.main_mixer.add_audio_source(source);
        }

        pub fn start(&mut self, clock: &mut WatchedClock, samples_out: &mut Vec<f32>) {
            loop {
                if clock.visit_watchers() {
                    break;
                }
                samples_out.push(self.main_mixer.source_audio(clock.inner_clock()));
                clock.tick();
            }
        }
    }

    #[derive(Debug)]
    pub struct TestSynth {
        oscillator: Rc<RefCell<dyn SourcesAudio>>,
        envelope: Rc<RefCell<dyn SourcesAudio>>,
    }

    impl TestSynth {
        #[deprecated]
        /// You really don't want to call this, because you need a sample rate
        /// for it to do anything meaningful, and it's a bad practice to hardcode
        /// a 44.1KHz rate.
        fn new() -> Self {
            Self {
                oscillator: Rc::new(RefCell::new(Oscillator::new())),
                envelope: Rc::new(RefCell::new(AdsrEnvelope::new_with(
                    &EnvelopePreset::default(),
                ))),
            }
        }
        pub fn new_with(
            oscillator: Rc<RefCell<dyn SourcesAudio>>,
            envelope: Rc<RefCell<dyn SourcesAudio>>,
        ) -> Self {
            Self {
                oscillator,
                envelope,
            }
        }
    }

    impl Default for TestSynth {
        fn default() -> Self {
            #[allow(deprecated)]
            Self::new()
        }
    }

    impl SourcesAudio for TestSynth {
        fn source_audio(&mut self, clock: &Clock) -> MonoSample {
            self.oscillator.borrow_mut().source_audio(clock)
                * self.envelope.borrow_mut().source_audio(clock)
        }
    }

    #[derive(Debug, Default)]
    pub struct TestTimer {
        time_to_run_seconds: f32,
    }
    impl TestTimer {
        pub fn new(time_to_run_seconds: f32) -> Self {
            Self {
                time_to_run_seconds,
            }
        }
    }
    impl WatchesClock for TestTimer {
        fn tick(&mut self, clock: &Clock) -> bool {
            clock.seconds() >= self.time_to_run_seconds
        }
    }

    #[derive(Debug, Default)]
    pub struct TestTrigger {
        control_sinks: Vec<Weak<RefCell<dyn SinksControl>>>,
        time_to_trigger_seconds: f32,
        value: f32,
        has_triggered: bool,
    }
    impl TestTrigger {
        pub fn new(time_to_trigger_seconds: f32, value: f32) -> Self {
            Self {
                time_to_trigger_seconds,
                value,
                ..Default::default()
            }
        }
    }
    impl WatchesClock for TestTrigger {
        fn tick(&mut self, clock: &Clock) -> bool {
            if !self.has_triggered && clock.seconds() >= self.time_to_trigger_seconds {
                self.has_triggered = true;
                let value = self.value;
                self.issue_control(clock, &SinksControlParam::Primary { value });
            }
            clock.seconds() >= self.time_to_trigger_seconds
        }
    }
    impl SourcesControl for TestTrigger {
        fn control_sinks(&self) -> &[Weak<RefCell<dyn SinksControl>>] {
            &self.control_sinks
        }

        fn control_sinks_mut(&mut self) -> &mut Vec<Weak<RefCell<dyn SinksControl>>> {
            &mut self.control_sinks
        }
    }

    /// Lets a SourcesAudio act like an IsController
    #[derive(Debug)]
    pub struct TestControlSourceContinuous {
        control_sinks: Vec<Weak<RefCell<dyn SinksControl>>>,
        source: Box<dyn SourcesAudio>,
    }
    impl TestControlSourceContinuous {
        pub fn new_with(source: Box<dyn SourcesAudio>) -> Self {
            Self {
                control_sinks: Vec::new(),
                source: source,
            }
        }
    }
    impl SourcesControl for TestControlSourceContinuous {
        fn control_sinks(&self) -> &[Weak<RefCell<dyn SinksControl>>] {
            &self.control_sinks
        }

        fn control_sinks_mut(&mut self) -> &mut Vec<Weak<RefCell<dyn SinksControl>>> {
            &mut self.control_sinks
        }
    }
    impl WatchesClock for TestControlSourceContinuous {
        fn tick(&mut self, clock: &Clock) -> bool {
            let value = self.source.source_audio(clock);
            self.issue_control(clock, &SinksControlParam::Primary { value });
            true
        }
    }
    impl IsController for TestControlSourceContinuous {}

    /// Helper for testing SinksMidi
    #[derive(Debug, Default)]
    pub struct TestMidiSink {
        pub is_playing: bool,
        midi_channel: MidiChannel,
        pub midi_messages_received: usize,
        pub midi_messages_handled: usize,
        pub value: f32,
    }

    impl TestMidiSink {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
        pub fn set_value(&mut self, value: f32) {
            self.value = value;
        }
    }
    impl SinksMidi for TestMidiSink {
        fn midi_channel(&self) -> MidiChannel {
            self.midi_channel
        }

        fn set_midi_channel(&mut self, midi_channel: MidiChannel) {
            self.midi_channel = midi_channel;
        }
        fn handle_midi_for_channel(&mut self, _clock: &Clock, message: &MidiMessage) {
            self.midi_messages_received += 1;

            match message.status {
                MidiMessageType::NoteOn => {
                    self.is_playing = true;
                    self.midi_messages_handled += 1;
                }
                MidiMessageType::NoteOff => {
                    self.is_playing = false;
                    self.midi_messages_handled += 1;
                }
                MidiMessageType::ProgramChange => {
                    self.midi_messages_handled += 1;
                }
            }
        }
    }
    impl SinksControl for TestMidiSink {
        fn handle_control(&mut self, _clock: &Clock, param: &SinksControlParam) {
            match param {
                SinksControlParam::Primary { value } => self.set_value(*value),
                #[allow(unused_variables)]
                SinksControlParam::Secondary { value } => todo!(),
            }
        }
    }
    impl SourcesAudio for TestMidiSink {
        fn source_audio(&mut self, _clock: &Clock) -> MonoSample {
            self.value
        }
    }
}
