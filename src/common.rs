use std::cmp::Ordering;

use serde::{Serialize, Deserialize};

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum WaveformType {
    None,
    Sine,
    Square,
    PulseWidth(f32),
    Triangle,
    Sawtooth,
    Noise,
}

impl Default for WaveformType {
    fn default() -> Self {
        WaveformType::None
    }
}

#[allow(dead_code)]
pub enum MidiNote {
    None = 0,
    A0 = 21,
    D3 = 50,
    C4 = 60,
    A4 = 69,
    G9 = 127,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone, Serialize, Deserialize)]
pub enum MidiMessageType {
    NoteOn = 0b1001,
    NoteOff = 0b1000,
    ProgramChange = 0b1100,
}
#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Clone, Serialize, Deserialize)]
pub struct MidiMessage {
    // status and channel are normally packed into one byte, but for ease of use
    // we're unpacking here.
    pub status: MidiMessageType,
    pub channel: u8,
    pub data1: u8,
    pub data2: u8,
}

impl MidiMessage {
    pub fn note_to_frequency(note: u8) -> f32 {
        2.0_f32.powf((note as f32 - 69.0) / 12.0) * 440.0
    }

    pub fn note_type_to_frequency(midi_note: MidiNote) -> f32 {
        2.0_f32.powf((midi_note as u8 as f32 - 69.0) / 12.0) * 440.0
    }

    pub fn to_frequency(&self) -> f32 {
        Self::note_to_frequency(self.data1)
    }

    pub(crate) fn new_note_on(channel: u8, note: u8, vel: u8) -> Self {
        MidiMessage {
            status: MidiMessageType::NoteOn,
            channel,
            data1: note,
            data2: vel,
        }
    }

    pub(crate) fn new_note_off(channel: u8, note: u8, vel: u8) -> Self {
        MidiMessage {
            status: MidiMessageType::NoteOff,
            channel,
            data1: note,
            data2: vel,
        }
    }

    pub(crate) fn new_program_change(channel: u8, program: u8) -> Self {
        MidiMessage {
            status: MidiMessageType::ProgramChange,
            channel: channel,
            data1: program,
            data2: 0,
        }
    }
}

#[derive(Eq, Debug, Clone, Serialize, Deserialize)]
pub struct OrderedMidiMessage {
    pub when: u32,
    pub message: MidiMessage,
}

impl Ord for OrderedMidiMessage {
    fn cmp(&self, other: &Self) -> Ordering {
        self.when.cmp(&other.when)
    }
}

impl PartialOrd for OrderedMidiMessage {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for OrderedMidiMessage {
    fn eq(&self, other: &Self) -> bool {
        self.when == other.when
    }
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;

    use super::*;

    impl MidiMessage {
        pub fn note_on_c4() -> MidiMessage {
            MidiMessage::new_note_on(0, MidiNote::C4 as u8, 0)
        }

        pub fn note_off_c4() -> MidiMessage {
            MidiMessage::new_note_off(0, MidiNote::C4 as u8, 0)
        }
    }

    #[test]
    fn test_note_to_frequency() {
        assert_approx_eq!(
            MidiMessage::new_note_on(0, MidiNote::C4 as u8, 0).to_frequency(),
            261.625549
        );
        assert_approx_eq!(MidiMessage::new_note_on(0, 0, 0).to_frequency(), 8.175798);
        assert_approx_eq!(
            MidiMessage::new_note_on(0, MidiNote::G9 as u8, 0).to_frequency(),
            12543.855
        );
    }
}
