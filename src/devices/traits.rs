use crate::primitives::{
    SinksAudio, SinksControl, SinksMidi, SourcesAudio, SourcesMidi, WatchesClock,
};

pub trait SequencerTrait: SourcesMidi + WatchesClock {}
impl<T: SourcesMidi + WatchesClock> SequencerTrait for T {}

pub trait AutomatorTrait: WatchesClock {}
impl<T: WatchesClock> AutomatorTrait for T {}

pub trait InstrumentTrait: SinksMidi + SourcesAudio + SinksControl + WatchesClock {}
impl<T: SinksMidi + SourcesAudio + SinksControl + WatchesClock> InstrumentTrait for T {}

pub trait ArpTrait: SourcesMidi + SinksMidi + SinksControl + WatchesClock {}
impl<T: SourcesMidi + SinksMidi + SinksControl + WatchesClock> ArpTrait for T {}

pub trait EffectTrait: SourcesAudio + SinksAudio + SinksControl + WatchesClock {}
impl<T: SourcesAudio + SinksAudio + SinksControl + WatchesClock> EffectTrait for T {}

#[cfg(test)]
mod tests {
    use std::{
        cell::RefCell,
        collections::HashMap,
        rc::{Rc, Weak},
    };

    use crate::{
        common::{self, MidiChannel, MidiMessage, MidiMessageType, MidiNote, MONO_SAMPLE_SILENCE},
        primitives::{
            clock::Clock,
            SinksControl,
            SinksControlParam::{self, Primary, Secondary},
            SourcesControl,
        },
    };

    use super::*;

    /// Keeps asking for time slices until end of specified lifetime.
    struct TestWatchesClock {
        lifetime_seconds: f32,
    }

    impl WatchesClock for TestWatchesClock {
        fn tick(&mut self, clock: &Clock) -> bool {
            clock.seconds >= self.lifetime_seconds
        }
    }

    impl TestWatchesClock {
        pub fn new(lifetime_seconds: f32) -> Self {
            Self { lifetime_seconds }
        }
    }

    #[derive(Default)]
    struct TestAudioSource {}

    impl SourcesAudio for TestAudioSource {
        fn source_audio(&mut self, _clock: &Clock) -> crate::common::MonoSample {
            0.
        }
    }

    impl TestAudioSource {
        fn new() -> Self {
            TestAudioSource {}
        }
    }

    #[derive(Default)]
    struct TestSinksAudio {
        audio_sources: Vec<Rc<RefCell<dyn SourcesAudio>>>,
    }
    impl SinksAudio for TestSinksAudio {
        fn sources(&mut self) -> &mut Vec<Rc<RefCell<dyn SourcesAudio>>> {
            &mut self.audio_sources
        }
    }
    impl TestSinksAudio {
        fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }

    #[derive(Default)]
    struct TestAutomationSource {
        control_sinks: Vec<Weak<RefCell<dyn SinksControl>>>,
    }

    impl SourcesControl for TestAutomationSource {
        fn control_sinks(&self) -> &[Weak<RefCell<dyn SinksControl>>] {
            &self.control_sinks
        }
        fn control_sinks_mut(&mut self) -> &mut Vec<Weak<RefCell<dyn SinksControl>>> {
            &mut self.control_sinks
        }
    }

    impl TestAutomationSource {
        fn new() -> Self {
            Self {
                ..Default::default()
            }
        }

        fn handle_test_event(&mut self, value: f32) {
            self.issue_control(&Clock::new(), &Primary { value });
        }
    }

    #[derive(Default)]
    struct TestAutomationSink {
        value: f32,
    }

    impl SinksControl for TestAutomationSink {
        fn handle_control(&mut self, _clock: &Clock, param: &SinksControlParam) {
            match param {
                Primary { value } => {
                    self.value = *value;
                }
                #[allow(unused_variables)]
                Secondary { value } => todo!(),
            }
        }
    }

    impl TestAutomationSink {
        fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }

    #[derive(Default)]
    struct TestMidiSource {
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
    }

    impl TestMidiSource {
        fn new() -> Self {
            Self {
                ..Default::default()
            }
        }

        fn source_some_midi(&mut self, clock: &Clock) {
            let message =
                MidiMessage::new_note_on(TestSinksMidi::MIDI_CHANNEL, MidiNote::C4 as u8, 100);
            self.issue_midi(clock, &message);
        }
    }

    #[derive(Default)]
    struct TestSinksMidi {
        midi_channel: MidiChannel,
        is_note_on: bool,
    }

    impl SinksMidi for TestSinksMidi {
        fn set_midi_channel(&mut self, midi_channel: MidiChannel) {
            self.midi_channel = midi_channel;
        }

        fn handle_midi_for_channel(&mut self, _clock: &Clock, message: &MidiMessage) {
            match message.status {
                MidiMessageType::NoteOn => {
                    self.is_note_on = true;
                }
                MidiMessageType::NoteOff => {
                    self.is_note_on = false;
                }
                MidiMessageType::ProgramChange => todo!(),
            }
        }

        fn midi_channel(&self) -> common::MidiChannel {
            Self::MIDI_CHANNEL
        }
    }

    impl TestSinksMidi {
        const MIDI_CHANNEL: MidiChannel = 7;

        fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }

    // Gets called with native functions telling it about external keyboard events.
    // Translates those into automation events that influence an arpeggiator,
    // which controls a MIDI instrument.
    //
    // This shows how all these traits work together.
    #[derive(Default)]
    struct TestSimpleKeyboard {
        control_sinks: Vec<Weak<RefCell<dyn SinksControl>>>,
    }

