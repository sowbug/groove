use crate::{
    common::{BipolarNormal, F32ControlValue, Sample, StereoSample},
    midi::MidiUtils,
    settings::patches::EnvelopeSettings,
    traits::{Controllable, HasUid, IsInstrument, SourcesAudio},
    Clock, Oscillator,
};
use anyhow::{anyhow, Result};
use groove_macros::{Control, Uid};
use midly::{num::u7, MidiMessage};
use std::str::FromStr;
use std::{collections::HashMap, fmt::Debug};
use strum_macros::{Display, EnumString, FromRepr};

use self::envelopes::{GeneratesEnvelope, SimpleEnvelope};

pub(crate) mod drumkit_sampler;
pub(crate) mod envelopes;
pub(crate) mod oscillators;
pub(crate) mod sampler;
pub(crate) mod welsh;

pub trait HandlesMidi {
    #[allow(unused_variables)]
    fn handle_midi_message(&mut self, message: &MidiMessage) {}
}

/// As an experiment, we're going to define PlaysNotes as a different interface
/// from HandlesMidi. This will give the HandlesMidi Synthesizer an opportunity
/// to manage more about the note lifecycle, including concepts like glide
/// (which I believe needs a higher-level definition of a "note" than just MIDI
/// on/off)
pub trait PlaysNotes {
    fn is_playing(&self) -> bool;
    fn are_events_pending(&self) -> bool;
    fn set_frequency_hz(&mut self, frequency_hz: f32);
    fn enqueue_note_on(&mut self, velocity: u8);
    fn enqueue_aftertouch(&mut self, velocity: u8);
    fn enqueue_note_off(&mut self, velocity: u8);
    fn set_pan(&mut self, value: f32);
}

// TODO: I didn't want StoresVoices to know anything about audio (i.e.,
// SourcesAudio), but I couldn't figure out how to return an IterMut from a
// HashMap, so I couldn't define a trait method that allowed the implementation
// to return an iterator from either a Vec or a HashMap.
pub trait StoresVoices: SourcesAudio + Send + Debug {
    type Voice;

    /// Generally, this value won't change after initialization, because we try
    /// not to dynamically allocate new voices.
    fn voice_count(&self) -> usize;

    /// The number of voices reporting is_playing() true. Notably, this excludes
    /// any voice with pending events. So if you call attack() on a voice in the
    /// store but don't tick it, the voice-store active number won't include it.
    fn active_voice_count(&self) -> usize;

    /// Fails if we run out of idle voices.
    fn get_voice(&mut self, key: &midly::num::u7) -> Result<&mut Box<Self::Voice>>;

    /// Uh-oh, StoresVoices is turning into a synth
    fn set_pan(&mut self, value: f32);
}

/// A synthesizer is composed of Voices. Ideally, a synth will know how to
/// construct Voices, and then handle all the MIDI events properly for them.
pub trait IsVoice: SourcesAudio + PlaysNotes + Send + Default {}

