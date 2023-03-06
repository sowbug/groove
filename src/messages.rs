//! The [messages] module defines the App's Iced messages.

use groove_core::{
    midi::{MidiChannel, MidiMessage},
    traits::MessageBounds,
    StereoSample,
};
use groove_entities::EntityMessage;
use std::fmt::Debug;

#[derive(Clone, Debug)]
pub enum GrooveMessage {
    /// A wrapped EntityMessage that contains the uid of the receipient in
    /// addition to the EntityMessage.
    EntityMessage(usize, EntityMessage),

    /// A MIDI message that has arrived from outside Groove, typically from
    /// MidiInputHandler.
    MidiFromExternal(MidiChannel, MidiMessage),

    /// A MIDI message that should be routed from Groove to outside.
    MidiToExternal(MidiChannel, MidiMessage),

    /// An audio sample for the current time slice. Intended to be sent in
    /// response to a downstream Tick, and consumed by the application.
    AudioOutput(StereoSample),

    /// If sent, then the Orchestrator performance is done. Intended to be sent
    /// in response to a downstream Tick, and consumed by the application.
    OutputComplete,

    LoadProject(String),
    LoadedProject(String, Option<String>),
}
impl MessageBounds for GrooveMessage {}

#[derive(Debug)]
pub struct Response<T>(pub Internal<T>);

#[derive(Debug)]
pub enum Internal<T> {
    None,
    Single(T),
    Batch(Vec<T>),
}

impl<T> Response<T> {
    pub const fn none() -> Self {
        Self(Internal::None)
    }

    pub const fn single(action: T) -> Self {
        Self(Internal::Single(action))
    }

    pub fn batch(commands: impl IntoIterator<Item = Response<T>>) -> Self {
        let mut batch = Vec::new();

        for Response(command) in commands {
            match command {
                Internal::None => {}
                Internal::Single(command) => batch.push(command),
                Internal::Batch(commands) => batch.extend(commands),
            }
        }
        if batch.is_empty() {
            Self(Internal::None)
        } else {
            Self(Internal::Batch(batch))
        }
    }
}
