use crate::{
    clock::{BeatValue, PerfectTimeUnit},
    messages::EntityMessage,
};
use groove_core::{
    midi::HandlesMidi,
    traits::{HasUid, IsController, Resets, TicksWithMessages},
};
use groove_macros::Uid;
use std::fmt::Debug;

#[derive(Clone, Debug)]
pub enum PatternMessage {
    SomethingHappened,
    ButtonPressed,
}

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

    pub fn note_to_value(note: &str) -> u8 {
        // TODO https://en.wikipedia.org/wiki/Scientific_pitch_notation labels,
        // e.g., for General MIDI percussion
        note.parse().unwrap_or_default()
    }
}

// TODO: why is there so much paperwork for a vector?
#[derive(Clone, Debug, Default, Uid)]
pub struct PatternManager {
    uid: usize,
    patterns: Vec<Pattern<Note>>,
}
impl IsController<EntityMessage> for PatternManager {}
impl HandlesMidi for PatternManager {}
impl Resets for PatternManager {}
impl TicksWithMessages<EntityMessage> for PatternManager {
    type Message = EntityMessage;

    #[allow(unused_variables)]
    fn tick(&mut self, tick_count: usize) -> (Option<Vec<Self::Message>>, usize) {
        (None, 0)
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
