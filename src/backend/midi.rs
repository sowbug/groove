use std::cell::RefCell;
use std::rc::Rc;
pub enum MIDIMessageType {
    NoteOn = 0x1001,
    NoteOff = 0x1000,
}
pub struct MIDIMessage {
    // status and channel are normally packed into one byte, but for ease of use
    // we're unpacking here.
    pub status: MIDIMessageType,
    pub channel: u8,
    pub data1: u8,
    pub data2: u8,
}

impl MIDIMessage {
    pub fn to_frequency(&self) -> f32 {
        match self.data1 {
            60 => 261.63,
            66 => 392.00,
            _ => 0.,
        }
    }
}

pub trait MIDIReceiverTrait {
    fn handle_midi(&mut self, midi_message: MIDIMessage) -> bool;
}

use super::clock::Clock;
use super::devices::DeviceTrait;

pub struct Sequencer {
    sinks: Vec<Rc<RefCell<dyn DeviceTrait>>>,
}

impl Sequencer {
    pub fn new() -> Sequencer {
        Sequencer { sinks: Vec::new() }
    }
}

impl DeviceTrait for Sequencer {
    fn sources_midi(&self) -> bool {
        true
    }

    fn tick(&mut self, clock: &Clock) {
        let note = if clock.real_clock < 0.25 {
            0
        } else if clock.real_clock < 0.50 {
            60
        } else if clock.real_clock < 0.75 {
            66
        } else {
            0
        };
        // decide what there is to do
        for i in self.sinks.clone() {
            i.borrow_mut().handle_midi_message(note);
        }
    }

    fn connect_midi_sink(&mut self, device: Rc<RefCell<dyn DeviceTrait>>) {
        self.sinks.push(device);
    }
}
