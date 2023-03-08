// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::VoiceStore;
use groove_core::{
    generators::{AdsrParams, Envelope, Oscillator},
    midi::{note_to_frequency, HandlesMidi, MidiChannel, MidiMessage},
    traits::{
        Generates, GeneratesEnvelope, IsInstrument, IsStereoSampleVoice, IsVoice, PlaysNotes,
        Resets, StoresVoices, Ticks,
    },
    Normal, ParameterType, StereoSample,
};
use groove_macros::{Control, Uid};
use std::str::FromStr;
use strum_macros::{Display, EnumString, FromRepr};

/// A generic [IsInstrument] that takes a [StoresVoices] and makes a synthesizer
/// instrument out of it.
#[derive(Control, Debug, Uid)]
pub struct Synthesizer<V: IsStereoSampleVoice> {
    uid: usize,
    sample_rate: usize,

    voice_store: Box<dyn StoresVoices<Voice = V>>,

    /// Ranges from -1.0..=1.0. Applies to all notes.
    #[controllable]
    pitch_bend: f32,

    /// Ranges from 0..127. Applies to all notes.
    #[controllable]
    channel_aftertouch: u8,

    /// TODO: bipolar modal, -1.0 = all left, 1.0 = all right, 0.0 = center
    #[controllable]
    pan: f32,
}
impl<V: IsStereoSampleVoice> IsInstrument for Synthesizer<V> {}
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
            uid: Default::default(),
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

    pub fn set_control_pitch_bend(&mut self, pitch_bend: groove_core::control::F32ControlValue) {
        self.set_pitch_bend(pitch_bend.0);
    }

    pub fn set_channel_aftertouch(&mut self, channel_aftertouch: u8) {
        self.channel_aftertouch = channel_aftertouch;
    }

    pub fn set_control_channel_aftertouch(
        &mut self,
        channel_aftertouch: groove_core::control::F32ControlValue,
    ) {
        // TODO - will this ever be needed? Do we need to introduce a whole new
        // schema to describe non-f32 control parameters?
        //
        // For now this is silly code to allow it to compile
        self.set_channel_aftertouch((channel_aftertouch.0 * 63.0 + 64.0) as u8);
        todo!()
    }

    pub fn pan(&self) -> f32 {
        self.pan
    }

    pub fn set_pan(&mut self, pan: f32) {
        self.pan = pan;
        self.voice_store.set_pan(pan);
    }

    pub fn set_control_pan(&mut self, value: groove_core::control::F32ControlValue) {
        // TODO: more toil. Let me say this is a bipolar normal
        self.set_pan(value.0 * 2.0 - 1.0);
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

#[derive(Debug)]
pub struct SimpleVoice {
    sample_rate: usize,
    oscillator: Oscillator,
    envelope: Envelope,

    sample: StereoSample,

    note_on_key: u8,
    note_on_velocity: u8,
    steal_is_underway: bool,
}
impl IsStereoSampleVoice for SimpleVoice {}
impl IsVoice<StereoSample> for SimpleVoice {}
impl PlaysNotes for SimpleVoice {
    fn is_playing(&self) -> bool {
        !self.envelope.is_idle()
    }

    fn note_on(&mut self, key: u8, velocity: u8) {
        if self.is_playing() {
            self.steal_is_underway = true;
            self.note_on_key = key;
            self.note_on_velocity = velocity;
        } else {
            self.set_frequency_hz(note_to_frequency(key));
            self.envelope.trigger_attack();
        }
    }

    fn aftertouch(&mut self, _velocity: u8) {
        todo!()
    }

    fn note_off(&mut self, _velocity: u8) {
        self.envelope.trigger_release();
    }

    fn set_pan(&mut self, _value: f32) {
        // We don't handle this.
    }
}
impl Generates<StereoSample> for SimpleVoice {
    fn value(&self) -> StereoSample {
        self.sample
    }

    fn batch_values(&mut self, values: &mut [StereoSample]) {
        for sample in values {
            self.tick(1);
            *sample = self.value();
        }
    }
}
impl Resets for SimpleVoice {
    fn reset(&mut self, sample_rate: usize) {
        self.sample_rate = sample_rate;
        self.oscillator.reset(sample_rate);
        self.envelope.reset(sample_rate);
    }
}
impl Ticks for SimpleVoice {
    fn tick(&mut self, tick_count: usize) {
        for _ in 0..tick_count {
            let was_playing = self.is_playing();
            self.oscillator.tick(1);
            self.envelope.tick(1);
            if was_playing && !self.is_playing() {
                if self.steal_is_underway {
                    self.steal_is_underway = false;
                    self.note_on(self.note_on_key, self.note_on_velocity);
                }
            }
            self.sample = StereoSample::from(self.oscillator.value() * self.envelope.value());
        }
    }
}

impl SimpleVoice {
    pub fn new_with(sample_rate: usize) -> Self {
        Self {
            sample_rate,
            oscillator: Oscillator::new_with(sample_rate),
            envelope: Envelope::new_with(
                sample_rate,
                AdsrParams::new_with(0.0, 0.0, Normal::maximum(), 0.0),
            ),
            sample: Default::default(),
            note_on_key: Default::default(),
            note_on_velocity: Default::default(),
            steal_is_underway: Default::default(),
        }
    }
    fn set_frequency_hz(&mut self, frequency_hz: ParameterType) {
        self.oscillator.set_frequency(frequency_hz);
    }
}

#[derive(Control, Debug, Uid)]
pub struct SimpleSynthesizer {
    uid: usize,
    inner_synth: Synthesizer<SimpleVoice>,
}
impl IsInstrument for SimpleSynthesizer {}
impl HandlesMidi for SimpleSynthesizer {
    fn handle_midi_message(
        &mut self,
        message: &MidiMessage,
    ) -> Option<Vec<(MidiChannel, MidiMessage)>> {
        self.inner_synth.handle_midi_message(message)
    }
}
impl Generates<StereoSample> for SimpleSynthesizer {
    fn value(&self) -> StereoSample {
        self.inner_synth.value()
    }

    fn batch_values(&mut self, values: &mut [StereoSample]) {
        self.inner_synth.batch_values(values)
    }
}
impl Resets for SimpleSynthesizer {
    fn reset(&mut self, sample_rate: usize) {
        self.inner_synth.reset(sample_rate);
    }
}
impl Ticks for SimpleSynthesizer {
    fn tick(&mut self, tick_count: usize) {
        self.inner_synth.tick(tick_count);
    }
}
impl SimpleSynthesizer {
    pub fn new(sample_rate: usize) -> Self {
        Self {
            uid: Default::default(),
            inner_synth: Synthesizer::<SimpleVoice>::new_with(
                sample_rate,
                Box::new(VoiceStore::<SimpleVoice>::new_with_voice(
                    sample_rate,
                    4,
                    || SimpleVoice::new_with(sample_rate),
                )),
            ),
        }
    }
    pub fn notes_playing(&self) -> usize {
        99
    }
}

#[cfg(test)]
mod tests {
    use super::SimpleVoice;
    use groove_core::ParameterType;

    impl SimpleVoice {
        pub fn debug_is_shutting_down(&self) -> bool {
            true
            // TODO bring back when this moves elsewhere
            //     self.envelope.debug_is_shutting_down()
        }

        pub fn debug_oscillator_frequency(&self) -> ParameterType {
            self.oscillator.frequency()
        }
    }
}
