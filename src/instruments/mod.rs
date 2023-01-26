use crate::{
    common::{F32ControlValue, MonoSample},
    midi::MidiUtils,
    settings::patches::EnvelopeSettings,
    traits::{Controllable, HasUid, IsInstrument, Response, SourcesAudio, Updateable},
    AdsrEnvelope, Clock, EntityMessage, Oscillator,
};
use anyhow::{anyhow, Result};
use groove_macros::{Control, Uid};
use midly::{num::u7, MidiMessage};
use std::str::FromStr;
use std::{collections::HashMap, fmt::Debug};
use strum_macros::{Display, EnumString, FromRepr};

pub(crate) mod drumkit_sampler;
pub(crate) mod envelopes;
pub(crate) mod oscillators;
pub(crate) mod sampler;
pub(crate) mod welsh;

pub trait HandlesMidi {
    fn handle_midi_message(&mut self, message: &MidiMessage);
}

/// As an experiment, we're going to define PlaysNotes as a different interface
/// from HandlesMidi. This will give the HandlesMidi Synthesizer an opportunity
/// to manage more about the note lifecycle, including concepts like glide
/// (which I believe needs a higher-level definition of a "note" than just MIDI
/// on/off)
pub trait PlaysNotes {
    fn is_playing(&self) -> bool;
    fn set_frequency_hz(&mut self, frequency_hz: f32);
    fn attack(&mut self, velocity: u8);
    fn aftertouch(&mut self, velocity: u8);
    fn release(&mut self, velocity: u8);
}

// TODO: I didn't want VoiceStore to know anything about audio (i.e.,
// SourcesAudio), but I couldn't figure out how to return an IterMut from a
// HashMap, so I couldn't define a trait method that allowed the implementation
// to return an iterator from either a Vec or a HashMap.
pub trait VoiceStore: SourcesAudio + Send + Debug {
    type Voice;

    fn voice_count(&self) -> usize;
    fn active_voice_count(&self) -> usize;
    fn get_voice(&mut self, key: &midly::num::u7) -> Result<&mut Box<Self::Voice>>;
}

/// A synthesizer is composed of Voices. Ideally, a synth will know how to
/// construct Voices, and then handle all the MIDI events properly for them.
pub trait IsVoice: SourcesAudio + PlaysNotes + Send + Default {}

#[derive(Control, Debug, Uid)]
pub struct Synthesizer<V: IsVoice> {
    uid: usize,

    voice_store: Box<dyn VoiceStore<Voice = V>>,

    /// Ranges from -1.0..=1.0
    /// Applies to all notes
    #[controllable]
    pitch_bend: f32,

    /// Ranges from 0..127
    /// Applies to all notes
    #[controllable]
    channel_aftertouch: u8,
}
impl<V: IsVoice> IsInstrument for Synthesizer<V> {}
impl<V: IsVoice> Updateable for Synthesizer<V> {
    type Message = EntityMessage;
}
impl<V: IsVoice> SourcesAudio for Synthesizer<V> {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        self.voice_store.source_audio(clock)
    }
}

impl<V: IsVoice> Synthesizer<V> {
    fn new_with(voice_store: Box<dyn VoiceStore<Voice = V>>) -> Self {
        Self {
            uid: Default::default(),
            voice_store: voice_store,
            pitch_bend: Default::default(),
            channel_aftertouch: Default::default(),
        }
    }
    pub fn set_pitch_bend(&mut self, pitch_bend: f32) {
        self.pitch_bend = pitch_bend;
    }

    pub(crate) fn set_control_pitch_bend(&mut self, pitch_bend: F32ControlValue) {
        self.set_pitch_bend(pitch_bend.0);
    }

    pub fn set_channel_aftertouch(&mut self, channel_aftertouch: u8) {
        self.channel_aftertouch = channel_aftertouch;
    }

