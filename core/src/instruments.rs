// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::{
    midi::{HandlesMidi, MidiChannel, MidiMessage},
    traits::{Generates, IsStereoSampleVoice, Resets, StoresVoices, Ticks},
    BipolarNormal, Normal, StereoSample,
};

/// [Synthesizer] provides the smallest possible functional core of a
/// synthesizer built around [StoresVoices]. A full
/// [IsInstrument](crate::traits::IsInstrument) will typically compose itself
/// from a concrete [Synthesizer], providing implementations of
/// [HasUid](crate::traits::HasUid) and
/// [Controllable](crate::traits::Controllable) as needed.
///
/// [Synthesizer] exists so that this crate's synthesizer voices can be used in
/// other projects without needing all the other crates.
#[derive(Debug)]
pub struct Synthesizer<V: IsStereoSampleVoice> {
    sample_rate: usize,

    voice_store: Box<dyn StoresVoices<Voice = V>>,

    /// Ranges from -1.0..=1.0. Applies to all notes.
    pitch_bend: f32,

    /// Ranges from 0..127. Applies to all notes.
    channel_aftertouch: u8,

    gain: Normal,

    pan: BipolarNormal,
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
    pub fn new_with(voice_store: Box<dyn StoresVoices<Voice = V>>) -> Self {
        Self {
            voice_store,
            sample_rate: Default::default(),
            pitch_bend: Default::default(),
            channel_aftertouch: Default::default(),
            gain: Default::default(),
            pan: Default::default(),
        }
    }

    pub fn voices<'a>(&'a self) -> Box<dyn Iterator<Item = &Box<V>> + 'a> {
        self.voice_store.voices()
    }

    pub fn voices_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &mut Box<V>> + 'a> {
        self.voice_store.voices_mut()
    }

    pub fn set_pitch_bend(&mut self, pitch_bend: f32) {
        self.pitch_bend = pitch_bend;
    }

    pub fn set_channel_aftertouch(&mut self, channel_aftertouch: u8) {
        self.channel_aftertouch = channel_aftertouch;
    }

    pub fn gain(&self) -> Normal {
        self.gain
    }

    pub fn pan(&self) -> BipolarNormal {
        self.pan
    }

    pub fn sample_rate(&self) -> usize {
        self.sample_rate
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

#[cfg(test)]
mod tests {
    use crate::{
        instruments::Synthesizer,
        midi::{new_note_on, HandlesMidi, MidiChannel, MidiMessage},
        traits::{Generates, Resets, Ticks},
        voices::{tests::TestVoice, VoiceStore},
        StereoSample,
    };

    #[derive(Debug)]
    pub struct TestSynthesizer {
        inner_synth: Synthesizer<TestVoice>,
    }
    impl HandlesMidi for TestSynthesizer {
        fn handle_midi_message(
            &mut self,
            message: &MidiMessage,
        ) -> Option<Vec<(MidiChannel, MidiMessage)>> {
            self.inner_synth.handle_midi_message(message)
        }
    }
    impl Generates<StereoSample> for TestSynthesizer {
        fn value(&self) -> StereoSample {
            self.inner_synth.value()
        }

        fn batch_values(&mut self, values: &mut [StereoSample]) {
            self.inner_synth.batch_values(values)
        }
    }
    impl Resets for TestSynthesizer {
        fn reset(&mut self, sample_rate: usize) {
            self.inner_synth.reset(sample_rate);
        }
    }
    impl Ticks for TestSynthesizer {
        fn tick(&mut self, tick_count: usize) {
            self.inner_synth.tick(tick_count);
        }
    }
    impl Default for TestSynthesizer {
        fn default() -> Self {
            Self {
                inner_synth: Synthesizer::<TestVoice>::new_with(Box::new(
                    VoiceStore::<TestVoice>::new_with_voice(4, || TestVoice::new()),
                )),
            }
        }
    }

    #[test]
    fn mainline_test_synthesizer() {
        let mut s = TestSynthesizer::default();
        s.handle_midi_message(&new_note_on(100, 99));

        // Tick a few because the oscillator correctly starts at zero.
        s.tick(3);
        assert!(s.value() != StereoSample::from(StereoSample::SILENCE));
    }
}
