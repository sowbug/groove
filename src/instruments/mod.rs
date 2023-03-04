pub use drumkit::Drumkit;
pub use sampler::Sampler;
pub use welsh::WelshSynth;

pub(crate) mod drumkit;
pub(crate) mod envelopes;
pub(crate) mod oscillators;
pub(crate) mod sampler;
pub(crate) mod welsh;

use anyhow::{anyhow, Result};
use groove_core::{
    control::F32ControlValue,
    generators::{EnvelopeGenerator, Oscillator, Waveform},
    midi::{note_to_frequency, u7, HandlesMidi, MidiChannel, MidiMessage},
    time::ClockTimeUnit,
    traits::{
        Controllable, Generates, GeneratesEnvelope, HasUid, IsInstrument, IsStereoSampleVoice,
        IsVoice, PlaysNotes, Resets, StoresVoices, Ticks,
    },
    BipolarNormal, Normal, ParameterType, Sample, SampleType, StereoSample,
};
use groove_macros::{Control, Uid};
use std::{
    collections::{HashMap, VecDeque},
    fmt::Debug,
    marker::PhantomData,
    str::FromStr,
};
use strum_macros::{Display, EnumString, FromRepr};

#[derive(Debug, Default)]
pub(crate) struct PlaysNotesEventTracker {
    note_on_is_pending: bool,
    note_on_key: u8,
    note_on_velocity: u8,

    note_off_is_pending: bool,
    note_off_velocity: u8,

    aftertouch_is_pending: bool,
    aftertouch_velocity: u8,

    steal_is_pending: bool,
    steal_is_underway: bool,
}
impl PlaysNotesEventTracker {
    fn has_pending_events(&self) -> bool {
        self.note_on_is_pending
            || self.note_off_is_pending
            || self.aftertouch_is_pending
            || self.steal_is_pending
    }

    fn reset(&mut self) {
        self.clear_pending();
        self.note_on_key = 0;
        self.note_on_velocity = 0;
        self.aftertouch_velocity = 0;
        self.note_off_velocity = 0;
        self.steal_is_underway = false;
    }

    fn clear_pending(&mut self) {
        self.note_on_is_pending = false;
        self.note_off_is_pending = false;
        self.aftertouch_is_pending = false;
        self.steal_is_pending = false;
    }

    fn enqueue_note_on(&mut self, key: u8, velocity: u8) {
        self.note_on_is_pending = true;
        self.note_on_key = key;
        self.note_on_velocity = velocity;
    }

    fn enqueue_steal(&mut self, key: u8, velocity: u8) {
        self.steal_is_pending = true;
        self.note_on_key = key;
        self.note_on_velocity = velocity;
    }

    fn enqueue_aftertouch(&mut self, velocity: u8) {
        self.aftertouch_is_pending = true;
        self.aftertouch_velocity = velocity;
    }

    fn enqueue_note_off(&mut self, velocity: u8) {
        self.note_off_is_pending = true;
        self.note_off_velocity = velocity;
    }

    fn handle_steal_start(&mut self) {
        self.steal_is_pending = false;
        self.steal_is_underway = true;
    }

    fn handle_steal_end(&mut self) {
        if self.steal_is_underway {
            self.steal_is_underway = false;
            self.enqueue_note_on(self.note_on_key, self.note_on_velocity);
        }
    }
}

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

    pub fn pan(&self) -> f32 {
        self.pan
    }

    pub fn set_pan(&mut self, pan: f32) {
        self.pan = pan;
        self.voice_store.set_pan(pan);
    }

    pub(crate) fn set_control_pan(&mut self, value: F32ControlValue) {
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
    envelope: EnvelopeGenerator,

    sample: StereoSample,

    is_playing: bool,
    event_tracker: PlaysNotesEventTracker,
}
impl IsStereoSampleVoice for SimpleVoice {}
impl IsVoice<StereoSample> for SimpleVoice {}
impl PlaysNotes for SimpleVoice {
    fn is_playing(&self) -> bool {
        self.is_playing
    }

    fn has_pending_events(&self) -> bool {
        self.event_tracker.has_pending_events()
    }

    fn note_on(&mut self, key: u8, velocity: u8) {
        if self.is_active() {
            self.event_tracker.enqueue_steal(key, velocity);
        } else {
            self.event_tracker.enqueue_note_on(key, velocity);
        }
    }

    fn aftertouch(&mut self, velocity: u8) {
        self.event_tracker.enqueue_aftertouch(velocity);
    }

    fn note_off(&mut self, velocity: u8) {
        self.event_tracker.enqueue_note_off(velocity);
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
        self.event_tracker.reset();
    }
}
impl Ticks for SimpleVoice {
    fn tick(&mut self, tick_count: usize) {
        for _ in 0..tick_count {
            self.handle_pending_note_events();
            self.oscillator.tick(1);
            self.envelope.tick(1);
            let is_playing = self.is_playing;
            self.is_playing = !self.envelope.is_idle();
            if is_playing && !self.is_playing {
                self.event_tracker.handle_steal_end();
            }
            self.sample =
                StereoSample::from(self.oscillator.value() * self.envelope.value().value());
        }
    }
}

