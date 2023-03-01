// Copyright (c) 2023 Mike Tsao. All rights reserved.

pub use midly::{
    num::{u4, u7},
    MidiMessage,
};

pub type MidiChannel = u8;

/// Takes standard MIDI messages. Implementers can ignore MidiChannel if it's
/// not important, as the virtual cabling model tries to route only relevant
/// traffic to individual devices.
pub trait HandlesMidi {
    #[allow(unused_variables)]
    fn handle_midi_message(
        &mut self,
        message: &MidiMessage,
    ) -> Option<Vec<(MidiChannel, MidiMessage)>> {
        None
    }
}