    pub(crate) fn set_control_channel_aftertouch(&mut self, channel_aftertouch: F32ControlValue) {
        // TODO - will this ever be needed? Do we need to introduce a whole new
        // schema to describe non-f32 control parameters?
        //
        // For now this is silly code to allow it to compile
        self.set_channel_aftertouch((channel_aftertouch.0 * 63.0 + 64.0) as u8);
        todo!()
    }
}
impl<V: IsVoice> HandlesMidi for Synthesizer<V> {
    fn handle_midi_message(&mut self, message: &MidiMessage) {
        match message {
            MidiMessage::NoteOff { key, vel } => {
                if let Ok(voice) = self.voice_store.get_voice(key) {
                    voice.release(vel.as_int());
                }
            }
            MidiMessage::NoteOn { key, vel } => {
                if let Ok(voice) = self.voice_store.get_voice(key) {
                    voice.set_frequency_hz(MidiUtils::note_to_frequency(key.as_int()));
                    voice.attack(vel.as_int());
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
    }
}

#[derive(Debug, Default)]
pub struct SimpleVoice {
    oscillator: Oscillator,
    envelope: AdsrEnvelope,

    is_playing: bool,
    attack_is_pending: bool,
    attack_velocity: u8,
    release_is_pending: bool,
    release_velocity: u8,
    aftertouch_is_pending: bool,
    aftertouch_velocity: u8,
}
impl IsVoice for SimpleVoice {}
impl PlaysNotes for SimpleVoice {
    fn is_playing(&self) -> bool {
        self.is_playing
    }

    fn set_frequency_hz(&mut self, frequency_hz: f32) {
        self.oscillator.set_frequency(frequency_hz);
    }

    fn attack(&mut self, velocity: u8) {
        self.attack_is_pending = true;
        self.attack_velocity = velocity;
    }

    fn aftertouch(&mut self, velocity: u8) {
        self.aftertouch_is_pending = true;
        self.aftertouch_velocity = velocity;
    }

    fn release(&mut self, velocity: u8) {
        self.release_is_pending = true;
        self.release_velocity = velocity;
    }
}
impl SourcesAudio for SimpleVoice {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        self.handle_pending_note_events(clock);
        self.oscillator.source_audio(clock) * self.envelope.source_audio(clock)
    }
}

impl SimpleVoice {
    fn handle_pending_note_events(&mut self, clock: &Clock) {
        if self.attack_is_pending {
            self.attack_is_pending = false;
            self.envelope.handle_note_event(clock, true);
        }
        if self.aftertouch_is_pending {
            self.aftertouch_is_pending = false;
            // TODO: do something
        }
        if self.release_is_pending {
            self.release_is_pending = false;
            self.envelope.handle_note_event(clock, false);
        }
        self.is_playing = !self.envelope.is_idle_for_time(clock);
    }
}

#[derive(Control, Debug, Uid)]
pub struct SimpleSynthesizer {
    uid: usize,
    inner_synth: Synthesizer<SimpleVoice>,
}
impl IsInstrument for SimpleSynthesizer {}
impl Updateable for SimpleSynthesizer {
    type Message = EntityMessage;

    fn update(
        &mut self,
        _clock: &Clock,
        message: Self::Message,
    ) -> crate::traits::Response<Self::Message> {
        match message {
            EntityMessage::Midi(_channel, message) => {
                self.inner_synth.handle_midi_message(&message);
            }
            _ => todo!(),
        }
        crate::traits::Response::none()
    }
}
impl SourcesAudio for SimpleSynthesizer {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        self.inner_synth.source_audio(clock)
    }
}
impl Default for SimpleSynthesizer {
    fn default() -> Self {
        let mut voice_store = Box::new(SimpleVoiceStore::<SimpleVoice>::default());
        voice_store.add_voice(Box::new(SimpleVoice::default()));
        voice_store.add_voice(Box::new(SimpleVoice::default()));
        voice_store.add_voice(Box::new(SimpleVoice::default()));
        voice_store.add_voice(Box::new(SimpleVoice::default()));
        Self {
            uid: Default::default(),
            inner_synth: Synthesizer::<SimpleVoice>::new_with(voice_store),
        }
    }
}
impl SimpleSynthesizer {
    pub fn notes_playing(&self) -> usize {
        0
    }
}

#[derive(Debug, Default)]
pub struct SimpleVoiceStore<V: IsVoice> {
    voices: Vec<Box<V>>,
    notes_playing: Vec<u7>,
}
impl<V: IsVoice> VoiceStore for SimpleVoiceStore<V> {
    type Voice = V;

