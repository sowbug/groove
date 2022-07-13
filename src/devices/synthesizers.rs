use std::collections::HashMap;

use crate::{
    common::{MidiMessage, MidiMessageType},
    primitives::clock::Clock, preset::welsh::WelshSynthPreset,
};

use super::{
    instruments::{SuperVoice},
    traits::DeviceTrait, presets_ss::GeneralMidiProgram,
};

#[derive(Default)]
pub struct SuperSynth {
    sample_rate: u32,
    preset: WelshSynthPreset,
    voices: Vec<SuperVoice>,
    note_to_voice: HashMap<u8, usize>,
    current_value: f32,
}

impl SuperSynth {
    pub fn new(sample_rate: u32, preset: WelshSynthPreset) -> Self {
        Self {
            sample_rate,
            preset,
            voices: Vec::new(),
            note_to_voice: HashMap::new(),
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
            // TODO: this has lots of bugs. The model is wrong. We don't know when a voice stops playing,
            // so we'll never take it out of the hash map. I think right now it ends up creating max 127 voices,
            // but it's all bad.
            MidiMessageType::NoteOn => {
                let note = message.data1;
                let i_opt = self.note_to_voice.get(&note);
                if i_opt.is_some() {
                    let i = i_opt.unwrap();
                    self.voices[*i].handle_midi_message(message, clock);
                    return;
                }
                for i in 0..self.voices.len() {
                    if !self.voices[i].is_playing() {
                        self.note_to_voice.insert(note, i);
                        self.voices[i].handle_midi_message(message, clock);
                        return;
                    }
                }
                self.voices
                    .push(SuperVoice::new(self.sample_rate, &self.preset));
                    let i = self.voices.len() - 1;
                self.note_to_voice
                    .insert(note, i);
                self.voices[i].handle_midi_message(message, clock);
            }
            MidiMessageType::NoteOff => {
                let note = message.data1;
                let i_opt = self.note_to_voice.get(&note);
                if i_opt.is_some() {
                    let i = i_opt.unwrap();
                    self.voices[*i].handle_midi_message(message, clock);
                    self.note_to_voice.remove(&note);
                }
            }
            MidiMessageType::ProgramChange => {
                let program = match message.data1 {
                    42 => GeneralMidiProgram::Cello,
                    52 => GeneralMidiProgram::ChoirAahs,
                    _ => {
                        panic!("no patch");
                    }
                };
                self.preset = SuperSynth::get_general_midi_preset(program);
                // start changing to new voices
            }
        }
    }

    fn tick(&mut self, clock: &Clock) -> bool {
        let mut done = true;
        self.current_value = 0.0;
        for voice in self.voices.iter_mut() {
            self.current_value += voice.process(clock.seconds);
            done = done && !voice.is_playing();
        }
        done
    }

    fn get_audio_sample(&self) -> f32 {
        self.current_value
    }
}
