// Copyright (c) 2023 Mike Tsao. All rights reserved.

pub use midly::{
    num::{u4, u7},
    MidiMessage,
};

use crate::ParameterType;

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

/// There are two different mappings of piano notes to MIDI numbers. They both
/// agree that Midi note 0 is a C, but they otherwise differ by an octave. I
/// originally picked C4=60, because that was the top Google search result's
/// answer, but it seems like a slight majority thinks C3=60. I'm going to leave
/// it as-is so that I don't have to rename my test data files. I don't think it
/// matters because we're not actually mapping these to anything user-visible.
///
/// These also correspond to <https://en.wikipedia.org/wiki/Piano_key_frequencies>
#[derive(Clone, Copy, Debug, Default)]
pub enum MidiNote {
    None = 0,
    C0 = 12,
    Cs0 = 13,
    D0 = 14,
    Ds0 = 15,
    E0 = 16,
    F0 = 17,
    Fs0 = 18,
    G0 = 19,
    Gs0 = 20,
    A0 = 21,
    As0 = 22,
    B0 = 23,
    C1 = 24,
    C2 = 36,
    C3 = 48,
    D3 = 50,
    #[default]
    C4 = 60,
    G4 = 67,
    A4 = 69,
    C5 = 72,
    D5 = 74,
    D6 = 86,
    G9 = 127,
}

pub fn note_to_frequency(note: u8) -> ParameterType {
    2.0_f64.powf((note as ParameterType - 69.0) / 12.0) * 440.0
}

pub fn note_type_to_frequency(midi_note: MidiNote) -> ParameterType {
    2.0_f64.powf((midi_note as u8 as ParameterType - 69.0) / 12.0) * 440.0
}

pub fn note_description_to_frequency(text: String, default: ParameterType) -> ParameterType {
    if !text.is_empty() {
        if text.contains('.') {
            let frequency = text.parse::<ParameterType>().unwrap_or(default);
            if frequency > 0.0 {
                return frequency;
            }
        } else if let Ok(note) = text.parse::<u8>() {
            return note_to_frequency(note);
        }
    }
    default
}

pub fn new_note_on(note: u8, vel: u8) -> MidiMessage {
    MidiMessage::NoteOn {
        key: u7::from(note),
        vel: u7::from(vel),
    }
}

pub fn new_note_off(note: u8, vel: u8) -> MidiMessage {
    MidiMessage::NoteOff {
        key: u7::from(note),
        vel: u7::from(vel),
    }
}

#[cfg(test)]
mod tests {
    use super::note_description_to_frequency;
    use crate::midi::{note_type_to_frequency, MidiNote};

    #[test]
    fn note_to_frequency() {
        // https://www.colincrawley.com/midi-note-to-audio-frequency-calculator/
        assert_eq!(note_type_to_frequency(MidiNote::C0), 16.351_597_831_287_414);
        assert_eq!(note_type_to_frequency(MidiNote::C4), 261.625_565_300_598_6);
        assert_eq!(note_type_to_frequency(MidiNote::D5), 587.329_535_834_815_1);
        assert_eq!(
            note_type_to_frequency(MidiNote::D6),
            1_174.659_071_669_630_3
        );
        assert_eq!(note_type_to_frequency(MidiNote::G9), 12_543.853_951_415_975);
    }

    #[test]
    fn text_to_frequency() {
        assert_eq!(
            note_description_to_frequency("440.0".to_string(), 999.9),
            440.0,
            "A floating-point number should parse as a frequency"
        );
        assert_eq!(
            note_description_to_frequency("69".to_string(), 999.9),
            440.0,
            "An integer should parse as a MIDI note with 69 = 440.0Hz"
        );
        assert_eq!(
            note_description_to_frequency("0".to_string(), 999.9),
            8.175_798_915_643_707,
            "MIDI note zero is valid!"
        );
        assert_eq!(
            note_description_to_frequency("-4".to_string(), 999.9),
            999.9,
            "Negative note numbers are invalid"
        );
        assert_eq!(
            note_description_to_frequency("0.0".to_string(), 999.9),
            999.9,
            "Frequency zero is not valid (design decision)"
        );
        assert_eq!(
            note_description_to_frequency("-440.0".to_string(), 999.9),
            999.9,
            "Negative frequencies are invalid"
        );
        assert_eq!(
            note_description_to_frequency("1.2.3.4".to_string(), 999.9),
            999.9,
            "Gobbledygook should parse as default"
        );
        assert_eq!(
            note_description_to_frequency("chartreuse".to_string(), 999.9),
            999.9,
            "Gobbledygook should parse as default"
        );
        assert_eq!(
            note_description_to_frequency("".to_string(), 999.9),
            999.9,
            "Empty string should parse as default"
        );
    }
}
