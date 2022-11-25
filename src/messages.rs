use crate::{gui::PatternMessage, midi::MidiChannel};
use midly::MidiMessage;

pub trait MessageBounds: Clone + std::fmt::Debug + Default + 'static {} // TODO: that 'static scares me

// How do you decide what's a GrooveMessage and what's an EntityMessage? Some
// rules (I'll add as I go):
//
// - If it knows about UIDs, it's a GrooveMessage. EntityMessages get sent to
//   the right entity via GrooveMessage's UID, so EntityMessages don't need to
//   care about them.

#[derive(Clone, Debug, Default)]
pub enum GrooveMessage {
    /// "no operation" $EA, exists only as a default. Nobody should do anything
    /// in response to this message; in fact, it's probably OK to panic.
    #[default]
    Nop,

    /// It's time to do a slice of work. Since update() includes a Clock
    /// parameter, Tick is just a message without time information. We assume
    /// that anyone getting a Tick got it via update(), directly or indirectly,
    /// so it's the responsibility of the message handler to pass time
    /// information when needed.
    Tick,

    /// A wrapped EntityMessage that contains the uid of the receipient in
    /// addition to the EntityMessage.
    EntityMessage(usize, EntityMessage),

    /// A MIDI message sent to a channel. There is an identical message type in
    /// EntityMessage. This one is for MIDI messages coming from outside Groove,
    /// for example from a MIDI hardware instrument.  
    Midi(MidiChannel, MidiMessage),

}
impl MessageBounds for GrooveMessage {}

#[derive(Clone, Debug, Default)]
pub enum EntityMessage {
    /// "no operation" $EA, exists only as a default. Nobody should do anything
    /// in response to this message; in fact, it's probably OK to panic.
    #[default]
    Nop,

    /// It's time to do a slice of work. Since update() includes a Clock
    /// parameter, Tick is just a message without time information. We assume
    /// that anyone getting a Tick got it via update(), directly or indirectly,
    /// so it's the responsibility of the message handler to pass time
    /// information when needed.
    Tick,

    /// A MIDI message sent to a channel. In most cases, MidiChannel is
    /// redundant, as the sender of a message generally won't route a message to
    /// someone not listening on the channel.
    Midi(MidiChannel, MidiMessage),

    /// (new controller_value)
    ///
    /// Sent by controller. Indicates "My value has changed to \[value\], and
    /// I'd like subscribers to know about that." The recipient will typically
    /// turn this into one or more UpdateF32 messages, each going to a target
    /// controlled by the controller.
    ControlF32(f32),

    /// (param_id, new value)
    ///
    /// Sent by the system to targets of controllers. They should respond by
    /// mapping the param_id to one of their internal controllable parameters,
    /// and then set it to the updated f32 value.
    ///
    /// In the future we'll add richer types for the new_value parameter, but
    /// for now most parameter updates are representable by a plain old float.
    UpdateF32(usize, f32),

    /// A series of UpdateF32-like messages that are (hopefully) placeholders
    /// until I figure out how to send a Msg(_, _) to a thing that wants a
    /// Msg(_). If that isn't a Rust thing, I think I can ask someone up the
    /// chain to do it for me.
    /// 
    /// For sanity, please make sure the ParamN corresponds to the
    /// ___ControlParams enum.
    UpdateParam0F32(f32),
    UpdateParam0String(String),
    UpdateParam0U8(u8),
    UpdateParam1F32(f32),
    UpdateParam1U8(u8),

    /// Enable or disable the recipient.
    Enable(bool),

    /// Wrapper for PatternMessages.
    PatternMessage(usize, PatternMessage),

    // Temp things
    MutePressed(bool),
    EnablePressed(bool),
}
impl MessageBounds for EntityMessage {}

#[cfg(test)]
pub mod tests {
    use super::{EntityMessage, MessageBounds};

    #[derive(Clone, Debug, Default)]
    pub enum TestMessage {
        #[default]
        Nop, // "no-op"

        /// It's time to do a slice of work. Since update() includes a Clock
        /// parameter, Tick is just a message without time information. We assume
        /// that anyone getting a Tick got it via update(), directly or indirectly,
        /// so it's the responsibility of the message handler to pass time
        /// information when needed.
        Tick,

        /// A wrapped EntityMessage that contains the uid of the receipient in
        /// addition to the EntityMessage.
        EntityMessage(usize, EntityMessage),
    }
    impl MessageBounds for TestMessage {}
}
