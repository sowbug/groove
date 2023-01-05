use groove_macros::Uid;

use crate::{
    clock::{BeatValue, PerfectTimeUnit},
    messages::EntityMessage,
    traits::{HasUid, IsController, Terminates, Updateable},
};
use std::fmt::Debug;

#[derive(Clone, Debug, Default)]
pub struct Note {
    pub key: u8,
    pub velocity: u8,
    pub duration: PerfectTimeUnit, // expressed as multiple of the containing Pattern's note value.
}

#[derive(Clone, Debug, Default)]
pub struct Pattern<T: Default> {
    pub note_value: Option<BeatValue>,
    pub notes: Vec<Vec<T>>,
}

impl<T: Default> Pattern<T> {
    #[allow(dead_code)]
    pub(crate) fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    fn note_to_value(note: &str) -> u8 {
        // TODO https://en.wikipedia.org/wiki/Scientific_pitch_notation labels,
        // e.g., for General MIDI percussion
        note.parse().unwrap_or_default()
    }
}

// TODO: I got eager with the <T> and then tired when I realized it would affect
// more stuff. Thus there's only an implementation for Note.
impl Pattern<Note> {
    pub(crate) fn from_settings(settings: &crate::settings::PatternSettings) -> Self {
        let mut r = Self {
            note_value: settings.note_value.clone(),
            notes: Vec::new(),
        };
        for note_sequence in settings.notes.iter() {
            let mut note_vec = Vec::new();
            for note in note_sequence.iter() {
                note_vec.push(Note {
                    key: Self::note_to_value(note),
                    velocity: 127,
                    duration: PerfectTimeUnit(1.0),
                });
            }
            r.notes.push(note_vec);
        }
        r
    }
}

#[derive(Clone, Debug, Default, Uid)]
pub struct PatternManager {
    uid: usize,
    patterns: Vec<Pattern<Note>>,
}
impl IsController for PatternManager {}
impl Updateable for PatternManager {
    type Message = EntityMessage;
}
impl Terminates for PatternManager {
    fn is_finished(&self) -> bool {
        true
    }
}

impl PatternManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn register(&mut self, pattern: Pattern<Note>) {
        self.patterns.push(pattern);
    }

    pub fn patterns(&self) -> &[Pattern<Note>] {
        &self.patterns
    }
}
