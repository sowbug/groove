pub enum MidiMessageType {
    NoteOn = 0x1001,
    _NoteOff = 0x1000,
}
pub struct MidiMessage {
    // status and channel are normally packed into one byte, but for ease of use
    // we're unpacking here.
    pub status: MidiMessageType,
    pub channel: u8,
    pub data1: u8,
    pub data2: u8,
}

impl MidiMessage {
    pub fn to_frequency(&self) -> f32 {
        match self.data1 {
            0 => 0.,
            _ => 2.0_f32.powf((self.data1 as f32 - 69.0) / 12.0) * 440.0,
        }
    }
}