impl SimpleVoice {
    pub fn new_with(sample_rate: usize) -> Self {
        Self {
            sample_rate,
            oscillator: Oscillator::new_with(sample_rate),
            envelope: EnvelopeGenerator::new_with(sample_rate, 0.0, 0.0, Normal::maximum(), 0.0),
            sample: Default::default(),
            is_playing: Default::default(),
            event_tracker: Default::default(),
        }
    }
    fn set_frequency_hz(&mut self, frequency_hz: ParameterType) {
        self.oscillator.set_frequency(frequency_hz);
    }
    fn handle_pending_note_events(&mut self) {
        if self.event_tracker.steal_is_pending {
            self.handle_steal_event()
        }
        if self.event_tracker.note_on_is_pending && self.event_tracker.note_off_is_pending {
            // Handle the case where both are pending at the same time.
            if self.is_playing {
                self.handle_note_off_event();
                self.handle_note_on_event();
            } else {
                self.handle_note_on_event();
                self.handle_note_off_event();
            }
        } else {
            if self.event_tracker.note_off_is_pending {
                self.handle_note_off_event();
            }
            if self.event_tracker.note_on_is_pending {
                self.handle_note_on_event();
            }
        }
        if self.event_tracker.aftertouch_is_pending {
            self.handle_aftertouch_event();
        }
        self.event_tracker.clear_pending();
    }

    fn handle_note_on_event(&mut self) {
        self.set_frequency_hz(note_to_frequency(self.event_tracker.note_on_key));
        self.envelope.trigger_attack();
    }

    fn handle_aftertouch_event(&mut self) {
        // TODO: do something
    }

    fn handle_note_off_event(&mut self) {
        self.envelope.trigger_release();
    }

    fn handle_steal_event(&mut self) {
        self.event_tracker.handle_steal_start();
        self.envelope.trigger_shutdown();
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
        let mut voice_store = Box::new(SimpleVoiceStore::<SimpleVoice>::new_with(sample_rate));
        for _ in 0..4 {
            voice_store.add_voice(Box::new(SimpleVoice::new_with(sample_rate)));
        }
        Self {
            uid: Default::default(),
            inner_synth: Synthesizer::<SimpleVoice>::new_with(sample_rate, voice_store),
        }
    }
    pub fn notes_playing(&self) -> usize {
        0
    }
}

#[derive(Debug)]
pub struct SimpleVoiceStore<V: IsStereoSampleVoice> {
    sample: StereoSample,
    voices: Vec<Box<V>>,
    notes_playing: Vec<u7>,
}
impl<V: IsStereoSampleVoice> StoresVoices for SimpleVoiceStore<V> {
    type Voice = V;

    fn voice_count(&self) -> usize {
        self.voices.len()
    }

    fn active_voice_count(&self) -> usize {
        self.voices.iter().filter(|v| v.is_playing()).count()
    }

    fn get_voice(&mut self, key: &u7) -> Result<&mut Box<Self::Voice>> {
        // If we have a voice already going for this note, return it.
        if let Some(index) = self.notes_playing.iter().position(|note| *key == *note) {
            return Ok(&mut self.voices[index]);
        }
        // If we can find an inactive voice, return it.
        for (index, voice) in self.voices.iter().enumerate() {
            if voice.is_active() {
                continue;
            }
            self.notes_playing[index] = *key;
            return Ok(&mut self.voices[index]);
        }

        Err(anyhow!("out of voices"))
    }

    fn set_pan(&mut self, value: f32) {
        for voice in self.voices.iter_mut() {
            voice.set_pan(value);
        }
    }
}
impl<V: IsStereoSampleVoice> Generates<StereoSample> for SimpleVoiceStore<V> {
    fn value(&self) -> StereoSample {
        self.sample
    }

    #[allow(unused_variables)]
    fn batch_values(&mut self, values: &mut [StereoSample]) {
        todo!()
    }
}
impl<V: IsStereoSampleVoice> Resets for SimpleVoiceStore<V> {
    fn reset(&mut self, sample_rate: usize) {
        self.voices.iter_mut().for_each(|v| v.reset(sample_rate));
    }
}
impl<V: IsStereoSampleVoice> Ticks for SimpleVoiceStore<V> {
    // TODO: this is not at all taking advantage of batching. When
    // batch_sample() calls it, it's lame.
    fn tick(&mut self, tick_count: usize) {
        self.voices.iter_mut().for_each(|v| v.tick(tick_count));
        self.sample = self.voices.iter().map(|v| v.value()).sum();
        self.voices.iter().enumerate().for_each(|(index, voice)| {
            if !voice.is_active() {
                self.notes_playing[index] = u7::from(0);
            }
        });
    }
}
impl<V: IsStereoSampleVoice> SimpleVoiceStore<V> {
    pub fn new_with(_sample_rate: usize) -> Self {
        Self {
            sample: Default::default(),
            voices: Default::default(),
            notes_playing: Default::default(),
        }
    }
    fn add_voice(&mut self, voice: Box<V>) {
        self.voices.push(voice);
        self.notes_playing.push(u7::from(0));
    }

    // When we need, make the voice count configurable.
    pub(crate) fn new_with_voice<F>(sample_rate: usize, new_voice_fn: F) -> Self
    where
        F: Fn() -> V,
    {
        let mut voice_store = Self::new_with(sample_rate);
        for _ in 0..4 {
            voice_store.add_voice(Box::new(new_voice_fn()));
        }
        voice_store
    }
}

/// A VoiceStore that steals voices to satisfy get_voice().
#[derive(Debug)]
pub struct StealingVoiceStore<V: IsStereoSampleVoice> {
    sample: StereoSample,
    voices: Vec<Box<V>>,
    notes_playing: Vec<u7>,
}
impl<V: IsStereoSampleVoice> StoresVoices for StealingVoiceStore<V> {
    type Voice = V;

