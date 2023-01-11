use crate::{
    common::MonoSample,
    midi::{subscription::PatternMessage, MidiChannel},
};
use iced_audio::Normal;
use midly::MidiMessage;

pub trait MessageBounds: Clone + std::fmt::Debug + Default + Send + 'static {} // TODO: that 'static scares me

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

    /// A MIDI message that has arrived from outside Groove, typically from
    /// MidiInputHandler.
    MidiFromExternal(MidiChannel, MidiMessage),

    /// A MIDI message that should be routed from Groove to outside.
    MidiToExternal(MidiChannel, MidiMessage),

    /// An audio sample for the current time slice. Intended to be sent in
    /// response to a downstream Tick, and consumed by the application.
    AudioOutput(MonoSample),

    /// If sent, then the Orchestrator performance is done. Intended to be sent
    /// in response to a downstream Tick, and consumed by the application.
    OutputComplete,

    LoadProject(String),
    LoadedProject(String, Option<String>),
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
    /// Sent by controller. Handled by system. Indicates "My value has changed
    /// to \[value\], and I'd like subscribers to know about that." The
    /// recipient will typically fan this out to multiple targets controlled by
    /// the controller.
    ControlF32(f32),

    /// Enable or disable the recipient.
    Enable(bool),

    /// Wrapper for PatternMessages.
    PatternMessage(usize, PatternMessage),

    /// iced_audio convention.
    HSliderInt(Normal),

    PickListSelected(String),

    // GUI things.
    ExpandPressed,
    CollapsePressed,

    // Temp things
    MutePressed(bool),
    EnablePressed(bool),
}
impl MessageBounds for EntityMessage {}

#[cfg(test)]
pub mod tests {
    use super::{EntityMessage, MessageBounds};
    use crate::{common::MonoSample, midi::MidiChannel};
    use midly::MidiMessage;

    #[derive(Clone, Debug, Default)]
    pub enum TestMessage {
        #[default]
        Nop,
        Tick,
        EntityMessage(usize, EntityMessage),
        MidiFromExternal(MidiChannel, MidiMessage),
        MidiToExternal(MidiChannel, MidiMessage),
        AudioOutput(MonoSample),
        OutputComplete,
    }
    impl MessageBounds for TestMessage {}
}
