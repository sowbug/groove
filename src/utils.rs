#[cfg(test)]
pub mod tests {
    use crate::clock::ClockTimeUnit;
    use crate::common::{W, WW};
    use crate::midi::MidiNote;
    use crate::traits::MakesControlSink;
    use crate::{
        clock::{Clock, WatchedClock},
        common::{
            tests::{MONO_SAMPLE_MAX, MONO_SAMPLE_MIN},
            MonoSample, MONO_SAMPLE_SILENCE,
        },
        effects::mixer::Mixer,
        envelopes::AdsrEnvelope,
        midi::{MidiChannel, MidiMessage, MidiMessageType, MIDI_CHANNEL_RECEIVE_ALL},
        oscillators::Oscillator,
        preset::EnvelopePreset,
        traits::{
            IsController, IsEffect, SinksAudio, SinksControl, SinksMidi, SourcesAudio,
            SourcesControl, SourcesMidi, Terminates, WatchesClock,
        },
    };
    use assert_approx_eq::assert_approx_eq;
    use std::collections::{HashMap, VecDeque};
    use std::{
        cell::RefCell,
        rc::{Rc, Weak},
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

    #[derive(Debug, Default)]
    pub struct TestNullController {
        control_sinks: Vec<Box<dyn SinksControl>>,
    }

    impl TestNullController {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }

    impl SourcesControl for TestNullController {
        fn control_sinks(&self) -> &[Box<dyn SinksControl>] {
            &self.control_sinks
        }
        fn control_sinks_mut(&mut self) -> &mut Vec<Box<dyn SinksControl>> {
            &mut self.control_sinks
        }
    }

    impl WatchesClock for TestNullController {
        fn tick(&mut self, _clock: &Clock) {}
    }

    impl Terminates for TestNullController {
        fn is_finished(&self) -> bool {
            true
        }
    }

    impl IsController for TestNullController {}

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
        has_more_work: bool,
        time_to_run_seconds: f32,
    }
    impl TestTimer {
        pub fn new(time_to_run_seconds: f32) -> Self {
            Self {
                time_to_run_seconds,
                ..Default::default()
            }
        }
    }
    impl WatchesClock for TestTimer {
        fn tick(&mut self, clock: &Clock) {
            self.has_more_work = clock.seconds() < self.time_to_run_seconds;
        }
    }
    impl Terminates for TestTimer {
        fn is_finished(&self) -> bool {
            !self.has_more_work
        }
    }

    #[derive(Debug, Default)]
    pub struct TestTrigger {
        control_sinks: Vec<Box<dyn SinksControl>>,
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
        fn tick(&mut self, clock: &Clock) {
            if !self.has_triggered && clock.seconds() >= self.time_to_trigger_seconds {
                self.has_triggered = true;
                let value = self.value;
                self.issue_control(clock, value);
            }
        }
    }
    impl Terminates for TestTrigger {
        fn is_finished(&self) -> bool {
            self.has_triggered
        }
    }
    impl SourcesControl for TestTrigger {
        fn control_sinks(&self) -> &[Box<dyn SinksControl>] {
            &self.control_sinks
        }

        fn control_sinks_mut(&mut self) -> &mut Vec<Box<dyn SinksControl>> {
            &mut self.control_sinks
        }
    }

    /// Lets a SourcesAudio act like an IsController
    #[derive(Debug)]
    pub struct TestControlSourceContinuous {
        control_sinks: Vec<Box<dyn SinksControl>>,
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
        fn control_sinks(&self) -> &[Box<dyn SinksControl>] {
            &self.control_sinks
        }

        fn control_sinks_mut(&mut self) -> &mut Vec<Box<dyn SinksControl>> {
            &mut self.control_sinks
        }
    }
    impl WatchesClock for TestControlSourceContinuous {
        fn tick(&mut self, clock: &Clock) {
            let value = self.source.source_audio(clock);
            self.issue_control(clock, value);
        }
    }
    impl Terminates for TestControlSourceContinuous {
        fn is_finished(&self) -> bool {
            true
        }
    }
    impl IsController for TestControlSourceContinuous {}

