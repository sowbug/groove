use crate::{
    common::{MidiMessage, MidiMessageType},
    primitives::clock::Clock,
};

use super::{
    instruments::{MiniSynthPreset, MiniVoice},
    traits::DeviceTrait,
};

#[derive(Default)]
pub struct SuperSynth {
    sample_rate: u32,
    preset: MiniSynthPreset,
    voice: MiniVoice,
    current_value: f32, // TODO: this needs to scale up for voices
                        // TODO let's start with just one    voices: Vec<MiniVoice>,
}

impl SuperSynth {
    pub fn new(sample_rate: u32, preset: MiniSynthPreset) -> Self {
        Self {
            sample_rate,
            preset,
            voice: MiniVoice::new(sample_rate, &preset),
            ..Default::default()
        }
    }
}

impl DeviceTrait for SuperSynth {
    fn sources_audio(&self) -> bool {
        true
    }

    fn sinks_midi(&self) -> bool {
        true
    }

    fn handle_midi_message(&mut self, message: &MidiMessage, clock: &Clock) {
        match message.status {
            MidiMessageType::NoteOn => {
                // figure out which voice should do it
                self.voice.handle_midi_message(message, clock);
            }
            MidiMessageType::NoteOff => {
                // figure out which voice should do it
                self.voice.handle_midi_message(message, clock);
            }
            MidiMessageType::ProgramChange => {
                // start changing to new voices
            }
        }
    }

    fn tick(&mut self, clock: &Clock) -> bool {
        self.current_value = self.voice.process(clock.seconds);
        !self.voice.is_playing()
    }

    fn get_audio_sample(&self) -> f32 {
        self.current_value
    }
}
