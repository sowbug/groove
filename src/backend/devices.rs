use std::cell::RefCell;
use std::rc::Rc;

// Composition of AudioSource and AudioSink and a bunch of other stuff.
// See https://users.rust-lang.org/t/dyn-multiple-traits-in-a-type-alias/21051
pub trait DeviceTrait {
    fn sources_midi(&self) -> bool {
        false
    }
    fn sinks_midi(&self) -> bool {
        false
    }
    fn sources_audio(&self) -> bool {
        false
    }
    fn sinks_audio(&self) -> bool {
        false
    }

    fn tick(&mut self, _time: f32) {}
    fn get_audio_sample(&self) -> f32 {
        0.
    }
    fn add_audio_source(&mut self, _audio_instrument: Rc<RefCell<dyn DeviceTrait>>) {}
    fn connect_midi_sink(&mut self, _audio_instrument: Rc<RefCell<dyn DeviceTrait>>) {}
    fn handle_midi_message(&mut self, _note: u8) {}
}
