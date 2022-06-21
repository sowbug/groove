use crate::clock::Clock;
use crate::midi::MIDIMessage;
use crate::midi::MIDIMessageType;
use crate::midi::MIDIReceiverTrait;

pub struct Oscillator {
    frequency: f32,
}

impl Oscillator {
    pub fn new() -> Oscillator {
        Oscillator { frequency: 0. }
    }

    pub fn get_sample(&self, clock: &Clock) -> f32 {
        (clock.sample_clock * self.frequency * 2.0 * std::f32::consts::PI / clock.sample_rate).sin()
    }
}

impl MIDIReceiverTrait for Oscillator {
    fn handle_midi(&mut self, midi_message: MIDIMessage) -> bool {
        match midi_message.status {
            MIDIMessageType::NoteOn => {
                self.frequency = midi_message.to_frequency();
                ()
            }
            MIDIMessageType::NoteOff => {
                println!("note off");
                self.frequency = 0.;
                ()
            }
        }
        true
    }
}