    /// Helper for testing SinksMidi
    #[derive(Debug, Default)]
    pub struct TestMidiSink {
        pub(crate) me: WW<Self>,
        pub is_playing: bool,
        midi_channel: MidiChannel,
        pub midi_messages_received: usize,
        pub midi_messages_handled: usize,
        pub value: f32,
    }

    impl TestMidiSink {
        pub const TEST_MIDI_CHANNEL: u8 = 42;
        pub const CONTROL_PARAM_DEFAULT: &str = "param";

        pub fn new() -> Self {
            Self {
                me: Weak::new(),
                midi_channel: Self::TEST_MIDI_CHANNEL,
                ..Default::default()
            }
        }
        pub fn new_wrapped() -> W<Self> {
            // TODO: Rc::new_cyclic() should make this easier, but I couldn't get the syntax right.
            // https://doc.rust-lang.org/std/rc/struct.Rc.html#method.new_cyclic

            let wrapped = Rc::new(RefCell::new(Self::new()));
            wrapped.borrow_mut().me = Rc::downgrade(&wrapped);
            wrapped
        }
        pub fn new_with(midi_channel: MidiChannel) -> Self {
            Self {
                midi_channel,
                ..Default::default()
            }
        }
        #[allow(dead_code)]
        pub fn new_wrapped_with(midi_channel: MidiChannel) -> W<Self> {
            // TODO: Rc::new_cyclic() should make this easier, but I couldn't get the syntax right.
            // https://doc.rust-lang.org/std/rc/struct.Rc.html#method.new_cyclic

            let wrapped = Rc::new(RefCell::new(Self::new_with(midi_channel)));
            wrapped.borrow_mut().me = Rc::downgrade(&wrapped);
            wrapped
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
    impl SourcesAudio for TestMidiSink {
        fn source_audio(&mut self, _clock: &Clock) -> MonoSample {
            self.value
        }
    }
    impl MakesControlSink for TestMidiSink {
        fn make_control_sink(&self, param_name: &str) -> Option<Box<dyn SinksControl>> {
            if self.me.strong_count() != 0 {
                match param_name {
                    Self::CONTROL_PARAM_DEFAULT => Some(Box::new(TestMidiSinkController {
                        target: self.me.clone(),
                    })),
                    _ => None,
                }
            } else {
                None
            }
        }
    }

    #[derive(Debug)]
    pub struct TestMidiSinkController {
        target: WW<TestMidiSink>,
    }
    impl SinksControl for TestMidiSinkController {
        fn handle_control(&mut self, _clock: &Clock, param: f32) {
            if let Some(target) = self.target.upgrade() {
                target.borrow_mut().set_value(param);
            }
        }
    }

    /// Keeps asking for time slices until end of specified lifetime.
    #[derive(Debug, Default)]
    pub struct TestClockWatcher {
        has_more_work: bool,
        lifetime_seconds: f32,
    }

    impl WatchesClock for TestClockWatcher {
        fn tick(&mut self, clock: &Clock) {
            self.has_more_work = clock.seconds() < self.lifetime_seconds;
        }
    }

    impl Terminates for TestClockWatcher {
        fn is_finished(&self) -> bool {
            !self.has_more_work
        }
    }

    impl TestClockWatcher {
        pub fn new(lifetime_seconds: f32) -> Self {
            Self {
                lifetime_seconds,
                ..Default::default()
            }
        }
    }

    #[derive(Debug, Default)]
    pub struct TestAudioSource {}

    impl SourcesAudio for TestAudioSource {
        fn source_audio(&mut self, _clock: &Clock) -> crate::common::MonoSample {
            0.
        }
    }

    impl TestAudioSource {
        pub fn new() -> Self {
            TestAudioSource {}
        }
    }

    #[derive(Debug, Default)]
    pub struct TestAudioSink {
        sources: Vec<Rc<RefCell<dyn SourcesAudio>>>,
    }
    impl SinksAudio for TestAudioSink {
        fn sources(&self) -> &[Rc<RefCell<dyn SourcesAudio>>] {
            &self.sources
        }
        fn sources_mut(&mut self) -> &mut Vec<Rc<RefCell<dyn SourcesAudio>>> {
            &mut self.sources
        }
    }
    impl TestAudioSink {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }

    #[derive(Debug, Default)]
    pub struct TestControlSource {
        control_sinks: Vec<Box<dyn SinksControl>>,
    }

    impl SourcesControl for TestControlSource {
        fn control_sinks(&self) -> &[Box<dyn SinksControl>] {
            &self.control_sinks
        }
        fn control_sinks_mut(&mut self) -> &mut Vec<Box<dyn SinksControl>> {
            &mut self.control_sinks
        }
    }

    impl TestControlSource {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }

        pub fn handle_test_event(&mut self, value: f32) {
            self.issue_control(&Clock::new(), value);
        }
    }

