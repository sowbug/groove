use crate::common::{
    self, MidiChannel, MidiMessage, MonoSample, MIDI_CHANNEL_RECEIVE_ALL, MIDI_CHANNEL_RECEIVE_NONE,
};
use crate::primitives::clock::Clock;
use std::cell::RefCell;
use std::rc::Rc;

/// Represents an aggregate that can do its work in time slices.
/// Almost everything about digital music works this way. For example,
/// a sine wave isn't a continuous wave. Rather, it's a series of
/// samples across time. The sine wave can always tell you its value,
/// as long as you provide a _when_ for the moment in time that you're
/// asking about.
///
/// TimeSlicer's most natural unit of time is a *sample*. Typical digital
/// sounds are 44.1KHz, so a tick in that case would be for 1/44100th of
/// a second.
pub trait TimeSlicer {
    // Returns whether this device has completed all it has to do.
    // A typical audio effect or instrument will always return true,
    // because it doesn't know when it's done, but false would suggest
    // that it does need to keep doing work.
    //
    // More often used for MIDI instruments.
    #[allow(unused_variables)]
    fn tick(&mut self, clock: &Clock) -> bool;
}

/// An AudioSource can provide audio in the form of digital samples.
pub trait AudioSource {
    fn sample(&mut self) -> MonoSample;
}

/// An AudioSink can do something with an AudioSource's audio samples.
/// It is given a set of AudioSources, and when it needs to do its work,
/// it asks them for their samples.
pub trait AudioSink {
    fn audio_sources(&mut self) -> &mut Vec<Rc<RefCell<dyn AudioSource>>>;

    fn add_audio_source(&mut self, source: Rc<RefCell<dyn AudioSource>>) {
        self.audio_sources().push(source);
    }
}

/// An AutomationSource controls AutomationSinks through AutomationMessages.
pub trait AutomationSource {
    fn automation_sinks(&mut self) -> &mut Vec<Rc<RefCell<dyn AutomationSink>>>;

    fn add_automation_sink(&mut self, sink: Rc<RefCell<dyn AutomationSink>>) {
        self.automation_sinks().push(sink);
    }

    fn broadcast_automation_message(&mut self, message: &AutomationMessage) {
        for sink in self.automation_sinks().clone() {
            sink.borrow_mut().handle_automation_message(message);
        }
    }
}

#[derive(Debug)]
pub enum AutomationMessage {
    UpdatePrimaryValue { value: f32 },
    UpdateSecondaryValue { value: f32 },
    UpdateNamedValue { name: String, value: f32 },
}

/// AutomationSinks agree to handle AutomationMessages.
pub trait AutomationSink {
    fn handle_automation_message(&mut self, message: &AutomationMessage);
}

/// A MidiSource controls MidiSinks through MidiMessages.
///
/// TODO: might some MidiSinks want to see *all* MidiMessages, not just some for
/// a single channel?
pub trait MidiSource {
    // TODO: we should change this to a map
    fn midi_sinks(&mut self) -> &mut Vec<Rc<RefCell<dyn MidiSink>>>;

    #[allow(unused_variables)]
    fn add_midi_sink(&mut self, sink: Rc<RefCell<dyn MidiSink>>, channel: MidiChannel) {
        self.midi_sinks().push(sink);
    }

