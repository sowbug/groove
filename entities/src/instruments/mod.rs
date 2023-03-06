// Copyright (c) 2023 Mike Tsao. All rights reserved.

pub use drumkit::Drumkit;
pub use fm::FmSynthesizer;
pub use fm::FmVoice;
pub use sampler::Sampler;
pub use sampler::SamplerControlParams;
pub use synthesizer::SimpleSynthesizer;
pub use synthesizer::SimpleVoice;
pub use voice_stores::StealingVoiceStore;
pub use voice_stores::VoiceStore;
pub use welsh::LfoRouting;
pub use welsh::WelshSynth;
pub use welsh::WelshVoice;

//pub(crate) use synthesizer::Synthesizer;

mod drumkit;
mod envelopes;
mod fm;
mod sampler;
mod synthesizer;
mod voice_stores;
mod welsh;

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
