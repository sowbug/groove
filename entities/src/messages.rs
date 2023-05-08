// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::controllers::PatternMessage;
use groove_core::{
    midi::{MidiChannel, MidiMessage},
    traits::MessageBounds,
};
#[cfg(toy_controller_disabled)]
use groove_toys::MessageMaker;
use std::fmt::Debug;

/// An [EntityMessage] describes how external components, such as an application
/// GUI, communicate with [Entities](Entity). Some variants, such as `Midi` and
/// `ControlF32`, go in the other direction; Entities send them to the rest of
/// the system.
#[derive(Clone, Debug)]
pub enum EntityMessage {
    /// A MIDI message sent to a channel. In most cases, MidiChannel is
    /// redundant, as the sender of a message generally won't route a message to
    /// someone not listening on the channel.
    Midi(MidiChannel, MidiMessage),

    /// (new controller_value)
    ///
    /// Sent by controller. Handled by system. Indicates "My value has changed
    /// to \[value\], and I'd like subscribers to know about that." The
    /// recipient will typically fan this out to multiple targets controlled by
    /// the controller.
    ControlF32(f32),

    /// Sent by system to every entity that subscribes to a control.
    HandleControlF32(usize, f32),

    /// Wrapper for PatternMessages.
    PatternMessage(usize, PatternMessage),

    PickListSelected(String),

    // GUI things.
    ExpandPressed,
    CollapsePressed,
    EnablePressed(bool),
}
impl MessageBounds for EntityMessage {}

// core_entities must know about core_toys, because it creates the monolithic
// matching blocks that contain all entities. So it's not too weird for this
// crate to also include a mapper of abstract messages to concrete
// EntityMessages. If this becomes too much of an architectural sore thumb, it's
// OK for everyone using ToyController to create their own ToyMessageMaker.
#[cfg(toy_controller_disabled)]
#[derive(Debug)]
pub struct ToyMessageMaker {}
#[cfg(toy_controller_disabled)]
impl MessageMaker for ToyMessageMaker {
    type Message = EntityMessage;

    fn midi(&self, channel: MidiChannel, message: MidiMessage) -> Self::Message {
        EntityMessage::Midi(channel, message)
    }
}
