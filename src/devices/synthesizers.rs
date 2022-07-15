use std::{cell::RefCell, collections::HashMap, rc::Rc};

use num_traits::FromPrimitive;

use crate::{
    common::{MidiMessage, MidiMessageType},
    preset::welsh::WelshSynthPreset,
    primitives::clock::Clock,
};

use super::{instruments::SuperVoice, traits::DeviceTrait};

#[derive(Default)]
pub struct SuperSynth {
    sample_rate: u32,
    preset: WelshSynthPreset,
    note_to_voice: HashMap<u8, Rc<RefCell<SuperVoice>>>,
    current_value: f32,
}

impl SuperSynth {
    pub fn new(sample_rate: u32, preset: WelshSynthPreset) -> Self {
        Self {
            sample_rate,
            preset,
            //voices: Vec::new(),
            note_to_voice: HashMap::new(),
            ..Default::default()
        }
    }

    fn voice_for_note(&mut self, note: u8) -> Rc<RefCell<SuperVoice>> {
        let opt = self.note_to_voice.get(&note);
        if opt.is_some() {
            let voice = opt.unwrap().clone();
            voice
        } else {
            let voice = Rc::new(RefCell::new(SuperVoice::new(
                self.sample_rate,
                &self.preset,
            )));
            self.note_to_voice.insert(note, voice.clone());
            voice
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
                let note = message.data1;
                let voice = self.voice_for_note(note);
                voice.borrow_mut().handle_midi_message(message, clock);
            }
            MidiMessageType::NoteOff => {
                let note = message.data1;
                let voice = self.voice_for_note(note);
                voice.borrow_mut().handle_midi_message(message, clock);

                // TODO: this is incorrect because it kills voices before release is complete
                self.note_to_voice.remove(&note);
            }
            MidiMessageType::ProgramChange => {
                self.preset = SuperSynth::get_general_midi_preset(
                    FromPrimitive::from_u8(message.data1).unwrap(),
                );
            }
        }
    }

    fn tick(&mut self, clock: &Clock) -> bool {
        let mut done = true;
        self.current_value = 0.0;
        for (_note, voice) in self.note_to_voice.iter_mut() {
            self.current_value += voice.borrow_mut().process(clock.seconds);
            done = done && !voice.borrow().is_playing();
        }
        if !self.note_to_voice.is_empty() {
            self.current_value /= self.note_to_voice.len() as f32;
        }
        done
    }

    fn get_audio_sample(&self) -> f32 {
        self.current_value
    }
}