    fn broadcast_midi_message(&mut self, clock: &Clock, message: &MidiMessage) {
        for sink in self.midi_sinks().clone() {
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

pub trait SequencerTrait: MidiSource + TimeSlicer {}
impl<T: MidiSource + TimeSlicer> SequencerTrait for T {}

pub trait AutomatorTrait: TimeSlicer {}
impl<T: TimeSlicer> AutomatorTrait for T {}

pub trait InstrumentTrait: MidiSink + AudioSource + AutomationSink + TimeSlicer {}
impl<T: MidiSink + AudioSource + AutomationSink + TimeSlicer> InstrumentTrait for T {}

pub trait EffectTrait: AudioSource + AudioSink + AutomationSink + TimeSlicer {}
impl<T: AudioSource + AudioSink + AutomationSink + TimeSlicer> EffectTrait for T {}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use crate::{
        common::{MidiNote, MONO_SAMPLE_SILENCE},
        primitives::clock::Clock,
    };

    use super::{AutomationMessage, *};

    /// Keeps asking for time slices until end of specified lifetime.
    struct TestTimeSlicer {
        lifetime_seconds: f32,
    }

    impl TimeSlicer for TestTimeSlicer {
        fn tick(&mut self, clock: &Clock) -> bool {
            clock.seconds >= self.lifetime_seconds
        }
    }

    impl TestTimeSlicer {
        pub fn new(lifetime_seconds: f32) -> Self {
            Self { lifetime_seconds }
        }
    }

    #[derive(Default)]
    struct TestAudioSource {}

    impl AudioSource for TestAudioSource {
        fn sample(&mut self) -> crate::common::MonoSample {
            0.
        }
    }

    impl TestAudioSource {
        fn new() -> Self {
            TestAudioSource {}
        }
    }

    #[derive(Default)]
    struct TestAudioSink {
        audio_sources: Vec<Rc<RefCell<dyn AudioSource>>>,
    }

    impl AudioSink for TestAudioSink {
        fn audio_sources(&mut self) -> &mut Vec<Rc<RefCell<dyn AudioSource>>> {
            &mut self.audio_sources
        }
    }

    impl TestAudioSink {
        fn new() -> Self {
            TestAudioSink {
                ..Default::default()
            }
        }
    }

    #[derive(Default)]
    struct TestAutomationSource {
        sinks: Vec<Rc<RefCell<dyn AutomationSink>>>,
    }

    impl AutomationSource for TestAutomationSource {
        fn automation_sinks(&mut self) -> &mut Vec<Rc<RefCell<dyn AutomationSink>>> {
            &mut self.sinks
        }
    }

    impl TestAutomationSource {
        fn new() -> Self {
            Self {
                ..Default::default()
            }
        }

        fn handle_test_event(&mut self, value: f32) {
            self.broadcast_automation_message(&AutomationMessage::UpdatePrimaryValue { value });
        }
    }

    #[derive(Default)]
    struct TestAutomationSink {
        value: f32,
    }

    impl AutomationSink for TestAutomationSink {
        fn handle_automation_message(&mut self, message: &AutomationMessage) {
            match message {
                AutomationMessage::UpdatePrimaryValue { value } => {
                    self.value = *value;
                }
                #[allow(unused_variables)]
                AutomationMessage::UpdateSecondaryValue { value } => {
                    todo!()
                }
                #[allow(unused_variables)]
                AutomationMessage::UpdateNamedValue { name, value } => todo!(),
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
        sinks: Vec<Rc<RefCell<dyn MidiSink>>>,
    }

    impl MidiSource for TestMidiSource {
        fn midi_sinks(&mut self) -> &mut Vec<Rc<RefCell<dyn MidiSink>>> {
            &mut self.sinks
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
        automation_sinks: Vec<Rc<RefCell<dyn AutomationSink>>>,
    }

    impl AutomationSource for TestSimpleKeyboard {
        fn automation_sinks(&mut self) -> &mut Vec<Rc<RefCell<dyn AutomationSink>>> {
            &mut self.automation_sinks
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
                    self.broadcast_automation_message(&AutomationMessage::UpdatePrimaryValue {
                        value: 0.5,
                    });
                }
                _ => {}
            }
        }
    }

    #[derive(Default)]
    struct TestSimpleArpeggiator {
        tempo: f32,
        midi_sinks: Vec<Rc<RefCell<dyn MidiSink>>>,
    }

    impl AutomationSink for TestSimpleArpeggiator {
        fn handle_automation_message(&mut self, message: &AutomationMessage) {
            #[allow(unused_variables)]
            match message {
                AutomationMessage::UpdatePrimaryValue { value } => {
                    self.tempo = *value;
                }
                AutomationMessage::UpdateSecondaryValue { value } => todo!(),
                AutomationMessage::UpdateNamedValue { name, value } => todo!(),
            }
        }
    }

    impl MidiSource for TestSimpleArpeggiator {
        fn midi_sinks(&mut self) -> &mut Vec<Rc<RefCell<dyn MidiSink>>> {
            &mut self.midi_sinks
        }
    }

    impl TimeSlicer for TestSimpleArpeggiator {
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
        let mut time_slicer = TestTimeSlicer::new(1.0);

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
        assert_eq!(s.sample(), MONO_SAMPLE_SILENCE);
    }

    #[test]
    fn test_audio_sink() {
        let mut sink = TestAudioSink::new();
        let source = Rc::new(RefCell::new(TestAudioSource::new()));
        assert!(sink.audio_sources().is_empty());
        sink.add_audio_source(source);
        assert_eq!(sink.audio_sources().len(), 1);
    }

    #[test]
    fn test_automation_source_and_sink() {
        // By itself, TestAutomationSource doesn't do much, so we test both Source/Sink together.
        let mut source = TestAutomationSource::new();
        let sink = Rc::new(RefCell::new(TestAutomationSink::new()));

        // Can we add a sink to the source?
        assert!(source.automation_sinks().is_empty());
        source.add_automation_sink(sink.clone());
        assert!(!source.automation_sinks().is_empty());

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

        keyboard_interface.add_automation_sink(arpeggiator.clone());
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
