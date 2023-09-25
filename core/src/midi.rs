// Copyright (c) 2023 Mike Tsao. All rights reserved.

use bit_vec::BitVec;
use ensnare_core::{midi::prelude::*, prelude::*};
use std::fmt::Debug;

pub fn note_description_to_frequency(text: &str) -> Option<FrequencyHz> {
    if text.is_empty() {
        return None;
    }
    if text.contains('.') {
        if let Ok(parsed_float) = text.parse::<ParameterType>() {
            if parsed_float > 0.0 {
                return Some(FrequencyHz::from(parsed_float));
            }
        }
    } else if let Ok(note) = text.parse::<u8>() {
        return Some(u7::from(note).into());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::note_description_to_frequency;
    use ensnare_core::{
        midi::{new_note_off, new_note_on, MidiNote},
        prelude::*,
    };
    use midly::{num::u7, MidiMessage};

    #[test]
    fn note_to_frequency() {
        // https://www.colincrawley.com/midi-note-to-audio-frequency-calculator/
        assert_eq!(
            FrequencyHz::from(MidiNote::C0),
            16.351_597_831_287_414.into()
        );
        assert_eq!(
            FrequencyHz::from(MidiNote::C4),
            261.625_565_300_598_6.into()
        );
        assert_eq!(
            FrequencyHz::from(MidiNote::D5),
            587.329_535_834_815_1.into()
        );
        assert_eq!(
            FrequencyHz::from(MidiNote::D6),
            1_174.659_071_669_630_3.into()
        );
        assert_eq!(
            FrequencyHz::from(MidiNote::G9),
            12_543.853_951_415_975.into()
        );
    }

    #[test]
    fn text_to_frequency() {
        assert_eq!(
            note_description_to_frequency("440.0").unwrap().value(),
            440.0,
            "A floating-point number should parse as a frequency"
        );
        assert_eq!(
            note_description_to_frequency("69").unwrap().value(),
            440.0,
            "An integer should parse as a MIDI note with 69 = 440.0Hz"
        );
        assert_eq!(
            note_description_to_frequency("0").unwrap().value(),
            8.175_798_915_643_707,
            "MIDI note zero is valid!"
        );
        assert_eq!(
            note_description_to_frequency("-4"),
            None,
            "Negative note numbers are invalid"
        );
        assert_eq!(
            note_description_to_frequency("0.0"),
            None,
            "Frequency zero is not valid (design decision)"
        );
        assert_eq!(
            note_description_to_frequency("-440.0"),
            None,
            "Negative frequencies are invalid"
        );
        assert_eq!(
            note_description_to_frequency("1.2.3.4"),
            None,
            "Gobbledygook should fail to parse"
        );
        assert_eq!(
            note_description_to_frequency("chartreuse"),
            None,
            "Gobbledygook should fail to parse"
        );
        assert_eq!(
            note_description_to_frequency(""),
            None,
            "Empty string should fail to parse"
        );
    }
}