    fn voice_count(&self) -> usize {
        self.voices.len()
    }

    fn active_voice_count(&self) -> usize {
        self.voices.iter().filter(|v| v.is_playing()).count()
    }

    fn get_voice(&mut self, key: &u7) -> Result<&mut Box<Self::Voice>> {
        // If we have a voice already going for this note, return it.
        if let Some(index) = self.notes_playing.iter().position(|note| *key == *note) {
            return Ok(&mut self.voices[index]);
        }
        // If we can find an inactive voice, return it.
        for (index, voice) in self.voices.iter().enumerate() {
            if voice.is_active() {
                continue;
            }
            self.notes_playing[index] = *key;
            return Ok(&mut self.voices[index]);
        }

        // We need to steal a voice. For now, let's just pick the first one in the list.
        let index = 0;
        self.notes_playing[index] = *key;
        return Ok(&mut self.voices[index]);

        #[allow(unreachable_code)]
        Err(anyhow!("out of voices"))
    }

    fn set_pan(&mut self, value: f32) {
        for voice in self.voices.iter_mut() {
            voice.set_pan(value);
        }
    }
}
impl<V: IsStereoSampleVoice> Generates<StereoSample> for StealingVoiceStore<V> {
    fn value(&self) -> StereoSample {
        self.sample
    }

    #[allow(unused_variables)]
    fn batch_values(&mut self, values: &mut [StereoSample]) {
        todo!()
    }
}
impl<V: IsStereoSampleVoice> Resets for StealingVoiceStore<V> {
    fn reset(&mut self, sample_rate: usize) {
        self.voices.iter_mut().for_each(|v| v.reset(sample_rate));
    }
}
impl<V: IsStereoSampleVoice> Ticks for StealingVoiceStore<V> {
    // TODO: this is not at all taking advantage of batching. When
    // batch_sample() calls it, it's lame.
    fn tick(&mut self, tick_count: usize) {
        self.voices.iter_mut().for_each(|v| v.tick(tick_count));
        self.sample = self.voices.iter().map(|v| v.value()).sum();
        self.voices.iter().enumerate().for_each(|(index, voice)| {
            if !voice.is_active() {
                self.notes_playing[index] = u7::from(0);
            }
        });
    }
}
impl<V: IsStereoSampleVoice> StealingVoiceStore<V> {
    pub fn new_with(_sample_rate: usize) -> Self {
        Self {
            sample: Default::default(),
            voices: Default::default(),
            notes_playing: Default::default(),
        }
    }
    pub fn add_voice(&mut self, voice: Box<V>) {
        self.voices.push(voice);
        self.notes_playing.push(u7::from(0));
    }
}

#[derive(Debug)]
pub struct VoicePerNoteStore<V: IsStereoSampleVoice> {
    sample: StereoSample,
    voices: HashMap<u7, Box<V>>,
}
impl<V: IsStereoSampleVoice> StoresVoices for VoicePerNoteStore<V> {
    type Voice = V;

    fn voice_count(&self) -> usize {
        self.voices.len()
    }
    fn active_voice_count(&self) -> usize {
        self.voices.iter().filter(|(_k, v)| v.is_playing()).count()
    }
    fn get_voice(&mut self, key: &u7) -> Result<&mut Box<Self::Voice>> {
        if let Some(voice) = self.voices.get_mut(key) {
            return Ok(voice);
        }
        Err(anyhow!("no voice for key {}", key))
    }
    fn set_pan(&mut self, value: f32) {
        for voice in self.voices.iter_mut() {
            voice.1.set_pan(value);
        }
    }
}
impl<V: IsStereoSampleVoice> Generates<StereoSample> for VoicePerNoteStore<V> {
    fn value(&self) -> StereoSample {
        self.sample
    }

    #[allow(unused_variables)]
    fn batch_values(&mut self, values: &mut [StereoSample]) {
        todo!()
    }
}
impl<V: IsStereoSampleVoice> Resets for VoicePerNoteStore<V> {
    fn reset(&mut self, sample_rate: usize) {
        self.voices.values_mut().for_each(|v| v.reset(sample_rate));
    }
}
impl<V: IsStereoSampleVoice> Ticks for VoicePerNoteStore<V> {
    fn tick(&mut self, tick_count: usize) {
        self.voices.values_mut().for_each(|v| v.tick(tick_count));
        self.sample = self.voices.values().map(|v| v.value()).sum();
    }
}
impl<V: IsStereoSampleVoice> VoicePerNoteStore<V> {
    pub fn new_with(_sample_rate: usize) -> Self {
        Self {
            sample: Default::default(),
            voices: Default::default(),
        }
    }
    fn add_voice(&mut self, key: u7, voice: Box<V>) {
        self.voices.insert(key, voice);
    }
}

#[derive(Debug)]
pub struct FmVoice {
    sample: StereoSample,
    carrier: Oscillator,
    modulator: Oscillator,
    modulator_depth: ParameterType,
    envelope: EnvelopeGenerator,
    dca: Dca,

    is_playing: bool,
    event_tracker: PlaysNotesEventTracker,
}
impl IsStereoSampleVoice for FmVoice {}
impl IsVoice<StereoSample> for FmVoice {}
impl PlaysNotes for FmVoice {
    fn is_playing(&self) -> bool {
        self.is_playing
    }

