use crate::{midi::MidiChannel, gui::PatternMessage};
use midly::MidiMessage;

pub trait MessageBounds: Clone + std::fmt::Debug + Default + 'static {} // TODO: that 'static scares me

#[derive(Clone, Debug, Default)]
pub enum GrooveMessage {
    #[default]
    Nop, // "no operation" $EA
    Tick,
    ControlF32(usize, f32), // Sent by controller, (self.uid, new value)
    UpdateF32(usize, f32),  // sent by system, (param_id, new value)
    Midi(MidiChannel, MidiMessage),
    Enable(bool),
    PatternMessage(usize, PatternMessage),
}
impl MessageBounds for GrooveMessage {}

#[cfg(test)]
pub mod tests {
    use super::MessageBounds;
    use crate::midi::MidiChannel;
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
    }
    impl MessageBounds for TestMessage {}
}
