use crate::{
    common::{F32ControlValue, MonoSample},
    midi::MidiUtils,
    traits::{Controllable, HasUid, IsInstrument, SourcesAudio, Updateable},
    AdsrEnvelope, Clock, EntityMessage, Oscillator,
};
use anyhow::anyhow;
use groove_macros::{Control, Uid};
use midly::{num::u7, MidiMessage};
use std::str::FromStr;
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

/// A synthesizer is composed of Voices. Ideally, a synth will know how to
/// construct Voices, and then handle all the MIDI events properly for them.
pub trait IsVoice: SourcesAudio + PlaysNotes {}

#[derive(Control, Debug, Uid)]
pub struct Synthesizer<V: IsVoice> {
    uid: usize,

    // These two fields must always have the same number of elements in their
    // Vecs.
    voices: Vec<Box<V>>,
    notes_playing: Vec<u7>,

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
        self.voices.iter_mut().map(|v| v.source_audio(clock)).sum()
    }
}
impl<V: IsVoice> Default for Synthesizer<V> {
    fn default() -> Self {
        Self {
            uid: Default::default(),
            voices: Default::default(),
            notes_playing: Default::default(),
            pitch_bend: Default::default(),
            channel_aftertouch: Default::default(),
        }
    }
}

impl<V: IsVoice> Synthesizer<V> {
    fn add_voice(&mut self, voice: Box<V>) {
        self.voices.push(voice);
        self.notes_playing.push(u7::from(0));
    }

    fn get_voice(&mut self, key: &midly::num::u7) -> anyhow::Result<usize> {
        if let Some(index) = self.notes_playing.iter().position(|note| *key == *note) {
            return Ok(index);
        }
        for (index, voice) in self.voices.iter().enumerate() {
            if voice.is_playing() {
                continue;
            }
            self.notes_playing[index] = *key;
            return Ok(index);
        }
        Err(anyhow!("out of voices"))
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
                if let Ok(index) = self.get_voice(key) {
                    (&mut self.voices[index]).release(vel.as_int());
                }
            }
            MidiMessage::NoteOn { key, vel } => {
                if let Ok(index) = self.get_voice(key) {
                    (&mut self.voices[index])
                        .set_frequency_hz(MidiUtils::note_to_frequency(key.as_int()));
                    (&mut self.voices[index]).attack(vel.as_int());
                }
            }
            MidiMessage::Aftertouch { key, vel } => {
                if let Ok(index) = self.get_voice(key) {
                    (&mut self.voices[index]).aftertouch(vel.as_int());
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
        self.is_playing = !self.envelope.is_idle(clock);
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
        let mut r = Self {
            uid: Default::default(),
            inner_synth: Synthesizer::<SimpleVoice>::default(),
        };
        r.inner_synth.add_voice(Box::new(SimpleVoice::default()));
        r.inner_synth.add_voice(Box::new(SimpleVoice::default()));
        r.inner_synth.add_voice(Box::new(SimpleVoice::default()));
        r.inner_synth.add_voice(Box::new(SimpleVoice::default()));
        r
    }
}
impl SimpleSynthesizer {
    pub fn notes_playing(&self) -> usize {
        0
    }
}