    fn has_pending_events(&self) -> bool {
        self.event_tracker.has_pending_events()
    }

    fn note_on(&mut self, key: u8, velocity: u8) {
        if self.is_active() {
            self.event_tracker.enqueue_steal(key, velocity);
        } else {
            self.event_tracker.enqueue_note_on(key, velocity);
        }
    }

    fn aftertouch(&mut self, velocity: u8) {
        self.event_tracker.enqueue_aftertouch(velocity);
    }

    fn note_off(&mut self, velocity: u8) {
        self.event_tracker.enqueue_note_off(velocity);
    }

    fn set_pan(&mut self, value: f32) {
        self.dca.set_pan(BipolarNormal::from(value));
    }
}
impl Generates<StereoSample> for FmVoice {
    fn value(&self) -> StereoSample {
        todo!()
    }

    #[allow(unused_variables)]
    fn batch_values(&mut self, values: &mut [StereoSample]) {
        todo!()
    }
}
impl Resets for FmVoice {
    fn reset(&mut self, sample_rate: usize) {
        self.envelope.reset(sample_rate);
        self.carrier.reset(sample_rate);
        self.modulator.reset(sample_rate);
        self.event_tracker.reset();
    }
}
impl Ticks for FmVoice {
    fn tick(&mut self, tick_count: usize) {
        self.handle_pending_note_events();
        self.carrier.set_frequency_modulation(BipolarNormal::from(
            self.modulator.value() * self.modulator_depth,
        ));
        self.envelope.tick(tick_count);
        self.carrier.tick(tick_count);
        self.modulator.tick(tick_count);
        let r = self.carrier.value() * self.envelope.value().value();
        let is_playing = self.is_playing;
        self.is_playing = !self.envelope.is_idle();
        if is_playing && !self.is_playing {
            self.event_tracker.handle_steal_end();
        }
        self.sample = self.dca.transform_audio_to_stereo(Sample(r));
    }
}
impl FmVoice {
    pub(crate) fn new_with(sample_rate: usize) -> Self {
        Self {
            sample: Default::default(),
            carrier: Oscillator::new_with(sample_rate),
            modulator: Oscillator::new_with(sample_rate),
            modulator_depth: 0.2,
            envelope: EnvelopeGenerator::new_with(sample_rate, 0.1, 0.1, Normal::new(0.8), 0.25),
            dca: Default::default(),
            is_playing: Default::default(),
            event_tracker: Default::default(),
        }
    }
    pub(crate) fn new_with_modulator_frequency(
        sample_rate: usize,
        modulator_frequency: ParameterType,
    ) -> Self {
        let mut modulator = Oscillator::new_with(sample_rate);
        modulator.set_frequency(modulator_frequency);
        let mut r = Self::new_with(sample_rate);
        r.modulator = modulator;
        r
    }
    fn handle_pending_note_events(&mut self) {
        if self.event_tracker.steal_is_pending {
            self.handle_steal_event();
        }
        if self.event_tracker.note_on_is_pending && self.event_tracker.note_off_is_pending {
            // Handle the case where both are pending at the same time.
            if self.is_playing {
                self.handle_note_off_event();
                self.handle_note_on_event();
            } else {
                self.handle_note_on_event();
                self.handle_note_off_event();
            }
        } else {
            if self.event_tracker.note_off_is_pending {
                self.handle_note_off_event();
            }
            if self.event_tracker.note_on_is_pending {
                self.handle_note_on_event();
            }
        }
        if self.event_tracker.aftertouch_is_pending {
            self.handle_aftertouch_event();
        }
        self.event_tracker.clear_pending();
    }

    fn handle_note_on_event(&mut self) {
        self.set_frequency_hz(note_to_frequency(self.event_tracker.note_on_key));
        self.envelope.trigger_attack();
    }

    fn handle_aftertouch_event(&mut self) {
        // TODO: do something
    }

    fn handle_note_off_event(&mut self) {
        self.envelope.trigger_release();
    }

    fn handle_steal_event(&mut self) {
        self.event_tracker.handle_steal_start();
        self.envelope.trigger_shutdown();
    }

    #[allow(dead_code)]
    pub fn modulator_frequency(&self) -> ParameterType {
        self.modulator.frequency()
    }

    #[allow(dead_code)]
    pub fn set_modulator_frequency(&mut self, value: ParameterType) {
        self.modulator.set_frequency(value);
    }

    fn set_frequency_hz(&mut self, frequency_hz: ParameterType) {
        self.carrier.set_frequency(frequency_hz);
    }
}

#[derive(Control, Debug, Uid)]
pub struct FmSynthesizer {
    uid: usize,
    inner_synth: Synthesizer<FmVoice>,
}
impl IsInstrument for FmSynthesizer {}
impl Generates<StereoSample> for FmSynthesizer {
    fn value(&self) -> StereoSample {
        self.inner_synth.value()
    }

