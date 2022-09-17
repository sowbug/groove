use crate::common::{MidiMessage, MonoSample};
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
    fn tick(&mut self, clock: &Clock) -> bool {
        true
    }
}

pub trait MidiSource {
    fn connect_midi_sink(&mut self, device: Rc<RefCell<dyn MidiSink>>) {}
}

pub trait MidiSink {
    fn handle_midi_message(&mut self, message: &MidiMessage, clock: &Clock) {}
}

pub trait AudioSource {
    fn get_audio_sample(&mut self) -> MonoSample {
        0.
    }
}

pub trait AudioSink {
    fn add_audio_source(&mut self, device: Rc<RefCell<dyn AudioSource>>) {}
}

pub trait AutomationSink {
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

// // Composition of AudioSource and AudioSink and a bunch of other stuff.
// // See https://users.rust-lang.org/t/dyn-multiple-traits-in-a-type-alias/21051
// #[allow(unused_variables)]
// pub trait DeviceTrait {
//     fn sources_midi(&self) -> bool {
//         false
//     }
//     fn sinks_midi(&self) -> bool {
//         false
//     }
//     fn sources_audio(&self) -> bool {
//         false
//     }
//     fn sinks_audio(&self) -> bool {
//         false
//     }

// }
