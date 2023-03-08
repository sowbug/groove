// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::{
    midi::{HandlesMidi, MidiChannel, MidiMessage},
    traits::{Generates, IsStereoSampleVoice, Resets, StoresVoices, Ticks},
    StereoSample,
};

/// [Synthesizer] provides the smallest possible functional core of a
/// synthesizer built around [StoresVoices]. A full
/// [IsInstrument](crate::traits::IsInstrument) will typically compose itself
/// from a concrete [Synthesizer], providing implementations of
/// [HasUid](crate::traits::HasUid) and
/// [Controllable](crate::traits::Controllable) as needed.
///
/// [Synthesizer] exists so that this crate's synthesizer voices can be used in
/// other projects without needing all the other Groove crates.
#[derive(Debug)]
pub struct Synthesizer<V: IsStereoSampleVoice> {
    sample_rate: usize,

    voice_store: Box<dyn StoresVoices<Voice = V>>,

    /// Ranges from -1.0..=1.0. Applies to all notes.
    pitch_bend: f32,

    /// Ranges from 0..127. Applies to all notes.
    channel_aftertouch: u8,

    /// TODO: bipolar modal, -1.0 = all left, 1.0 = all right, 0.0 = center
    pan: f32,
}
impl<V: IsStereoSampleVoice> Generates<StereoSample> for Synthesizer<V> {
    fn value(&self) -> StereoSample {
        self.voice_store.value()
    }

    fn batch_values(&mut self, values: &mut [StereoSample]) {
        self.voice_store.batch_values(values);
    }
}
impl<V: IsStereoSampleVoice> Resets for Synthesizer<V> {
    fn reset(&mut self, sample_rate: usize) {
        self.sample_rate = sample_rate;
        self.voice_store.reset(sample_rate);
    }
}
impl<V: IsStereoSampleVoice> Ticks for Synthesizer<V> {
    fn tick(&mut self, tick_count: usize) {
        self.voice_store.tick(tick_count);
    }
}
impl<V: IsStereoSampleVoice> Synthesizer<V> {
    pub fn new_with(sample_rate: usize, voice_store: Box<dyn StoresVoices<Voice = V>>) -> Self {
        Self {
            sample_rate,
            voice_store,
            pitch_bend: Default::default(),
            channel_aftertouch: Default::default(),
            pan: Default::default(),
        }
    }
    pub fn set_pitch_bend(&mut self, pitch_bend: f32) {
        self.pitch_bend = pitch_bend;
    }

    pub fn set_channel_aftertouch(&mut self, channel_aftertouch: u8) {
        self.channel_aftertouch = channel_aftertouch;
    }

    pub fn pan(&self) -> f32 {
        self.pan
    }

    pub fn set_pan(&mut self, pan: f32) {
        self.pan = pan;
        self.voice_store.set_pan(pan);
    }
}
impl<V: IsStereoSampleVoice> HandlesMidi for Synthesizer<V> {
    fn handle_midi_message(
        &mut self,
        message: &MidiMessage,
    ) -> Option<Vec<(MidiChannel, MidiMessage)>> {
        match message {
            MidiMessage::NoteOff { key, vel } => {
                if let Ok(voice) = self.voice_store.get_voice(key) {
                    voice.note_off(vel.as_int());
                }
            }
            MidiMessage::NoteOn { key, vel } => {
                if let Ok(voice) = self.voice_store.get_voice(key) {
                    voice.note_on(key.as_int(), vel.as_int());
                }
            }
            MidiMessage::Aftertouch { key, vel } => {
                if let Ok(voice) = self.voice_store.get_voice(key) {
                    voice.aftertouch(vel.as_int());
                }
            }
            #[allow(unused_variables)]
            MidiMessage::Controller { controller, value } => todo!(),
            #[allow(unused_variables)]
            MidiMessage::ProgramChange { program } => todo!(),
            #[allow(unused_variables)]
            MidiMessage::ChannelAftertouch { vel } => todo!(),
            #[allow(unused_variables)]
            MidiMessage::PitchBend { bend } => self.set_pitch_bend(bend.as_f32()),
        }
        None
    }
}