    fn batch_values(&mut self, values: &mut [StereoSample]) {
        self.inner_synth.batch_values(values);
    }
}
impl Resets for FmSynthesizer {
    fn reset(&mut self, sample_rate: usize) {
        self.inner_synth.reset(sample_rate)
    }
}
impl Ticks for FmSynthesizer {
    fn tick(&mut self, tick_count: usize) {
        self.inner_synth.tick(tick_count);
    }
}
impl HandlesMidi for FmSynthesizer {
    fn handle_midi_message(
        &mut self,
        message: &MidiMessage,
    ) -> Option<Vec<(MidiChannel, MidiMessage)>> {
        self.inner_synth.handle_midi_message(message)
    }
}
impl FmSynthesizer {
    #[allow(dead_code)]
    pub(crate) fn new_with(sample_rate: usize) -> Self {
        let mut voice_store = Box::new(SimpleVoiceStore::<FmVoice>::new_with(sample_rate));
        for _ in 0..4 {
            voice_store.add_voice(Box::new(FmVoice::new_with(sample_rate)));
        }
        Self::new_with_voice_store(sample_rate, voice_store)
    }
    pub(crate) fn new_with_preset(sample_rate: usize, preset: &FmSynthesizerPreset) -> Self {
        let voice_store = Box::new(SimpleVoiceStore::<FmVoice>::new_with_preset(
            sample_rate,
            preset,
        ));
        Self {
            uid: Default::default(),
            inner_synth: Synthesizer::<FmVoice>::new_with(sample_rate, voice_store),
        }
    }

    pub(crate) fn new_with_voice_store(
        sample_rate: usize,
        voice_store: Box<dyn StoresVoices<Voice = FmVoice>>,
    ) -> Self {
        Self {
            uid: Default::default(),
            inner_synth: Synthesizer::<FmVoice>::new_with(sample_rate, voice_store),
        }
    }

    pub(crate) fn preset_for_name(_name: &str) -> FmSynthesizerPreset {
        FmSynthesizerPreset {
            modulator_frequency_hz: 388.0,
        }
    }
}

impl SimpleVoiceStore<FmVoice> {
    pub(crate) fn new_with_preset(sample_rate: usize, preset: &FmSynthesizerPreset) -> Self {
        let mut voice_store = SimpleVoiceStore::<FmVoice>::new_with(sample_rate);
        for _ in 0..4 {
            voice_store.add_voice(Box::new(FmVoice::new_with_modulator_frequency(
                sample_rate,
                preset.modulator_frequency_hz,
            )));
        }
        voice_store
    }
}

pub(crate) struct FmSynthesizerPreset {
    modulator_frequency_hz: ParameterType,
}

/// The Digitally Controller Amplifier (DCA) handles gain and pan for many kinds
/// of synths.
///
/// See DSSPC++, Section 7.9 for requirements. TODO: implement
#[derive(Debug)]
pub(crate) struct Dca {
    gain: f64,
    pan: f64,
}
impl Default for Dca {
    fn default() -> Self {
        Self {
            gain: 1.0,
            pan: 0.0,
        }
    }
}
impl Dca {
    #[allow(dead_code)]
    pub(crate) fn set_pan(&mut self, value: BipolarNormal) {
        self.pan = value.value()
    }

    pub(crate) fn transform_audio_to_stereo(&mut self, input_sample: Sample) -> StereoSample {
        // See Pirkle, DSSPC++, p.73
        let input_sample: f64 = input_sample.0 * self.gain;
        let left_pan: f64 = 1.0 - 0.25 * (self.pan + 1.0).powi(2);
        let right_pan: f64 = 1.0 - (0.5 * self.pan - 0.5).powi(2);
        StereoSample::new_from_f64(left_pan * input_sample, right_pan * input_sample)
    }
}

/// A simple implementation of IsInstrument that's useful for testing and
/// debugging. Uses a default Oscillator to produce sound, and its "envelope" is
/// just a boolean that responds to MIDI NoteOn/NoteOff.
///
/// To act as a controller target, it has two parameters: Oscillator waveform
/// and frequency.
#[derive(Control, Debug, Uid)]
pub struct TestInstrument {
    uid: usize,
    sample_rate: usize,
    sample: StereoSample,

    /// -1.0 is Sawtooth, 1.0 is Square, anything else is Sine.
    #[controllable]
    pub waveform: PhantomData<Waveform>, // interesting use of PhantomData

    #[controllable]
    pub fake_value: f32,

    oscillator: Oscillator,
    dca: Dca,
    pub is_playing: bool,
    pub received_count: usize,
    pub handled_count: usize,

    pub checkpoint_values: VecDeque<f32>,
    pub checkpoint: f32,
    pub checkpoint_delta: f32,
    pub time_unit: ClockTimeUnit,

    pub debug_messages: Vec<MidiMessage>,
}
impl IsInstrument for TestInstrument {}
impl Generates<StereoSample> for TestInstrument {
    fn value(&self) -> StereoSample {
        self.sample
    }