    fn voice_count(&self) -> usize {
        self.voices.len()
    }

    fn active_voice_count(&self) -> usize {
        self.voices.iter().filter(|v| v.is_playing()).count()
    }

    fn get_voice(&mut self, key: &midly::num::u7) -> Result<&mut Box<Self::Voice>> {
        if let Some(index) = self.notes_playing.iter().position(|note| *key == *note) {
            return Ok(&mut self.voices[index]);
        }
        for (index, voice) in self.voices.iter().enumerate() {
            if voice.is_playing() {
                continue;
            }
            self.notes_playing[index] = *key;
            return Ok(&mut self.voices[index]);
        }
        Err(anyhow!("out of voices"))
    }
}
impl<V: IsVoice> SourcesAudio for SimpleVoiceStore<V> {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        self.voices.iter_mut().map(|v| v.source_audio(clock)).sum()
    }
}
impl<V: IsVoice> SimpleVoiceStore<V> {
    fn add_voice(&mut self, voice: Box<V>) {
        self.voices.push(voice);
        self.notes_playing.push(u7::from(0));
    }
}

#[derive(Debug, Default)]
pub struct VoicePerNoteStore<V: IsVoice> {
    voices: HashMap<u7, Box<V>>,
}
impl<V: IsVoice> VoiceStore for VoicePerNoteStore<V> {
    type Voice = V;

    fn voice_count(&self) -> usize {
        self.voices.len()
    }
    fn active_voice_count(&self) -> usize {
        self.voices.iter().filter(|(_k, v)| v.is_playing()).count()
    }
    fn get_voice(&mut self, key: &midly::num::u7) -> Result<&mut Box<Self::Voice>> {
        if let Some(voice) = self.voices.get_mut(key) {
            return Ok(voice);
        }
        Err(anyhow!("no voice for key {}", key))
    }
}
impl<V: IsVoice> SourcesAudio for VoicePerNoteStore<V> {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        self.voices
            .values_mut()
            .map(|v| v.source_audio(clock))
            .sum()
    }
}
impl<V: IsVoice> VoicePerNoteStore<V> {
    fn add_voice(&mut self, key: u7, voice: Box<V>) {
        self.voices.insert(key, voice);
    }
}

#[derive(Debug)]
pub struct FmVoice {
    carrier: Oscillator,
    modulator: Oscillator,
    modulator_depth: f32,
    envelope: AdsrEnvelope,

    is_playing: bool,
    attack_is_pending: bool,
    attack_velocity: u8,
    release_is_pending: bool,
    release_velocity: u8,
    aftertouch_is_pending: bool,
    aftertouch_velocity: u8,
}
impl IsVoice for FmVoice {}
impl PlaysNotes for FmVoice {
    fn is_playing(&self) -> bool {
        self.is_playing
    }

    fn set_frequency_hz(&mut self, frequency_hz: f32) {
        self.carrier.set_frequency(frequency_hz);
    }

    fn attack(&mut self, velocity: u8) {
        self.attack_is_pending = true;
        self.attack_velocity = velocity;
    }

    fn aftertouch(&mut self, velocity: u8) {
        self.aftertouch_is_pending = true;
        self.aftertouch_velocity = velocity;
    }