    impl SourcesControl for TestSimpleKeyboard {
        fn control_sinks(&self) -> &[Weak<RefCell<dyn SinksControl>>] {
            &self.control_sinks
        }
        fn control_sinks_mut(&mut self) -> &mut Vec<Weak<RefCell<dyn SinksControl>>> {
            &mut self.control_sinks
        }
    }

    impl TestSimpleKeyboard {
        fn new() -> Self {
            Self {
                ..Default::default()
            }
        }

        fn handle_keypress(&mut self, key: u8) {
            match key {
                1 => {
                    self.issue_control(&Clock::new(), &Primary { value: 0.5 });
                }
                _ => {}
            }
        }
    }

    #[derive(Default)]
    struct TestSimpleArpeggiator {
        tempo: f32,
        channels_to_sink_vecs: HashMap<MidiChannel, Vec<Weak<RefCell<dyn SinksMidi>>>>,
    }

    impl SinksControl for TestSimpleArpeggiator {
        fn handle_control(&mut self, _clock: &Clock, param: &SinksControlParam) {
            match param {
                Primary { value } => self.tempo = *value,
                #[allow(unused_variables)]
                Secondary { value } => todo!(),
            }
        }
    }

    impl SourcesMidi for TestSimpleArpeggiator {
        fn midi_sinks(&self) -> &HashMap<MidiChannel, Vec<Weak<RefCell<dyn SinksMidi>>>> {
            &self.channels_to_sink_vecs
        }
        fn midi_sinks_mut(
            &mut self,
        ) -> &mut HashMap<MidiChannel, Vec<Weak<RefCell<dyn SinksMidi>>>> {
            &mut self.channels_to_sink_vecs
        }
    }

    impl WatchesClock for TestSimpleArpeggiator {
        fn tick(&mut self, clock: &Clock) -> bool {
            // We don't actually pay any attention to self.tempo, but it's easy
            // enough to see that tempo could have influenced this MIDI message.
            self.issue_midi(
                &clock,
                &MidiMessage::new_note_on(TestSinksMidi::MIDI_CHANNEL, 60, 100),
            );
            true
        }
    }

    impl TestSimpleArpeggiator {
        fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }

    #[test]
    fn test_time_slicer() {
        let mut clock = Clock::new_test();
        let mut time_slicer = TestWatchesClock::new(1.0);

        loop {
            clock.tick();
            if time_slicer.tick(&mut clock) {
                break;
            }
        }
        assert!(clock.seconds >= 1.0);
    }

    #[test]
    fn test_audio_source() {
        let mut s = TestAudioSource::new();
        assert_eq!(s.source_audio(&Clock::new()), MONO_SAMPLE_SILENCE);
    }

    #[test]
    fn test_audio_sink() {
        let mut sink = TestSinksAudio::new();
        let source = Rc::new(RefCell::new(TestAudioSource::new()));
        assert!(sink.sources().is_empty());
        sink.add_audio_source(source);
        assert_eq!(sink.sources().len(), 1);
    }

    #[test]
    fn test_automation_source_and_sink() {
        // By itself, TestAutomationSource doesn't do much, so we test both Source/Sink together.
        let mut source = TestAutomationSource::new();
        let sink = Rc::new(RefCell::new(TestAutomationSink::new()));

        // Can we add a sink to the source?
        assert!(source.control_sinks().is_empty());
        let sink_weak = Rc::downgrade(&sink);
        source.add_control_sink(sink_weak);
        assert!(!source.control_sinks().is_empty());

        // Does the source propagate to its sinks?
        assert_eq!(sink.borrow().value, 0.0);
        source.handle_test_event(42.0);
        assert_eq!(sink.borrow().value, 42.0);
    }

    #[test]
    fn test_midi_source_and_sink() {
        let mut source = TestMidiSource::new();
        let sink = Rc::new(RefCell::new(TestSinksMidi::new()));

        assert!(source.midi_sinks().is_empty());
        let sink_down = Rc::downgrade(&sink);
        source.add_midi_sink(7, sink_down);
        assert!(!source.midi_sinks().is_empty());

        let clock = Clock::new_test();
        assert!(!sink.borrow().is_note_on);
        source.source_some_midi(&clock);
        assert!(sink.borrow().is_note_on);
    }

    #[test]
    fn test_keyboard_to_automation_to_midi() {
        let mut keyboard_interface = TestSimpleKeyboard::new();
        let arpeggiator = Rc::new(RefCell::new(TestSimpleArpeggiator::new()));
        let instrument = Rc::new(RefCell::new(TestSinksMidi::new()));

        let arpeggiator_weak = Rc::downgrade(&arpeggiator);
        keyboard_interface.add_control_sink(arpeggiator_weak);
        let sink = Rc::downgrade(&instrument);
        arpeggiator
            .borrow_mut()
            .add_midi_sink(TestSinksMidi::MIDI_CHANNEL, sink);

        assert_eq!(arpeggiator.borrow().tempo, 0.0);
        keyboard_interface.handle_keypress(1); // set tempo to 50%
        assert_eq!(arpeggiator.borrow().tempo, 0.5);

        let clock = Clock::new_test();

        assert!(!instrument.borrow().is_note_on);
        arpeggiator.borrow_mut().tick(&clock);
        assert!(instrument.borrow().is_note_on);
    }
}