    #[allow(unused_variables)]
    fn batch_values(&mut self, values: &mut [StereoSample]) {
        todo!()
    }
}
impl Resets for TestInstrument {
    fn reset(&mut self, sample_rate: usize) {
        self.oscillator.reset(sample_rate);
    }
}
impl Ticks for TestInstrument {
    fn tick(&mut self, tick_count: usize) {
        self.oscillator.tick(tick_count);
        // If we've been asked to assert values at checkpoints, do so.

        // TODODODODO
        // if !self.checkpoint_values.is_empty() && clock.time_for(&self.time_unit) >= self.checkpoint
        // {
        //     const SAD_FLOAT_DIFF: f32 = 1.0e-2;
        //     assert_approx_eq!(self.fake_value, self.checkpoint_values[0], SAD_FLOAT_DIFF);
        //     self.checkpoint += self.checkpoint_delta;
        //     self.checkpoint_values.pop_front();
        // }
        self.sample = if self.is_playing {
            self.dca
                .transform_audio_to_stereo(Sample::from(self.oscillator.value()))
        } else {
            StereoSample::SILENCE
        };
    }
}
impl HandlesMidi for TestInstrument {
    fn handle_midi_message(
        &mut self,
        message: &MidiMessage,
    ) -> Option<Vec<(MidiChannel, MidiMessage)>> {
        self.debug_messages.push(*message);
        self.received_count += 1;

        match message {
            MidiMessage::NoteOn { key, vel: _ } => {
                self.is_playing = true;
                self.oscillator
                    .set_frequency(note_to_frequency(key.as_int()));
            }
            MidiMessage::NoteOff { key: _, vel: _ } => {
                self.is_playing = false;
            }
            _ => {}
        }
        None
    }
}
// impl TestsValues for TestInstrument {
//     fn has_checkpoint_values(&self) -> bool {
//         !self.checkpoint_values.is_empty()
//     }

//     fn time_unit(&self) -> &ClockTimeUnit {
//         &self.time_unit
//     }

//     fn checkpoint_time(&self) -> f32 {
//         self.checkpoint
//     }

//     fn advance_checkpoint_time(&mut self) {
//         self.checkpoint += self.checkpoint_delta;
//     }

//     fn value_to_check(&self) -> f32 {
//         self.fake_value
//     }

//     fn pop_checkpoint_value(&mut self) -> Option<f32> {
//         self.checkpoint_values.pop_front()
//     }
// }
impl TestInstrument {
    pub fn new_with(sample_rate: usize) -> Self {
        let mut r = Self {
            uid: Default::default(),
            waveform: Default::default(),
            sample_rate,
            sample: Default::default(),
            fake_value: Default::default(),
            oscillator: Oscillator::new_with(sample_rate),
            dca: Default::default(),
            is_playing: Default::default(),
            received_count: Default::default(),
            handled_count: Default::default(),
            checkpoint_values: Default::default(),
            checkpoint: Default::default(),
            checkpoint_delta: Default::default(),
            time_unit: Default::default(),
            debug_messages: Default::default(),
        };
        r.sample_rate = sample_rate;

        r
    }

    pub fn new_with_test_values(
        sample_rate: usize,
        values: &[f32],
        checkpoint: f32,
        checkpoint_delta: f32,
        time_unit: ClockTimeUnit,
    ) -> Self {
        let mut r = Self::new_with(sample_rate);
        r.checkpoint_values = VecDeque::from(Vec::from(values));
        r.checkpoint = checkpoint;
        r.checkpoint_delta = checkpoint_delta;
        r.time_unit = time_unit;
        r
    }

    // TODO: when we have a more specific control param type, we can do a real
    // into/from
    #[allow(dead_code)]
    fn waveform(&self) -> f32 {
        match self.oscillator.waveform() {
            Waveform::Sawtooth => -1.0,
            Waveform::Square => 1.0,
            _ => 0.0,
        }
    }

    pub fn set_control_waveform(&mut self, value: F32ControlValue) {
        self.oscillator.set_waveform(if value.0 == -1.0 {
            Waveform::Sawtooth
        } else if value.0 == 1.0 {
            Waveform::Square
        } else {
            Waveform::Sine
        });
    }

    pub fn set_fake_value(&mut self, fake_value: f32) {
        self.fake_value = fake_value;
    }

    pub fn fake_value(&self) -> f32 {
        self.fake_value
    }

    pub(crate) fn set_control_fake_value(&mut self, fake_value: F32ControlValue) {
        self.set_fake_value(fake_value.0);
    }
}

#[derive(Control, Debug, Uid)]
pub struct TestSynth {
    uid: usize,
    sample_rate: usize,
    sample: StereoSample,

    #[controllable]
    oscillator_modulation: BipolarNormal,

    oscillator: Box<Oscillator>,
    envelope: Box<dyn GeneratesEnvelope>,
}
impl IsInstrument for TestSynth {}
impl Generates<StereoSample> for TestSynth {
    fn value(&self) -> StereoSample {
        self.sample
    }

    #[allow(unused_variables)]
    fn batch_values(&mut self, values: &mut [StereoSample]) {
        todo!()
    }
}
impl Resets for TestSynth {
    fn reset(&mut self, sample_rate: usize) {
        self.sample_rate = sample_rate;
        self.oscillator.reset(sample_rate);
    }
}
impl Ticks for TestSynth {
    fn tick(&mut self, tick_count: usize) {
        // TODO: I don't think this can play sounds, because I don't see how the
        // envelope ever gets triggered.
        self.oscillator.tick(tick_count);
        self.envelope.tick(tick_count);
        self.sample = StereoSample::from(self.oscillator.value() * self.envelope.value().value());
    }
}
impl HandlesMidi for TestSynth {}
impl TestSynth {
    pub fn new_with_components(
        sample_rate: usize,
        oscillator: Box<Oscillator>,
        envelope: Box<dyn GeneratesEnvelope>,
    ) -> Self {
        Self {
            uid: Default::default(),
            sample_rate,
            sample: Default::default(),
            oscillator_modulation: Default::default(),
            oscillator,
            envelope,
        }
    }

    pub fn oscillator_modulation(&self) -> BipolarNormal {
        self.oscillator.frequency_modulation()
    }

