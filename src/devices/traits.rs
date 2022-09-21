use crate::common::{
    self, MidiChannel, MidiMessage, MonoSample, MIDI_CHANNEL_RECEIVE_ALL, MIDI_CHANNEL_RECEIVE_NONE,
};
use crate::primitives::clock::Clock;
use std::cell::RefCell;
use std::rc::Rc;

pub trait TimeSlicer {
    // TODO - with the better granularity of traits, maybe we don't need this anymore.
    // fn needs_tick(&self) -> bool;
    // fn set_needs_tick(&mut self, needs_tick: bool);
    // fn reset_needs_tick(&mut self) {
    //     self.set_needs_tick(true);
    // }

    // Returns whether this device has completed all it has to do.
    // A typical audio effect or instrument will always return true,
    // because it doesn't know when it's done, but false would suggest
    // that it does need to keep doing work.
    //
    // More often used for MIDI instruments.
    #[allow(unused_variables)]
    fn tick(&mut self, clock: &Clock) -> bool {
        true
    }
}

pub trait AudioSource {
    fn sample(&mut self) -> MonoSample {
        0.
    }
}

pub trait AudioSink {
    #[allow(unused_variables)]
    fn add_source(&mut self, source: Rc<RefCell<dyn AudioSource>>) {}
}

/// Represents something that triggers an automation.
pub trait ExternalEvent {}

#[allow(unused_variables)]
pub trait AutomationSource {
    fn add_sink(&mut self, sink: Rc<RefCell<dyn AutomationSink>>);
    fn handle_event(&mut self, event: &dyn ExternalEvent);
}

/// Tells something to do something.
#[derive(Debug)]
pub enum AutomationMessage {
    UpdatePrimaryValue { value: f32 },
    UpdateSecondaryValue { value: f32 },
}

#[allow(unused_variables)]
pub trait AutomationSink {
    fn handle_message(&mut self, message: &AutomationMessage) {
        panic!("unhandled automation message {:?}", message);
    }
}

pub trait MidiSource: AutomationSource {
    // TODO: similar comment as handle_midi_message()
    fn add_midi_sink(&mut self, sink: Rc<RefCell<dyn MidiSink>>, channel: MidiChannel);
}

pub trait MidiSink: AutomationSink {
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

    // TODO: see whether anyone cares about clock... can we remove that param?
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
    use crate::primitives::clock::Clock;

    use super::TimeSlicer;

    /// Keeps asking for time slices until end of specified lifetime.
    struct TestTimeSlicer {
        lifetime_seconds: f32,
    }

    impl TestTimeSlicer {
        pub fn new(lifetime_seconds: f32) -> Self {
            Self { lifetime_seconds }
        }
    }

    impl TimeSlicer for TestTimeSlicer {
        fn tick(&mut self, clock: &Clock) -> bool {
            clock.seconds >= self.lifetime_seconds
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
}
