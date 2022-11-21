use crate::{
    clock::{BeatValue, PerfectTimeUnit},
    traits::{HasUid, IsController, Updateable, Terminates},
    GrooveMessage,
};
use std::fmt::Debug;

#[derive(Clone, Debug, Default)]
pub struct Note {
    pub(crate) key: u8,
    pub(crate) velocity: u8,
    pub(crate) duration: PerfectTimeUnit, // expressed as multiple of the containing Pattern's note value.
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

#[derive(Clone, Debug, Default)]
pub struct PatternManager {
    uid: usize,
    patterns: Vec<Pattern<Note>>,
}
impl IsController for PatternManager {}
impl Updateable for PatternManager {
    type Message = GrooveMessage;

    // fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
    //     match message {
    //         ViewableMessage::PatternMessage(i, message) => {
    //             self.patterns_mut()[i].update(message);
    //         }
    //         _ => {
    //             dbg!(&message);
    //         }
    //     }
    //     Command::none()
    // }
}
impl Terminates for PatternManager {
    fn is_finished(&self) -> bool {
        true
    }
}
impl HasUid for PatternManager {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}

impl PatternManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn register(&mut self, pattern: Pattern<Note>) {
        self.patterns.push(pattern);
    }

    pub(crate) fn patterns(&self) -> &[Pattern<Note>] {
        &self.patterns
    }

    // TODO: this seems weird that we can give back a &mut to the slice.
    pub(crate) fn patterns_mut(&mut self) -> &mut [Pattern<Note>] {
        &mut self.patterns
    }
}
