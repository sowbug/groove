use crate::{gui::PatternMessage, midi::MidiChannel};
use midly::MidiMessage;

pub trait MessageBounds: Clone + std::fmt::Debug + Default + 'static {} // TODO: that 'static scares me

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

    /// (controller_uid, new controller_value)
    /// 
    /// Sent by controller. Indicates "I am \[uid\] and my value has changed to
    /// \[value\]." The recipient will typically turn this into one or more
    /// UpdateF32 messages, each going to a target controlled by the controller.
    ControlF32(usize, f32), 

    /// (param_id, new value)
    /// 
    /// Sent by the system to targets of controllers. They should respond by
    /// mapping the param_id to one of their internal controllable parameters,
    /// and then set it to the updated f32 value.
    /// 
    /// In the future we'll add richer types for the new_value parameter, but
    /// for now most parameter updates are representable by a plain old float.
    UpdateF32(usize, f32),

    /// A MIDI message sent to a channel. In most cases, MidiChannel is
    /// redundant, as the sender of a message generally won't route a message to
    /// someone not listening on the channel.
    Midi(MidiChannel, MidiMessage),

    /// Enable or disable the recipient.
    Enable(bool),

    /// Wrapper for PatternMessages.
    PatternMessage(usize, PatternMessage),

    // Temp things
    MutePressed(bool),
    EnablePressed(bool),
    ArpeggiatorChanged(u8),
    BitcrusherValueChanged(u8),
    FilterCutoffChangedAsF32(f32),
    FilterCutoffChangedAsU8Percentage(u8),
    GainLevelChangedAsString(String),
    GainLevelChangedAsU8Percentage(u8),
    LimiterMinChanged(f32),
    LimiterMaxChanged(f32),
}
impl MessageBounds for GrooveMessage {}

#[cfg(test)]
pub mod tests {
    use super::MessageBounds;
    use crate::{gui::PatternMessage, midi::MidiChannel};
    use midly::MidiMessage;

    #[derive(Clone, Debug, Default)]
    pub enum TestMessage {
        #[default]
        Nop, // "no-op"
        Tick,
        ControlF32(usize, f32),
        UpdateF32(usize, f32),
        Midi(MidiChannel, MidiMessage),
        Enable(bool),
        PatternMessage(usize, PatternMessage),

        // Temp things
        MutePressed(bool),
        EnablePressed(bool),
        ArpeggiatorChanged(u8),
        BitcrusherValueChanged(u8),
        FilterCutoffChangedAsF32(f32),
        FilterCutoffChangedAsU8Percentage(u8),
        GainLevelChangedAsString(String),
        GainLevelChangedAsU8Percentage(u8),
        LimiterMinChanged(f32),
        LimiterMaxChanged(f32),
    }
    impl MessageBounds for TestMessage {}
}
