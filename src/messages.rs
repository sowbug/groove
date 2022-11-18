use crate::{midi::MidiChannel, traits::Message};
use midly::MidiMessage;

#[derive(Clone, Debug, Default)]
pub(crate) enum GrooveMessage {
    #[default]
    Nop,
    Tick,
    ControlF32(usize, f32), // Sent by controller, (self.uid, new value)
    UpdateF32(usize, f32),  // sent by system, (param_id, new value)
    Midi(MidiChannel, MidiMessage),
}
impl Message for GrooveMessage {}

#[cfg(test)]
pub mod tests {
    use crate::traits::Message;

    #[derive(Clone, Debug, Default)]
    pub enum TestMessage {
        #[default]
        Nothing,
        #[allow(dead_code)]
        Something,
        Tick,
        ControlF32(usize, f32),
        UpdateF32(usize, f32),
    }
    impl Message for TestMessage {}
}
