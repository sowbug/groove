// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! The [messages](crate::messages) module defines the app's Iced messages.

use groove_core::{
    midi::{MidiChannel, MidiMessage},
    traits::MessageBounds,
    StereoSample,
};
use groove_entities::EntityMessage;
use std::fmt::Debug;

#[derive(Clone, Debug)]
pub enum GrooveInput {
    EntityMessage(usize, EntityMessage),

    /// A MIDI message that has arrived from outside Groove, typically from
    /// MidiInputHandler.
    MidiFromExternal(MidiChannel, MidiMessage),

    /// Connect an IsController to a Controllable's control point. First
    /// argument is controller uid, second is controllable uid, third is
    /// controllable's control index.
    ConnectController(usize, usize, usize),
}
impl MessageBounds for GrooveInput {}

#[derive(Clone, Debug)]
pub enum GrooveEvent {
    EntityMessage(usize, EntityMessage),

    /// An audio sample for the current time slice. Intended to be sent in
    /// response to a downstream Tick, and consumed by the application.
    AudioOutput(StereoSample),

    /// Each device's most recent audio info. (uid, sample). If a device is
    /// skipped, it means that its output hasn't changed.
    EntityAudioOutput(Vec<(usize, StereoSample)>),

    /// If sent, then the Orchestrator performance is done. Intended to be sent
    /// in response to a downstream Tick, and consumed by the application.
    OutputComplete,

    /// A MIDI message that should be routed from Groove to outside.
    MidiToExternal(MidiChannel, MidiMessage),

    /// The engine has loaded a new project with the supplied filename and
    /// optional title.
    ProjectLoaded(String, Option<String>),
}
impl MessageBounds for GrooveEvent {}

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