    #[derive(Debug, Default)]
    pub struct TestControllable {
        pub(crate) me: Weak<RefCell<Self>>,
        pub value: f32,
    }
    impl TestControllable {
        pub(crate) const CONTROL_PARAM_DEFAULT: &str = "value";

        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
        pub fn new_wrapped() -> W<Self> {
            // TODO: Rc::new_cyclic() should make this easier, but I couldn't get the syntax right.
            // https://doc.rust-lang.org/std/rc/struct.Rc.html#method.new_cyclic

            let wrapped = Rc::new(RefCell::new(Self::new()));
            wrapped.borrow_mut().me = Rc::downgrade(&wrapped);
            wrapped
        }
    }
    impl MakesControlSink for TestControllable {
        fn make_control_sink(&self, param_name: &str) -> Option<Box<dyn SinksControl>> {
            if self.me.strong_count() != 0 {
                match param_name {
                    Self::CONTROL_PARAM_DEFAULT => Some(Box::new(TestControllableController {
                        target: self.me.clone(),
                    })),
                    _ => None,
                }
            } else {
                None
            }
        }
    }

    #[derive(Debug)]
    pub(crate) struct TestControllableController {
        target: WW<TestControllable>,
    }
    impl SinksControl for TestControllableController {
        fn handle_control(&mut self, _clock: &Clock, param: f32) {
            if let Some(target) = self.target.upgrade() {
                target.borrow_mut().value = param;
            }
        }
    }

    #[derive(Debug, Default)]
    pub struct TestMidiSource {
        channels_to_sink_vecs: HashMap<MidiChannel, Vec<Weak<RefCell<dyn SinksMidi>>>>,
    }

    impl SourcesMidi for TestMidiSource {
        fn midi_sinks(&self) -> &HashMap<MidiChannel, Vec<Weak<RefCell<dyn SinksMidi>>>> {
            &self.channels_to_sink_vecs
        }
        fn midi_sinks_mut(
            &mut self,
        ) -> &mut HashMap<MidiChannel, Vec<Weak<RefCell<dyn SinksMidi>>>> {
            &mut self.channels_to_sink_vecs
        }

        fn midi_output_channel(&self) -> MidiChannel {
            MIDI_CHANNEL_RECEIVE_ALL
        }

        fn set_midi_output_channel(&mut self, _midi_channel: MidiChannel) {}
    }

    impl TestMidiSource {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }

        pub fn source_some_midi(&mut self, clock: &Clock) {
            let message =
                MidiMessage::new_note_on(TestMidiSink::TEST_MIDI_CHANNEL, MidiNote::C4 as u8, 100);
            self.issue_midi(clock, &message);
        }
    }

    // Gets called with native functions telling it about external keyboard events.
    // Translates those into automation events that influence an arpeggiator,
    // which controls a MIDI instrument.
    //
    // This shows how all these traits work together.
    #[derive(Debug, Default)]
    pub struct TestKeyboard {
        control_sinks: Vec<Box<dyn SinksControl>>,
    }

    impl SourcesControl for TestKeyboard {
        fn control_sinks(&self) -> &[Box<dyn SinksControl>] {
            &self.control_sinks
        }
        fn control_sinks_mut(&mut self) -> &mut Vec<Box<dyn SinksControl>> {
            &mut self.control_sinks
        }
    }

    impl TestKeyboard {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }

