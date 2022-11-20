use crate::{midi::MidiChannel, traits::MessageBounds};
use midly::MidiMessage;

#[derive(Clone, Debug, Default)]
pub enum GrooveMessage {
    #[default]
    Nop, // "no operation" $EA
    Tick,
    ControlF32(usize, f32), // Sent by controller, (self.uid, new value)
    UpdateF32(usize, f32),  // sent by system, (param_id, new value)
    Midi(MidiChannel, MidiMessage),
    Enable(bool),
}
impl MessageBounds for GrooveMessage {}

#[cfg(test)]
pub mod tests {
    use midly::MidiMessage;

    use crate::{midi::MidiChannel, traits::MessageBounds};

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
