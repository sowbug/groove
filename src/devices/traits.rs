use crate::common::{
    self, MidiChannel, MidiMessage, MIDI_CHANNEL_RECEIVE_ALL, MIDI_CHANNEL_RECEIVE_NONE,
};
use crate::primitives::clock::Clock;
use crate::primitives::{SinksAudio, SinksControl, SourcesAudio, WatchesClock};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// A MidiSource controls MidiSinks through MidiMessages.
///
/// TODO: might some MidiSinks want to see *all* MidiMessages, not just some for
/// a single channel?
pub trait MidiSource {
    // TODO: if this gets too unwieldy, consider https://crates.io/crates/multimap
    fn midi_sinks(&mut self) -> &mut HashMap<MidiChannel, Vec<Rc<RefCell<dyn MidiSink>>>>;

    fn add_midi_sink(&mut self, sink: Rc<RefCell<dyn MidiSink>>, channel: MidiChannel) {
        self.midi_sinks().entry(channel).or_default().push(sink);
    }

    fn broadcast_midi_message(&mut self, clock: &Clock, message: &MidiMessage) {
        for sink in self.midi_sinks().entry(message.channel).or_default() {
            sink.borrow_mut().handle_midi_message(clock, message);
        }
    }
}

/// A MidiSink handles MidiMessages. By default, the trait handles MIDI channels
/// for us.
pub trait MidiSink {
    fn midi_channel(&self) -> common::MidiChannel {
        MIDI_CHANNEL_RECEIVE_NONE
    }
    fn set_midi_channel(&mut self, midi_channel: MidiChannel);

    // TODO: the "_midi" part of the method name is redundant, but when the method
    // is named "handle_message", it collides with the same method name in AutomationSink,
    // and I couldn't figure out how to disambiguate when the pointer is wrapped
    // in Rc<RefCell<>>. The error messages are clear, and the editor suggestions
    // sensible, but they don't work.
    fn handle_midi_message(&mut self, clock: &Clock, message: &MidiMessage) {
        if self.midi_channel() == MIDI_CHANNEL_RECEIVE_NONE {
            return;
        }
        if self.midi_channel() == MIDI_CHANNEL_RECEIVE_ALL || self.midi_channel() == message.channel
        {
            self.handle_message_for_channel(clock, message);
        }
    }

    // TODO: does clock really need to be passed through here?
    //
    // Samplers need it to know when a sample started playback, and synths need to know it
    // to keep track of when envelopes get triggered.
    //
    // They could either keep track of the tick() clock, which is inaccurate because
    // they only know the value from the last tick() call (which would put the responsibility on
    // them to do clock math), or we could tell everyone the current clock value at the
    // start of the event loop, which feels architecturally nicer, but it also reduces memory
    // locality (call everyone, then call everyone again) -- maybe not a problem.
    //
    // TL;DR: yeah, maybe it does really need to be here.
    fn handle_message_for_channel(&mut self, clock: &Clock, message: &MidiMessage);
}

pub trait SequencerTrait: MidiSource + WatchesClock {}
impl<T: MidiSource + WatchesClock> SequencerTrait for T {}

pub trait AutomatorTrait: WatchesClock {}
impl<T: WatchesClock> AutomatorTrait for T {}

pub trait InstrumentTrait: MidiSink + SourcesAudio + SinksControl + WatchesClock {}
impl<T: MidiSink + SourcesAudio + SinksControl + WatchesClock> InstrumentTrait for T {}

pub trait ArpTrait: MidiSource + MidiSink + SinksControl + WatchesClock {}
impl<T: MidiSource + MidiSink + SinksControl + WatchesClock> ArpTrait for T {}

pub trait EffectTrait: SourcesAudio + SinksAudio + SinksControl + WatchesClock {}
impl<T: SourcesAudio + SinksAudio + SinksControl + WatchesClock> EffectTrait for T {}

#[cfg(test)]
mod tests {
    use std::{
        cell::RefCell,
        rc::{Rc, Weak},
    };

    use crate::{
        common::{MidiNote, MONO_SAMPLE_SILENCE},
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
        channels_to_sink_vecs: HashMap<MidiChannel, Vec<Rc<RefCell<dyn MidiSink>>>>,
    }

    impl MidiSource for TestMidiSource {
        fn midi_sinks(&mut self) -> &mut HashMap<MidiChannel, Vec<Rc<RefCell<dyn MidiSink>>>> {
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
                MidiMessage::new_note_on(TestMidiSink::MIDI_CHANNEL, MidiNote::C4 as u8, 100);
            self.broadcast_midi_message(clock, &message);
        }
    }

    #[derive(Default)]
    struct TestMidiSink {
        midi_channel: MidiChannel,
        is_note_on: bool,
    }

    impl MidiSink for TestMidiSink {
        fn set_midi_channel(&mut self, midi_channel: MidiChannel) {
            self.midi_channel = midi_channel;
        }

        fn handle_message_for_channel(&mut self, _clock: &Clock, message: &MidiMessage) {
            match message.status {
                common::MidiMessageType::NoteOn => {
                    self.is_note_on = true;
                }
                common::MidiMessageType::NoteOff => {
                    self.is_note_on = false;
                }
                common::MidiMessageType::ProgramChange => todo!(),
            }
        }

        fn midi_channel(&self) -> common::MidiChannel {
            Self::MIDI_CHANNEL
        }
    }

    impl TestMidiSink {
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
        channels_to_sink_vecs: HashMap<MidiChannel, Vec<Rc<RefCell<dyn MidiSink>>>>,
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

    impl MidiSource for TestSimpleArpeggiator {
        fn midi_sinks(&mut self) -> &mut HashMap<MidiChannel, Vec<Rc<RefCell<dyn MidiSink>>>> {
            &mut self.channels_to_sink_vecs
        }
    }

    impl WatchesClock for TestSimpleArpeggiator {
        fn tick(&mut self, clock: &Clock) -> bool {
            // We don't actually pay any attention to self.tempo, but it's easy
            // enough to see that tempo could have influenced this MIDI message.
            self.broadcast_midi_message(
                &clock,
                &MidiMessage::new_note_on(TestMidiSink::MIDI_CHANNEL, 60, 100),
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
        let sink = Rc::new(RefCell::new(TestMidiSink::new()));

        assert!(source.midi_sinks().is_empty());
        source.add_midi_sink(sink.clone(), 7);
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
        let instrument = Rc::new(RefCell::new(TestMidiSink::new()));

        let arpeggiator_weak = Rc::downgrade(&arpeggiator);
        keyboard_interface.add_control_sink(arpeggiator_weak);
        arpeggiator
            .borrow_mut()
            .add_midi_sink(instrument.clone(), TestMidiSink::MIDI_CHANNEL);

        assert_eq!(arpeggiator.borrow().tempo, 0.0);
        keyboard_interface.handle_keypress(1); // set tempo to 50%
        assert_eq!(arpeggiator.borrow().tempo, 0.5);

        let clock = Clock::new_test();

        assert!(!instrument.borrow().is_note_on);
        arpeggiator.borrow_mut().tick(&clock);
        assert!(instrument.borrow().is_note_on);
    }
}
