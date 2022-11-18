use crate::midi::MidiChannel;
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

#[cfg(test)]
pub mod tests {

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
}