    pub fn set_oscillator_modulation(&mut self, oscillator_modulation: BipolarNormal) {
        self.oscillator_modulation = oscillator_modulation;
        self.oscillator
            .set_frequency_modulation(oscillator_modulation);
    }

    pub fn set_control_oscillator_modulation(&mut self, oscillator_modulation: F32ControlValue) {
        self.set_oscillator_modulation(BipolarNormal::from(oscillator_modulation.0));
    }

    #[allow(dead_code)]
    pub(crate) fn new_with(sample_rate: usize) -> Self {
        Self::new_with_components(
            sample_rate,
            Box::new(Oscillator::new_with(sample_rate)),
            Box::new(EnvelopeGenerator::new_with(
                sample_rate,
                0.0,
                0.0,
                Normal::maximum(),
                0.0,
            )),
        )
    }
}

#[derive(Control, Debug, Default, Uid)]
pub struct AudioSource {
    uid: usize,

    #[controllable]
    level: SampleType,
}
impl IsInstrument for AudioSource {}
impl Generates<StereoSample> for AudioSource {
    fn value(&self) -> StereoSample {
        StereoSample::from(self.level)
    }

    #[allow(unused_variables)]
    fn batch_values(&mut self, values: &mut [StereoSample]) {
        todo!()
    }
}
impl Resets for AudioSource {
    fn reset(&mut self, _sample_rate: usize) {}
}
impl Ticks for AudioSource {
    fn tick(&mut self, _tick_count: usize) {}
}
impl HandlesMidi for AudioSource {}
#[allow(dead_code)]
impl AudioSource {
    pub const TOO_LOUD: SampleType = 1.1;
    pub const LOUD: SampleType = 1.0;
    pub const SILENT: SampleType = 0.0;
    pub const QUIET: SampleType = -1.0;
    pub const TOO_QUIET: SampleType = -1.1;

    pub fn new_with(level: SampleType) -> Self {
        Self {
            level,
            ..Default::default()
        }
    }

    pub fn level(&self) -> SampleType {
        self.level
    }

    pub fn set_level(&mut self, level: SampleType) {
        self.level = level;
    }

    fn set_control_level(&mut self, level: F32ControlValue) {
        self.set_level(level.0 as f64);
    }
}

#[cfg(test)]
mod tests {
    use super::SimpleVoice;
    use crate::{
        common::DEFAULT_SAMPLE_RATE,
        instruments::{Dca, PlaysNotes, SimpleVoiceStore, StealingVoiceStore, StoresVoices},
    };
    use float_cmp::approx_eq;
    use groove_core::{
        midi::{note_to_frequency, u7},
        traits::Ticks,
        BipolarNormal, ParameterType, Sample, StereoSample,
    };

    impl SimpleVoice {
        fn debug_is_shutting_down(&self) -> bool {
            true
            // TODO bring back when this moves elsewhere
            //     self.envelope.debug_is_shutting_down()
        }
    }

    #[test]
    fn dca_mainline() {
        let mut dca = Dca::default();
        const VALUE_IN: Sample = Sample(0.5);
        const VALUE: f64 = 0.5;
        assert_eq!(
            dca.transform_audio_to_stereo(VALUE_IN),
            StereoSample::new_from_f64(VALUE * 0.75, VALUE * 0.75),
            "Pan center should give 75% equally to each channel"
        );

        dca.set_pan(BipolarNormal::new(-1.0));
        assert_eq!(
            dca.transform_audio_to_stereo(VALUE_IN),
            StereoSample::new_from_f64(VALUE, 0.0),
            "Pan left should give 100% to left channel"
        );

        dca.set_pan(BipolarNormal::new(1.0));
        assert_eq!(
            dca.transform_audio_to_stereo(VALUE_IN),
            StereoSample::new_from_f64(0.0, VALUE),
            "Pan right should give 100% to right channel"
        );
    }

    #[test]
    fn simple_voice_store_mainline() {
        let mut voice_store = SimpleVoiceStore::<SimpleVoice>::new_with(DEFAULT_SAMPLE_RATE);
        assert_eq!(voice_store.voice_count(), 0);
        assert_eq!(voice_store.active_voice_count(), 0);

        for _ in 0..2 {
            voice_store.add_voice(Box::new(SimpleVoice::new_with(DEFAULT_SAMPLE_RATE)));
        }
        assert_eq!(voice_store.voice_count(), 2);
        assert_eq!(voice_store.active_voice_count(), 0);

        // Request and start the maximum number of voices.
        if let Ok(voice) = voice_store.get_voice(&u7::from(60)) {
            assert!(!voice.is_playing());
            voice.note_on(60, 127);
            voice.tick(1); // We must tick() register the trigger.
            assert!(voice.is_playing());
        }
        if let Ok(voice) = voice_store.get_voice(&u7::from(61)) {
            voice.note_on(61, 127);
            voice.tick(1);
        }

        // Request a voice for a new note that would exceed the count. Should
        // fail.
        assert!(voice_store.get_voice(&u7::from(62)).is_err());

        // Request to get back a voice that's already playing.
        if let Ok(voice) = voice_store.get_voice(&u7::from(60)) {
            assert!(voice.is_playing());
            voice.note_off(127);

            // All SimpleVoice envelope times are instantaneous, so we know the
            // release completes after asking for the next sample.
            voice.tick(1);
            assert!(!voice.is_playing());
        }
    }

