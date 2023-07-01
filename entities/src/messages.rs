// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    control::ControlValue,
    midi::{MidiChannel, MidiMessage},
    traits::MessageBounds,
};
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
    Control(ControlValue),

    /// Sent by system to every entity that subscribes to a control.
    #[deprecated]
    HandleControl(usize, ControlValue),
}
impl MessageBounds for EntityMessage {}
