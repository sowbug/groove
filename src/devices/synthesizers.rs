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
    //voices: Vec<Rc<RefCell<SuperVoice>>>,
    //    available_voices: Vec<Rc<RefCell<SuperVoice>>>,
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

impl<'a> DeviceTrait for SuperSynth {
    fn sources_audio(&self) -> bool {
        true
    }

    fn sinks_midi(&self) -> bool {
        true
    }

    // fn handle_midi_message_OLD(&mut self, message: &MidiMessage, clock: &Clock) {
    //     match message.status {
    //         // TODO: this has lots of bugs. The model is wrong. We don't know when a voice stops playing,
    //         // so we'll never take it out of the hash map. I think right now it ends up creating max 127 voices,
    //         // but it's all bad.
    //         MidiMessageType::NoteOn => {
    //             let note = message.data1;
    //             let i_opt = self.note_to_voice.get(&note);
    //             if i_opt.is_some() {
    //                 let i = i_opt.unwrap();
    //                 self.voices[*i].handle_midi_message(message, clock);
    //                 return;
    //             }
    //             for i in 0..self.voices.len() {
    //                 if !self.voices[i].is_playing() {
    //                     self.note_to_voice.insert(note, i);
    //                     self.voices[i].handle_midi_message(message, clock);
    //                     return;
    //                 }
    //             }
    //             self.voices
    //                 .push(SuperVoice::new(self.sample_rate, &self.preset));
    //             let i = self.voices.len() - 1;
    //             self.note_to_voice.insert(note, i);
    //             self.voices[i].handle_midi_message(message, clock);
    //         }
    //         MidiMessageType::NoteOff => {
    //             let note = message.data1;
    //             let i_opt = self.note_to_voice.get(&note);
    //             if i_opt.is_some() {
    //                 let i = i_opt.unwrap();
    //                 self.voices[*i].handle_midi_message(message, clock);
    //                 self.note_to_voice.remove(&note);
    //             }
    //         }
    //         MidiMessageType::ProgramChange => {
    //             let program = match message.data1 {
    //                 42 => GeneralMidiProgram::Cello,
    //                 52 => GeneralMidiProgram::ChoirAahs,
    //                 _ => {
    //                     panic!("no patch");
    //                 }
    //             };
    //             self.preset = SuperSynth::get_general_midi_preset(program);
    //             // start changing to new voices
    //         }
    //     }
    // }

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
        self.current_value /= self.note_to_voice.len() as f32;
        done
    }

    fn get_audio_sample(&self) -> f32 {
        self.current_value
    }
}
