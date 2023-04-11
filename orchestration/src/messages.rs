// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! The [messages](crate::messages) module defines the app's Iced messages.

use crate::entities::OtherEntityMessage;
use groove_core::{
    midi::{MidiChannel, MidiMessage},
    traits::MessageBounds,
};
use groove_entities::EntityMessage;
use std::fmt::Debug;

#[derive(Debug)]
pub enum GrooveInput {
    EntityMessage(usize, EntityMessage),

    /// A MIDI message that has arrived from outside Groove, typically from
    /// MidiInputHandler.
    MidiFromExternal(MidiChannel, MidiMessage),

    /// Ask the engine to add a control link.
    AddControlLink(ControlLink),

    /// Ask the engine to remove a control link.
    RemoveControlLink(ControlLink),

    /// An entity has been updated on the app side, and the engine should record the changes.
    Update(usize, OtherEntityMessage),

    /// Orchestrator should ask everyone to start playing.
    Play,

    /// Orchestrator should ask everyone to stop playing.
    Stop,

    /// Orchestrator should ask everyone to reset to start of performance.
    SkipToStart,

    /// Someone has requested this sample rate.
    SetSampleRate(usize),
}
impl MessageBounds for GrooveInput {}

#[derive(Clone, Debug)]
pub enum GrooveEvent {
    EntityMessage(usize, EntityMessage),

    /// Indicates that an Orchestrator performance has begun. The app should
    /// adjust the GUI state accordingly.
    PlaybackStarted,

    /// Indicates that the Orchestrator performance is done. The app should
    /// adjust the GUI state accordingly.
    PlaybackStopped,

    /// A MIDI message that should be routed from Groove to outside.
    MidiToExternal(MidiChannel, MidiMessage),

    /// The engine has loaded a new project with the supplied filename and
    /// optional title.
    ProjectLoaded(String, Option<String>),
}
impl MessageBounds for GrooveEvent {}

/// A [ControlLink] represents an automation. The source_uid entity must
/// implement IsController. The target_uid entity must implement Controllable.
/// The point_index determines which of the target entity's controllable fields
/// that this link controls.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ControlLink {
    pub source_uid: usize,
    pub target_uid: usize,
    pub control_index: usize,
}

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
