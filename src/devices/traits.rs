use crate::common::{
    self, MidiMessage, MonoSample, MIDI_CHANNEL_RECEIVE_ALL, MIDI_CHANNEL_RECEIVE_NONE, MidiChannel,
};
use crate::primitives::clock::Clock;
use std::cell::RefCell;
use std::rc::Rc;

pub trait TimeSlice {
    fn needs_tick(&self) -> bool {
        true // TODO: this should switch to false when everyone has been retrofitted
    }

    fn reset_needs_tick(&mut self) {}

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

pub trait MidiSource {
    #[allow(unused_variables)]
    fn connect_midi_sink(&mut self, device: Rc<RefCell<dyn MidiSink>>) {}
}

pub trait MidiSink {
    fn midi_channel(&self) -> common::MidiChannel {
        MIDI_CHANNEL_RECEIVE_NONE
    }

    fn set_midi_channel(&mut self, midi_channel: MidiChannel);

    fn handle_midi_message(&mut self, message: &MidiMessage, clock: &Clock) {
        if self.midi_channel() == MIDI_CHANNEL_RECEIVE_NONE {
            return;
        }
        if self.midi_channel() == MIDI_CHANNEL_RECEIVE_ALL || self.midi_channel() == message.channel
        {
            self.__handle_midi_message(message, clock);
        }
    }

    #[allow(unused_variables)]
    fn __handle_midi_message(&mut self, message: &MidiMessage, clock: &Clock);
}

pub trait AudioSource {
    fn get_audio_sample(&mut self) -> MonoSample {
        0.
    }
}

pub trait AudioSink {
    #[allow(unused_variables)]
    fn add_audio_source(&mut self, device: Rc<RefCell<dyn AudioSource>>) {}
}

pub trait AutomationSink {
    #[allow(unused_variables)]
    fn handle_automation(&mut self, param_name: &String, param_value: f32) {}
}

pub trait SequencerTrait: MidiSource + TimeSlice {}
impl<T: MidiSource + TimeSlice> SequencerTrait for T {}

pub trait AutomatorTrait: TimeSlice {}
impl<T: TimeSlice> AutomatorTrait for T {}

pub trait InstrumentTrait: MidiSink + AudioSource + AutomationSink + TimeSlice {}
impl<T: MidiSink + AudioSource + AutomationSink + TimeSlice> InstrumentTrait for T {}

pub trait EffectTrait: AudioSource + AudioSink + AutomationSink + TimeSlice {}
impl<T: AudioSource + AudioSink + AutomationSink + TimeSlice> EffectTrait for T {}
