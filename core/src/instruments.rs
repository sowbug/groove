// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::{
    midi::{HandlesMidi, MidiChannel, MidiMessage},
    time::SampleRate,
    traits::{Configurable, Generates, IsStereoSampleVoice, StoresVoices, Ticks},
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
#[derive(Debug, Default)]
pub struct Synthesizer<V: IsStereoSampleVoice> {
    sample_rate: SampleRate,

    voice_store: Option<Box<dyn StoresVoices<Voice = V>>>,

    /// Ranges from -1.0..=1.0. Applies to all notes.
    pitch_bend: f32,

    /// Ranges from 0..127. Applies to all notes.
    channel_aftertouch: u8,

    gain: Normal,

    pan: BipolarNormal,

    ticks_since_last_midi_input: usize,
}
impl<V: IsStereoSampleVoice> Generates<StereoSample> for Synthesizer<V> {
    fn value(&self) -> StereoSample {
        if let Some(vs) = &self.voice_store {
            vs.value()
        } else {
            StereoSample::default()
        }
    }

    fn generate_batch_values(&mut self, values: &mut [StereoSample]) {
        if let Some(vs) = self.voice_store.as_mut() {
            vs.generate_batch_values(values);
        } else {
            for v in values {
                *v = StereoSample::default()
            }
        }
    }
}
impl<V: IsStereoSampleVoice> Configurable for Synthesizer<V> {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        if let Some(vs) = self.voice_store.as_mut() {
            vs.update_sample_rate(sample_rate);
        }
    }
}
impl<V: IsStereoSampleVoice> Ticks for Synthesizer<V> {
    fn tick(&mut self, tick_count: usize) {
        if let Some(vs) = self.voice_store.as_mut() {
            vs.tick(tick_count);
        }
        self.ticks_since_last_midi_input += tick_count;
    }
}
impl<V: IsStereoSampleVoice> Synthesizer<V> {
    pub fn new_with(voice_store: Box<dyn StoresVoices<Voice = V>>) -> Self {
        Self {
            voice_store: Some(voice_store),
            sample_rate: Default::default(),
            pitch_bend: Default::default(),
            channel_aftertouch: Default::default(),
            gain: Default::default(),
            pan: Default::default(),
            ticks_since_last_midi_input: Default::default(),
        }
    }

    pub fn voices<'a>(&'a self) -> Box<dyn Iterator<Item = &Box<V>> + 'a> {
        if let Some(vs) = self.voice_store.as_ref() {
            vs.voices()
        } else {
            panic!()
        }
    }

    pub fn voices_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &mut Box<V>> + 'a> {
        if let Some(vs) = self.voice_store.as_mut() {
            vs.voices_mut()
        } else {
            eprintln!("TODO: this is horribly lame");
            Box::new(std::iter::empty())
        }
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

    pub fn set_gain(&mut self, gain: Normal) {
        self.gain = gain;
    }

    pub fn pan(&self) -> BipolarNormal {
        self.pan
    }

    pub fn set_pan(&mut self, pan: BipolarNormal) {
        self.pan = pan;
    }

    pub fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    pub fn is_midi_recently_active(&self) -> bool {
        // Last quarter-second
        self.ticks_since_last_midi_input < self.sample_rate().value() / 4
    }
}
impl<V: IsStereoSampleVoice> HandlesMidi for Synthesizer<V> {
    fn handle_midi_message(
        &mut self,
        _: MidiChannel,
        message: MidiMessage,
        _messages_fn: &mut dyn FnMut(MidiChannel, MidiMessage),
    ) {
        if let Some(vs) = self.voice_store.as_mut() {
            match message {
                MidiMessage::NoteOff { key, vel } => {
                    if let Ok(voice) = vs.get_voice(&key) {
                        voice.note_off(vel.as_int());
                    }
                }
                MidiMessage::NoteOn { key, vel } => {
                    if let Ok(voice) = vs.get_voice(&key) {
                        voice.note_on(key.as_int(), vel.as_int());
                    }
                }
                MidiMessage::Aftertouch { key, vel } => {
                    if let Ok(voice) = vs.get_voice(&key) {
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

            self.ticks_since_last_midi_input = Default::default();
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        instruments::Synthesizer,
        midi::{new_note_on, HandlesMidi, MidiChannel, MidiMessage},
        time::SampleRate,
        traits::{Configurable, Generates, Ticks},
        voices::{tests::TestVoice, VoiceCount, VoiceStore},
        StereoSample,
    };

    #[derive(Debug)]
    pub struct TestSynthesizer {
        inner_synth: Synthesizer<TestVoice>,
    }
    impl HandlesMidi for TestSynthesizer {
        fn handle_midi_message(
            &mut self,
            channel: MidiChannel,
            message: MidiMessage,
            messages_fn: &mut dyn FnMut(MidiChannel, MidiMessage),
        ) {
            self.inner_synth
                .handle_midi_message(channel, message, messages_fn)
        }
    }
    impl Generates<StereoSample> for TestSynthesizer {
        fn value(&self) -> StereoSample {
            self.inner_synth.value()
        }

        fn generate_batch_values(&mut self, values: &mut [StereoSample]) {
            self.inner_synth.generate_batch_values(values)
        }
    }
    impl Configurable for TestSynthesizer {
        fn update_sample_rate(&mut self, sample_rate: SampleRate) {
            self.inner_synth.update_sample_rate(sample_rate);
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
                    VoiceStore::<TestVoice>::new_with_voice(VoiceCount::from(4), || {
                        TestVoice::new()
                    }),
                )),
            }
        }
    }

    #[test]
    fn mainline_test_synthesizer() {
        let mut s = TestSynthesizer::default();
        s.handle_midi_message(MidiChannel(12), new_note_on(100, 99), &mut |_, _| {});

        // Get a few samples because the oscillator correctly starts at zero.
        let mut samples = [StereoSample::default(); 5];
        s.generate_batch_values(&mut samples);
        assert!(samples
            .iter()
            .any(|s| { s != &StereoSample::from(StereoSample::SILENCE) }));
    }
}