#[derive(Control, Debug, Uid)]
pub struct Synthesizer<V: IsVoice> {
    uid: usize,

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
impl<V: IsVoice> IsInstrument for Synthesizer<V> {}
impl<V: IsVoice> SourcesAudio for Synthesizer<V> {
    fn source_audio(&mut self, clock: &Clock) -> StereoSample {
        self.voice_store.source_audio(clock)
    }
}

impl<V: IsVoice> Synthesizer<V> {
    fn new_with(voice_store: Box<dyn StoresVoices<Voice = V>>) -> Self {
        Self {
            uid: Default::default(),
            voice_store: voice_store,
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
impl<V: IsVoice> HandlesMidi for Synthesizer<V> {
    fn handle_midi_message(&mut self, message: &MidiMessage) {
        match message {
            MidiMessage::NoteOff { key, vel } => {
                if let Ok(voice) = self.voice_store.get_voice(key) {
                    voice.enqueue_note_off(vel.as_int());
                }
            }
            MidiMessage::NoteOn { key, vel } => {
                if let Ok(voice) = self.voice_store.get_voice(key) {
                    voice.set_frequency_hz(MidiUtils::note_to_frequency(key.as_int()));
                    voice.enqueue_note_on(vel.as_int());
                }
            }
            MidiMessage::Aftertouch { key, vel } => {
                if let Ok(voice) = self.voice_store.get_voice(key) {
                    voice.enqueue_aftertouch(vel.as_int());
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
    envelope: SimpleEnvelope,

    is_playing: bool,
    note_on_is_pending: bool,
    note_on_velocity: u8,
    note_off_is_pending: bool,
    note_off_velocity: u8,
    aftertouch_is_pending: bool,
    aftertouch_velocity: u8,
}
impl IsVoice for SimpleVoice {}
impl PlaysNotes for SimpleVoice {
    fn is_playing(&self) -> bool {
        self.is_playing
    }

    fn are_events_pending(&self) -> bool {
        self.note_on_is_pending || self.note_off_is_pending || self.aftertouch_is_pending
    }

    fn set_frequency_hz(&mut self, frequency_hz: f32) {
        self.oscillator.set_frequency(frequency_hz);
    }

    fn enqueue_note_on(&mut self, velocity: u8) {
        self.note_on_is_pending = true;
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

    fn set_pan(&mut self, _value: f32) {
        // We don't handle this.
    }
}
impl SourcesAudio for SimpleVoice {
    fn source_audio(&mut self, clock: &Clock) -> StereoSample {
        self.handle_pending_note_events();
        let r = self.oscillator.source_signal(clock).value() * self.envelope.tick(clock).value();
        self.is_playing = !self.envelope.is_idle();
        StereoSample::from(r)
    }
}

impl SimpleVoice {
    fn handle_pending_note_events(&mut self) {
        if self.note_on_is_pending && self.note_off_is_pending {
            // Handle the case where both are pending at the same time.
            if self.is_playing {
                self.handle_note_off_event();
                self.handle_note_on_event();
            } else {
                self.handle_note_on_event();
                self.handle_note_off_event();
            }
        } else {
            if self.note_off_is_pending {
                self.handle_note_off_event();
            }
            if self.note_on_is_pending {
                self.handle_note_on_event();
            }
        }
        if self.aftertouch_is_pending {
            self.handle_aftertouch_event();
        }
    }

    fn handle_note_on_event(&mut self) {
        self.note_on_is_pending = false;
        self.envelope.enqueue_attack();
    }

    fn handle_aftertouch_event(&mut self) {
        // TODO: do something
        self.aftertouch_is_pending = false;
    }

    fn handle_note_off_event(&mut self) {
        self.note_off_is_pending = false;
        self.envelope.enqueue_release();
    }
}

#[derive(Control, Debug, Uid)]
pub struct SimpleSynthesizer {
    uid: usize,
    inner_synth: Synthesizer<SimpleVoice>,
}
impl IsInstrument for SimpleSynthesizer {}
impl HandlesMidi for SimpleSynthesizer {
    fn handle_midi_message(&mut self, message: &MidiMessage) {
        self.inner_synth.handle_midi_message(&message);
    }
}
impl SourcesAudio for SimpleSynthesizer {
    fn source_audio(&mut self, clock: &Clock) -> StereoSample {
        self.inner_synth.source_audio(clock)
    }
}
impl Default for SimpleSynthesizer {
    fn default() -> Self {
        let mut voice_store = Box::new(SimpleVoiceStore::<SimpleVoice>::default());
        for _ in 0..4 {
            voice_store.add_voice(Box::new(SimpleVoice::default()));
        }
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
impl<V: IsVoice> StoresVoices for SimpleVoiceStore<V> {
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
            if voice.is_playing() || voice.are_events_pending() {
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
impl<V: IsVoice> SourcesAudio for SimpleVoiceStore<V> {
    fn source_audio(&mut self, clock: &Clock) -> StereoSample {
        let r = self.voices.iter_mut().map(|v| v.source_audio(clock)).sum();
        for (index, voice) in self.voices.iter().enumerate() {
            if !voice.is_playing() {
                self.notes_playing[index] = u7::from(0);
            }
        }
        r
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
impl<V: IsVoice> StoresVoices for VoicePerNoteStore<V> {
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

    fn set_pan(&mut self, value: f32) {
        for voice in self.voices.iter_mut() {
            voice.1.set_pan(value);
        }
    }
}
impl<V: IsVoice> SourcesAudio for VoicePerNoteStore<V> {
    fn source_audio(&mut self, clock: &Clock) -> StereoSample {
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
    envelope: SimpleEnvelope,
    dca: Dca,

    is_playing: bool,
    note_on_is_pending: bool,
    note_on_velocity: u8,
    note_off_is_pending: bool,
    note_off_velocity: u8,
    aftertouch_is_pending: bool,
    aftertouch_velocity: u8,
}
impl IsVoice for FmVoice {}
impl PlaysNotes for FmVoice {
    fn is_playing(&self) -> bool {
        self.is_playing
    }

    fn are_events_pending(&self) -> bool {
        self.note_on_is_pending || self.note_off_is_pending || self.aftertouch_is_pending
    }

    fn set_frequency_hz(&mut self, frequency_hz: f32) {
        self.carrier.set_frequency(frequency_hz);
    }

    fn enqueue_note_on(&mut self, velocity: u8) {
        self.note_on_is_pending = true;
        self.note_on_velocity = velocity;
        self.envelope.enqueue_attack();
    }

    fn enqueue_aftertouch(&mut self, velocity: u8) {
        self.aftertouch_is_pending = true;
        self.aftertouch_velocity = velocity;
    }

    fn enqueue_note_off(&mut self, velocity: u8) {
        self.note_off_is_pending = true;
        self.note_off_velocity = velocity;
        self.envelope.enqueue_release();
    }

    fn set_pan(&mut self, value: f32) {
        self.dca.set_pan(BipolarNormal::from(value));
    }
}
impl SourcesAudio for FmVoice {
    fn source_audio(&mut self, clock: &Clock) -> StereoSample {
        self.handle_pending_note_events();
        self.carrier.set_frequency_modulation(
            self.modulator.source_signal(clock).value() as f32 * self.modulator_depth,
        );
        let r = self.carrier.source_signal(clock).value() * self.envelope.tick(clock).value();
        self.is_playing = !self.envelope.is_idle();
        self.dca.transform_audio_to_stereo(clock, Sample(r))
    }
}
impl Default for FmVoice {
    fn default() -> Self {
        Self {
            carrier: Default::default(),
            modulator: Default::default(),
            modulator_depth: 0.2,
            envelope: SimpleEnvelope::new_with(
                Clock::default().sample_rate(),
                &EnvelopeSettings {
                    attack: 0.1,
                    decay: 0.1,
                    sustain: 0.8,
                    release: 0.25,
                },
            ),
            dca: Default::default(),
            is_playing: Default::default(),
            note_on_is_pending: Default::default(),
            note_on_velocity: Default::default(),
            note_off_is_pending: Default::default(),
            note_off_velocity: Default::default(),
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
    fn handle_pending_note_events(&mut self) {
        if self.note_on_is_pending && self.note_off_is_pending {
            // Handle the case where both are pending at the same time.
            if self.is_playing {
                self.handle_note_off_event();
                self.handle_note_on_event();
            } else {
                self.handle_note_on_event();
                self.handle_note_off_event();
            }
        } else {
            if self.note_off_is_pending {
                self.handle_note_off_event();
            }
            if self.note_on_is_pending {
                self.handle_note_on_event();
            }
        }
        if self.aftertouch_is_pending {
            self.handle_aftertouch_event();
        }
    }

    fn handle_note_on_event(&mut self) {
        self.note_on_is_pending = false;
    }

    fn handle_aftertouch_event(&mut self) {
        // TODO: do something
        self.aftertouch_is_pending = false;
    }

    fn handle_note_off_event(&mut self) {
        self.note_off_is_pending = false;
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
impl HandlesMidi for FmSynthesizer {
    fn handle_midi_message(&mut self, message: &MidiMessage) {
        self.inner_synth.handle_midi_message(&message)
    }
}
impl SourcesAudio for FmSynthesizer {
    fn source_audio(&mut self, clock: &Clock) -> StereoSample {
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

    pub(crate) fn new_with_voice_store(
        voice_store: Box<dyn StoresVoices<Voice = FmVoice>>,
    ) -> Self {
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

    pub(crate) fn transform_audio_to_stereo(
        &mut self,
        _clock: &Clock,
        input_sample: Sample,
    ) -> StereoSample {
        // See Pirkle, DSSPC++, p.73
        let input_sample: f64 = input_sample.0 * self.gain;
        let left_pan: f64 = 1.0 - 0.25 * (self.pan + 1.0).powi(2);
        let right_pan: f64 = 1.0 - (0.5 * self.pan - 0.5).powi(2);
        StereoSample::new_from_f64(left_pan * input_sample, right_pan * input_sample)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dca_mainline() {
        let mut dca = Dca::default();
        let clock = Clock::default();
        const VALUE_IN: Sample = Sample(0.5);
        const VALUE: f64 = 0.5;
        assert_eq!(
            dca.transform_audio_to_stereo(&clock, VALUE_IN),
            StereoSample::new_from_f64(VALUE * 0.75, VALUE * 0.75),
            "Pan center should give 75% equally to each channel"
        );

        dca.set_pan(BipolarNormal::new(-1.0));
        assert_eq!(
            dca.transform_audio_to_stereo(&clock, VALUE_IN),
            StereoSample::new_from_f64(VALUE, 0.0),
            "Pan left should give 100% to left channel"
        );

        dca.set_pan(BipolarNormal::new(1.0));
        assert_eq!(
            dca.transform_audio_to_stereo(&clock, VALUE_IN),
            StereoSample::new_from_f64(0.0, VALUE),
            "Pan right should give 100% to right channel"
        );
    }

    #[test]
    fn voice_store_mainline() {
        let mut voice_store = SimpleVoiceStore::<SimpleVoice>::default();
        assert_eq!(voice_store.voice_count(), 0);
        assert_eq!(voice_store.active_voice_count(), 0);

        for _ in 0..2 {
            voice_store.add_voice(Box::new(SimpleVoice::default()));
        }
        assert_eq!(voice_store.voice_count(), 2);
        assert_eq!(voice_store.active_voice_count(), 0);

        let clock = Clock::default();

        // Request and start the maximum number of voices.
        if let Ok(voice) = voice_store.get_voice(&u7::from(60)) {
            assert!(!voice.is_playing());
            voice.enqueue_note_on(127);
            voice.source_audio(&clock); // We must ask for the sample to register the trigger.
            assert!(voice.is_playing());
        }
        if let Ok(voice) = voice_store.get_voice(&u7::from(61)) {
            voice.enqueue_note_on(127);
            voice.source_audio(&clock);
        }

        // Request a voice for a new note that would exceed the count. Should
        // fail.
        assert!(voice_store.get_voice(&u7::from(62)).is_err());

        // Request to get back a voice that's already playing.
        if let Ok(voice) = voice_store.get_voice(&u7::from(60)) {
            assert!(voice.is_playing());
            voice.enqueue_note_off(127);

            // All SimpleVoice envelope times are instantaneous, so we know the
            // release completes after asking for the next sample.
            voice.source_audio(&clock);
            assert!(!voice.is_playing());
        }
    }

    #[test]
    fn voice_store_simultaneous_events() {
        let mut voice_store = SimpleVoiceStore::<SimpleVoice>::default();
        assert_eq!(voice_store.voice_count(), 0);
        assert_eq!(voice_store.active_voice_count(), 0);

        for _ in 0..2 {
            voice_store.add_voice(Box::new(SimpleVoice::default()));
        }
        assert_eq!(voice_store.voice_count(), 2);
        assert_eq!(voice_store.active_voice_count(), 0);

        let clock = Clock::default();

        // Request multiple voices during the same tick.
        if let Ok(voice) = voice_store.get_voice(&u7::from(60)) {
            // this frequency is not correct for the MIDI note, but it's just to
            // disambiguate, so that's OK.
            voice.oscillator.set_frequency(60.0);
            voice.enqueue_note_on(127);
            assert!(!voice.is_playing(), "New voice shouldn't be marked is_playing() until both attack() and the next source_audio() have completed");
        }
        if let Ok(voice) = voice_store.get_voice(&u7::from(61)) {
            voice.oscillator.set_frequency(61.0);
            voice.enqueue_note_on(127);
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
        let _ = voice_store.source_audio(&clock);
        assert_eq!(voice_store.active_voice_count(), 2, "voices with pending attacks() should have been handled, and they should now be is_playing()");

        // Now ask for both voices again. Each should be playing and each should
        // have its individual frequency.
        if let Ok(voice) = voice_store.get_voice(&u7::from(60)) {
            assert!(voice.is_playing());
            assert_eq!(
                voice.oscillator.frequency(),
                60.0,
                "we should have gotten back the same voice for the requested note"
            );
        }
        if let Ok(voice) = voice_store.get_voice(&u7::from(61)) {
            assert!(voice.is_playing());
            assert_eq!(
                voice.oscillator.frequency(),
                61.0,
                "we should have gotten back the same voice for the requested note"
            );
        }
        let _ = voice_store.source_audio(&clock);

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
            assert_eq!(
                voice.oscillator.frequency(),
                60.0,
                "we should have gotten back the same voice for the requested note"
            );
            voice.enqueue_note_off(127);
        }
        let _ = voice_store.source_audio(&clock);
        if let Ok(voice) = voice_store.get_voice(&u7::from(62)) {
            // This is a bit too cute. We assume that we're getting back the
            // voice that serviced note #60 because (1) we set up the voice
            // store with only two voices, and the other one is busy, and (2) we
            // happen to know that this voice store recycles voices rather than
            // instantiating new ones. (2) is very likely to remain true for all
            // voice stores, but it's a little loosey-goosey right now.
            assert_eq!(
                voice.oscillator.frequency(),
                60.0,
                "we should have gotten the defunct voice for a new note"
            );
        } else {
            panic!("ran out of notes unexpectedly");
        }
    }
}