        pub fn handle_keypress(&mut self, key: u8) {
            match key {
                1 => {
                    self.issue_control(&Clock::new(), 0.5);
                }
                _ => {}
            }
        }
    }

    #[derive(Debug, Default)]
    pub struct TestArpeggiator {
        me: Weak<RefCell<Self>>,
        midi_channel_out: MidiChannel,
        pub tempo: f32,
        channels_to_sink_vecs: HashMap<MidiChannel, Vec<Weak<RefCell<dyn SinksMidi>>>>,
    }
    impl SourcesMidi for TestArpeggiator {
        fn midi_sinks(&self) -> &HashMap<MidiChannel, Vec<Weak<RefCell<dyn SinksMidi>>>> {
            &self.channels_to_sink_vecs
        }
        fn midi_sinks_mut(
            &mut self,
        ) -> &mut HashMap<MidiChannel, Vec<Weak<RefCell<dyn SinksMidi>>>> {
            &mut self.channels_to_sink_vecs
        }

        fn midi_output_channel(&self) -> MidiChannel {
            self.midi_channel_out
        }

        fn set_midi_output_channel(&mut self, midi_channel: MidiChannel) {
            self.midi_channel_out = midi_channel;
        }
    }
    impl WatchesClock for TestArpeggiator {
        fn tick(&mut self, clock: &Clock) {
            // We don't actually pay any attention to self.tempo, but it's easy
            // enough to see that tempo could have influenced this MIDI message.
            self.issue_midi(
                &clock,
                &MidiMessage::new_note_on(self.midi_channel_out, 60, 100),
            );
        }
    }
    impl Terminates for TestArpeggiator {
        fn is_finished(&self) -> bool {
            true
        }
    }
    impl MakesControlSink for TestArpeggiator {
        fn make_control_sink(&self, param_name: &str) -> Option<Box<dyn SinksControl>> {
            if self.me.strong_count() != 0 {
                match param_name {
                    Self::CONTROL_PARAM_TEMPO => Some(Box::new(TestArpeggiatorTempoController {
                        target: self.me.clone(),
                    })),
                    _ => None,
                }
            } else {
                None
            }
        }
    }
    impl TestArpeggiator {
        pub(crate) const CONTROL_PARAM_TEMPO: &str = "tempo";

        pub fn new_with(midi_channel_out: MidiChannel) -> Self {
            Self {
                midi_channel_out,
                ..Default::default()
            }
        }
        pub fn new_wrapped_with(midi_channel_out: MidiChannel) -> W<Self> {
            // TODO: Rc::new_cyclic() should make this easier, but I couldn't get the syntax right.
            // https://doc.rust-lang.org/std/rc/struct.Rc.html#method.new_cyclic

            let wrapped = Rc::new(RefCell::new(Self::new_with(midi_channel_out)));
            wrapped.borrow_mut().me = Rc::downgrade(&wrapped);
            wrapped
        }
    }

    #[derive(Debug)]
    pub struct TestArpeggiatorTempoController {
        target: WW<TestArpeggiator>,
    }
    impl SinksControl for TestArpeggiatorTempoController {
        fn handle_control(&mut self, _clock: &Clock, param: f32) {
            if let Some(target) = self.target.upgrade() {
                target.borrow_mut().tempo = param;
            }
        }
    }

    #[derive(Debug)]
    pub struct TestValueChecker {
        pub values: VecDeque<f32>,
        pub target: Rc<RefCell<dyn SourcesAudio>>,
        pub checkpoint: f32,
        pub checkpoint_delta: f32,
        pub time_unit: ClockTimeUnit,
    }

    impl WatchesClock for TestValueChecker {
        fn tick(&mut self, clock: &Clock) {
            // We have to check is_empty() twice
            // because we might still get called
            // back if someone else isn't done yet.
            if self.values.is_empty() {
                return;
            }
            if clock.time_for(&self.time_unit) >= self.checkpoint {
                const SAD_FLOAT_DIFF: f32 = 1.0e-4;
                assert_approx_eq!(
                    self.target.borrow_mut().source_audio(clock),
                    self.values[0],
                    SAD_FLOAT_DIFF
                );
                self.checkpoint += self.checkpoint_delta;
                self.values.pop_front();
            }
        }
    }

    impl Terminates for TestValueChecker {
        fn is_finished(&self) -> bool {
            self.values.is_empty()
        }
    }
}