    fn release(&mut self, velocity: u8) {
        self.release_is_pending = true;
        self.release_velocity = velocity;
    }
}
impl SourcesAudio for FmVoice {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        self.handle_pending_note_events(clock);
        self.carrier
            .set_frequency_modulation(self.modulator.source_audio(clock) * self.modulator_depth);
        self.carrier.source_audio(clock) * self.envelope.source_audio(clock)
    }
}
impl Default for FmVoice {
    fn default() -> Self {
        Self {
            carrier: Default::default(),
            modulator: Default::default(),
            modulator_depth: 0.2,
            envelope: AdsrEnvelope::new_with(&EnvelopeSettings {
                attack: 0.1,
                decay: 0.1,
                sustain: 0.8,
                release: 0.25,
            }),
            is_playing: Default::default(),
            attack_is_pending: Default::default(),
            attack_velocity: Default::default(),
            release_is_pending: Default::default(),
            release_velocity: Default::default(),
            aftertouch_is_pending: Default::default(),
            aftertouch_velocity: Default::default(),
        }
    }
}
impl FmVoice {
    pub(crate) fn new_with(modulator_frequency: f32) -> Self {
        let mut modulator = Oscillator::default();
        modulator.set_frequency(modulator_frequency);
        Self {
            modulator,
            ..Default::default()
        }
    }
    fn handle_pending_note_events(&mut self, clock: &Clock) {
        if self.attack_is_pending {
            self.attack_is_pending = false;
            self.envelope.handle_note_event(clock, true);
        }
        if self.aftertouch_is_pending {
            self.aftertouch_is_pending = false;
            // TODO: do something
        }
        if self.release_is_pending {
            self.release_is_pending = false;
            self.envelope.handle_note_event(clock, false);
        }
        self.is_playing = !self.envelope.is_idle_for_time(clock);
    }

    #[allow(dead_code)]
    pub fn modulator_frequency(&self) -> f32 {
        self.modulator.frequency()
    }

    #[allow(dead_code)]
    pub fn set_modulator_frequency(&mut self, value: f32) {
        self.modulator.set_frequency(value);
    }
}

#[derive(Control, Debug, Uid)]
pub struct FmSynthesizer {
    uid: usize,
    inner_synth: Synthesizer<FmVoice>,
}
impl IsInstrument for FmSynthesizer {}
impl Updateable for FmSynthesizer {
    type Message = EntityMessage;

    fn update(
        &mut self,
        _clock: &Clock,
        message: Self::Message,
    ) -> crate::traits::Response<Self::Message> {
        match message {
            EntityMessage::Midi(_channel, message) => {
                self.inner_synth.handle_midi_message(&message)
            }
            _ => todo!(),
        }
        Response::none()
    }
}
impl SourcesAudio for FmSynthesizer {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        self.inner_synth.source_audio(clock)
    }
}
impl Default for FmSynthesizer {
    fn default() -> Self {
        let mut voice_store = Box::new(SimpleVoiceStore::<FmVoice>::default());
        for _ in 0..4 {
            voice_store.add_voice(Box::new(FmVoice::default()));
        }
        Self::new_with_voice_store(voice_store)
    }
}
impl FmSynthesizer {
    pub(crate) fn new_with(preset: &FmSynthesizerPreset) -> Self {
        let voice_store = Box::new(SimpleVoiceStore::<FmVoice>::new_with(preset));
        Self {
            uid: Default::default(),
            inner_synth: Synthesizer::<FmVoice>::new_with(voice_store),
        }
    }

    pub(crate) fn new_with_voice_store(voice_store: Box<dyn VoiceStore<Voice = FmVoice>>) -> Self {
        Self {
            uid: Default::default(),
            inner_synth: Synthesizer::<FmVoice>::new_with(voice_store),
        }
    }

    pub(crate) fn preset_for_name(_name: &str) -> FmSynthesizerPreset {
        FmSynthesizerPreset {
            modulator_frequency_hz: 388.0,
        }
    }
}

impl SimpleVoiceStore<FmVoice> {
    pub(crate) fn new_with(preset: &FmSynthesizerPreset) -> Self {
        let mut voice_store = SimpleVoiceStore::<FmVoice>::default();
        for _ in 0..4 {
            voice_store.add_voice(Box::new(FmVoice::new_with(preset.modulator_frequency_hz)));
        }
        voice_store
    }
}

pub(crate) struct FmSynthesizerPreset {
    modulator_frequency_hz: f32,
}
