// Copyright (c) 2023 Mike Tsao. All rights reserved.

use bit_vec::BitVec;
use ensnare::{midi::prelude::*, prelude::*};
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

/// [MidiNoteMinder] watches a MIDI message stream and remembers which notes are
/// currently active (we've gotten a note-on without a note-off). Then, when
/// asked, it produces a list of MIDI message that turn off all active notes.
///
/// [MidiNoteMinder] doesn't know about [MidiChannel]s. It's up to the caller to
/// track channels, or else assume that if we got any message, it's for us, and
/// that the same is true for recipients of whatever we send.
#[derive(Debug)]
pub struct MidiNoteMinder {
    active_notes: BitVec,
}
impl Default for MidiNoteMinder {
    fn default() -> Self {
        Self {
            active_notes: BitVec::from_elem(128, false),
        }
    }
}
impl MidiNoteMinder {
    pub fn watch_message(&mut self, message: &MidiMessage) {
        #[allow(unused_variables)]
        match message {
            MidiMessage::NoteOff { key, vel } => {
                self.active_notes.set(key.as_int() as usize, false);
            }
            MidiMessage::NoteOn { key, vel } => {
                self.active_notes
                    .set(key.as_int() as usize, *vel != u7::from(0));
            }
            _ => {}
        }
    }

    pub fn generate_off_messages(&self) -> Vec<MidiMessage> {
        let mut v = Vec::default();
        for (i, active_note) in self.active_notes.iter().enumerate() {
            if active_note {
                v.push(MidiMessage::NoteOff {
                    key: u7::from_int_lossy(i as u8),
                    vel: u7::from(0),
                })
            }
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::{note_description_to_frequency, MidiNoteMinder};
    use ensnare::{
        midi::{new_note_off, new_note_on, MidiNote},
        prelude::FrequencyHz,
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

    #[test]
    fn midi_note_minder() {
        let mut mnm = MidiNoteMinder::default();

        assert!(mnm.generate_off_messages().is_empty());

        // Unexpected note-off doesn't explode
        mnm.watch_message(&new_note_off(42, 111));
        assert!(mnm.generate_off_messages().is_empty());

        // normal
        mnm.watch_message(&new_note_on(42, 99));
        let msgs = mnm.generate_off_messages();
        assert_eq!(msgs.len(), 1);
        assert_eq!(
            msgs[0],
            MidiMessage::NoteOff {
                key: u7::from(42),
                vel: u7::from(0)
            }
        );

        // duplicate on doesn't explode or add twice
        mnm.watch_message(&new_note_on(42, 88));
        let msgs = mnm.generate_off_messages();
        assert_eq!(msgs.len(), 1);
        assert_eq!(
            msgs[0],
            MidiMessage::NoteOff {
                key: u7::from(42),
                vel: u7::from(0)
            }
        );

        // normal
        mnm.watch_message(&new_note_off(42, 77));
        assert!(mnm.generate_off_messages().is_empty());

        // duplicate off doesn't explode
        mnm.watch_message(&new_note_off(42, 66));
        assert!(mnm.generate_off_messages().is_empty());

        // velocity zero treated same as note-off
        mnm.watch_message(&new_note_on(42, 99));
        assert_eq!(mnm.generate_off_messages().len(), 1);
        mnm.watch_message(&new_note_off(42, 99));
        assert_eq!(mnm.generate_off_messages().len(), 0);
        mnm.watch_message(&new_note_on(42, 99));
        assert_eq!(mnm.generate_off_messages().len(), 1);
        mnm.watch_message(&new_note_on(42, 0));
        assert_eq!(mnm.generate_off_messages().len(), 0);
    }
}
