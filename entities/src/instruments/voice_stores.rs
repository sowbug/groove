// Copyright (c) 2023 Mike Tsao. All rights reserved.

use anyhow::{anyhow, Result};
use groove_core::{
    midi::u7,
    traits::{Generates, IsStereoSampleVoice, Resets, StoresVoices, Ticks},
    StereoSample,
};
use rustc_hash::FxHashMap;

/// A [StoresVoices](groove_core::traits::StoresVoices) that fails when too many
/// voices are used simultaneously.
#[derive(Debug)]
pub struct VoiceStore<V: IsStereoSampleVoice> {
    sample: StereoSample,
    voices: Vec<Box<V>>,
    notes_playing: Vec<u7>,
}
impl<V: IsStereoSampleVoice> StoresVoices for VoiceStore<V> {
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
            if voice.is_playing() {
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
impl<V: IsStereoSampleVoice> Generates<StereoSample> for VoiceStore<V> {
    fn value(&self) -> StereoSample {
        self.sample
    }

    #[allow(unused_variables)]
    fn batch_values(&mut self, values: &mut [StereoSample]) {
        todo!()
    }
}
impl<V: IsStereoSampleVoice> Resets for VoiceStore<V> {
    fn reset(&mut self, sample_rate: usize) {
        self.voices.iter_mut().for_each(|v| v.reset(sample_rate));
    }
}
impl<V: IsStereoSampleVoice> Ticks for VoiceStore<V> {
    // TODO: this is not at all taking advantage of batching. When
    // batch_sample() calls it, it's lame.
    fn tick(&mut self, tick_count: usize) {
        self.voices.iter_mut().for_each(|v| v.tick(tick_count));
        self.sample = self.voices.iter().map(|v| v.value()).sum();
        self.voices.iter().enumerate().for_each(|(index, voice)| {
            if !voice.is_playing() {
                self.notes_playing[index] = u7::from(0);
            }
        });
    }
}
impl<V: IsStereoSampleVoice> VoiceStore<V> {
    #[deprecated]
    pub(crate) fn new_with(_sample_rate: usize) -> Self {
        Self {
            sample: Default::default(),
            voices: Default::default(),
            notes_playing: Default::default(),
        }
    }
    #[deprecated]
    pub(crate) fn add_voice(&mut self, voice: Box<V>) {
        self.voices.push(voice);
        self.notes_playing.push(u7::from(0));
    }

    pub fn new_with_voice<F>(sample_rate: usize, voice_capacity: usize, new_voice_fn: F) -> Self
    where
        F: Fn() -> V,
    {
        let mut voice_store = Self::new_with(sample_rate);
        for _ in 0..voice_capacity {
            voice_store.add_voice(Box::new(new_voice_fn()));
        }
        voice_store
    }
}

/// A [StoresVoices](groove_core::traits::StoresVoices) that steals voices as
/// needed.
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
            if voice.is_playing() {
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
            if !voice.is_playing() {
                self.notes_playing[index] = u7::from(0);
            }
        });
    }
}
impl<V: IsStereoSampleVoice> StealingVoiceStore<V> {
    #[deprecated(note = "private use is OK. Prefer new_with_voice instead")]
    fn new_with(_sample_rate: usize) -> Self {
        Self {
            sample: Default::default(),
            voices: Default::default(),
            notes_playing: Default::default(),
        }
    }

    pub fn new_with_voice<F>(sample_rate: usize, voice_capacity: usize, new_voice_fn: F) -> Self
    where
        F: Fn() -> V,
    {
        let mut voice_store = Self::new_with(sample_rate);
        for _ in 0..voice_capacity {
            voice_store.add_voice(Box::new(new_voice_fn()));
        }
        voice_store
    }

    #[deprecated(
        note = "private use is OK. Prefer new_with_voice instead, which calls add_voice for you"
    )]
    fn add_voice(&mut self, voice: Box<V>) {
        self.voices.push(voice);
        self.notes_playing.push(u7::from(0));
    }
}

/// A [StoresVoices](groove_core::traits::StoresVoices) that assumes a specific
/// voice is dedicated to each note. A good example is a drumkit sampler, which
/// uses the same [Voice] whenever a particular sample is played..
#[derive(Debug)]
pub struct VoicePerNoteStore<V: IsStereoSampleVoice> {
    sample: StereoSample,
    voices: FxHashMap<u7, Box<V>>,
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
    #[deprecated]
    pub fn new_with(_sample_rate: usize) -> Self {
        Self {
            sample: Default::default(),
            voices: Default::default(),
        }
    }

    #[deprecated]
    pub fn add_voice(&mut self, key: u7, voice: Box<V>) {
        self.voices.insert(key, voice);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        instruments::{SimpleVoice, StealingVoiceStore, VoiceStore},
        tests::DEFAULT_SAMPLE_RATE,
    };
    use float_cmp::approx_eq;
    use groove_core::{
        midi::{note_to_frequency, u7},
        traits::{PlaysNotes, StoresVoices, Ticks},
        ParameterType,
    };
    use more_asserts::assert_gt;

    #[test]
    fn simple_voice_store_mainline() {
        let mut voice_store =
            VoiceStore::<SimpleVoice>::new_with_voice(DEFAULT_SAMPLE_RATE, 2, || {
                SimpleVoice::new_with(DEFAULT_SAMPLE_RATE)
            });
        assert_gt!(!voice_store.voice_count(), 0);
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
        let mut voice_store =
            StealingVoiceStore::<SimpleVoice>::new_with_voice(DEFAULT_SAMPLE_RATE, 2, || {
                SimpleVoice::new_with(DEFAULT_SAMPLE_RATE)
            });
        assert_gt!(voice_store.voice_count(), 0);
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
        let mut voice_store =
            VoiceStore::<SimpleVoice>::new_with_voice(DEFAULT_SAMPLE_RATE, 2, || {
                SimpleVoice::new_with(DEFAULT_SAMPLE_RATE)
            });
        assert_gt!(voice_store.voice_count(), 0);
        assert_eq!(voice_store.active_voice_count(), 0);

        // Request multiple voices during the same tick.
        if let Ok(voice) = voice_store.get_voice(&u7::from(60)) {
            voice.note_on(60, 127);
            assert!(
                voice.is_playing(),
                "New voice should be marked is_playing() immediately after attack()"
            );
        }
        if let Ok(voice) = voice_store.get_voice(&u7::from(61)) {
            voice.note_on(61, 127);
            assert!(
                voice.is_playing(),
                "New voice should be marked is_playing() immediately after attack()"
            );
        }

        voice_store.tick(1);
        assert_eq!(voice_store.active_voice_count(), 2, "voices with pending attacks() should have been handled, and they should now be is_playing()");

        // Now ask for both voices again. Each should be playing and each should
        // have its individual frequency.
        if let Ok(voice) = voice_store.get_voice(&u7::from(60)) {
            assert!(voice.is_playing());
            assert!(
                approx_eq!(
                    ParameterType,
                    voice.debug_oscillator_frequency(),
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
                    voice.debug_oscillator_frequency(),
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
                    voice.debug_oscillator_frequency(),
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
                    voice.debug_oscillator_frequency(),
                    note_to_frequency(60) // 60, not 62!!
                ),
                "we should have gotten the defunct voice for a new note"
            );
        } else {
            panic!("ran out of notes unexpectedly");
        }
    }
}
