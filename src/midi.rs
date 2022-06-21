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