    #[test]
    fn stealing_voice_store_mainline() {
        let mut voice_store = StealingVoiceStore::<SimpleVoice>::new_with(DEFAULT_SAMPLE_RATE);
        assert_eq!(voice_store.voice_count(), 0);
        assert_eq!(voice_store.active_voice_count(), 0);

        for _ in 0..2 {
            voice_store.add_voice(Box::new(SimpleVoice::new_with(DEFAULT_SAMPLE_RATE)));
        }
        assert_eq!(voice_store.voice_count(), 2);
        assert_eq!(voice_store.active_voice_count(), 0);

        // Request and start the full number of voices.
        if let Ok(voice) = voice_store.get_voice(&u7::from(60)) {
            assert!(!voice.is_playing());
            voice.note_on(60, 127);
            voice.tick(1); // We must tick() register the trigger.
            assert!(voice.is_playing());
        }
        if let Ok(voice) = voice_store.get_voice(&u7::from(61)) {
            assert!(!voice.is_playing());
            voice.note_on(61, 127);
            voice.tick(1);
        }

        // Request a voice for a new note that would exceed the count. It should
        // already be playing, because we're about to steal it.
        if let Ok(voice) = voice_store.get_voice(&u7::from(62)) {
            assert!(voice.is_playing());

            // This is testing the shutdown state, rather than the voice store,
            // but I'm feeling lazy today.
            voice.note_on(62, 127);
            voice.tick(1);
            assert!(voice.debug_is_shutting_down());
        } else {
            assert!(false, "StealingVoiceStore didn't return a voice");
        }
    }

    #[test]
    fn voice_store_simultaneous_events() {
        let mut voice_store = SimpleVoiceStore::<SimpleVoice>::new_with(DEFAULT_SAMPLE_RATE);
        assert_eq!(voice_store.voice_count(), 0);
        assert_eq!(voice_store.active_voice_count(), 0);

        for _ in 0..2 {
            voice_store.add_voice(Box::new(SimpleVoice::new_with(DEFAULT_SAMPLE_RATE)));
        }
        assert_eq!(voice_store.voice_count(), 2);
        assert_eq!(voice_store.active_voice_count(), 0);

        // Request multiple voices during the same tick.
        if let Ok(voice) = voice_store.get_voice(&u7::from(60)) {
            voice.note_on(60, 127);
            assert!(!voice.is_playing(), "New voice shouldn't be marked is_playing() until both attack() and the next source_audio() have completed");
        }
        if let Ok(voice) = voice_store.get_voice(&u7::from(61)) {
            voice.note_on(61, 127);
            assert!(!voice.is_playing(), "New voice shouldn't be marked is_playing() until both attack() and the next source_audio() have completed");
        }

        // To beat a dead horse, pending operations like attack() and release()
        // aren't fulfilled until the next tick, which happens as part of
        // source_audio().
        assert_eq!(
            voice_store.active_voice_count(),
            0,
            "voices shouldn't be marked as playing until next source_audio()"
        );
        voice_store.tick(1);
        assert_eq!(voice_store.active_voice_count(), 2, "voices with pending attacks() should have been handled, and they should now be is_playing()");

        // Now ask for both voices again. Each should be playing and each should
        // have its individual frequency.
        if let Ok(voice) = voice_store.get_voice(&u7::from(60)) {
            assert!(voice.is_playing());
            assert!(
                approx_eq!(
                    ParameterType,
                    voice.oscillator.frequency(),
                    note_to_frequency(60)
                ),
                "we should have gotten back the same voice for the requested note"
            );
        }
        if let Ok(voice) = voice_store.get_voice(&u7::from(61)) {
            assert!(voice.is_playing());
            assert!(
                approx_eq!(
                    ParameterType,
                    voice.oscillator.frequency(),
                    note_to_frequency(61)
                ),
                "we should have gotten back the same voice for the requested note"
            );
        }
        voice_store.tick(1);

        // Finally, mark a note done and then ask for a new one. We should get
        // assigned the one we just gave up.
        //
        // Note that we're taking advantage of the fact that SimpleVoice has
        // instantaneous envelope parameters, which means we can treat the
        // release as the same as the note stopping playing. For most voices
        // with nonzero release, we'd have to wait more time for the voice to
        // stop on its own. This is also why we need to spin the source_audio()
        // loop in between the two get_voice() requests; it's actually correct
        // for the system to consider a voice to still be playing after
        // release() during the same tick.
        if let Ok(voice) = voice_store.get_voice(&u7::from(60)) {
            assert!(
                approx_eq!(
                    ParameterType,
                    voice.oscillator.frequency(),
                    note_to_frequency(60)
                ),
                "we should have gotten back the same voice for the requested note"
            );
            voice.note_off(127);
        }
        voice_store.tick(1);
        if let Ok(voice) = voice_store.get_voice(&u7::from(62)) {
            // This is a bit too cute. We assume that we're getting back the
            // voice that serviced note #60 because (1) we set up the voice
            // store with only two voices, and the other one is busy, and (2) we
            // happen to know that this voice store recycles voices rather than
            // instantiating new ones. (2) is very likely to remain true for all
            // voice stores, but it's a little loosey-goosey right now.
            assert!(
                approx_eq!(
                    ParameterType,
                    voice.oscillator.frequency(),
                    note_to_frequency(60) // 60, not 62!!
                ),
                "we should have gotten the defunct voice for a new note"
            );
        } else {
            panic!("ran out of notes unexpectedly");
        }
    }
}
