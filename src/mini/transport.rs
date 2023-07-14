// Copyright (c) 2023 Mike Tsao. All rights reserved.

use eframe::{
    egui::{Label, Layout, RichText, TextStyle, Ui},
    emath::Align,
    epaint::vec2,
};
use groove_core::{
    time::{MusicalTime, SampleRate, Tempo, TimeSignature},
    traits::{
        gui::Shows, Configurable, ControlEventsFn, Controls, HandlesMidi, Performs, Serializable,
    },
    Uid,
};
use groove_proc_macros::{Control, IsController, Uid};
use serde::{Deserialize, Serialize};
use std::ops::Range;

#[derive(Debug, Clone, Default)]
pub struct TransportEphemerals {
    /// The global time pointer within the song.
    current_time: MusicalTime,

    current_frame: usize,

    sample_rate: SampleRate,

    is_performing: bool,
}

/// [Transport] is the global clock. It knows where in the song we are, and how
/// fast time should advance.
#[derive(Serialize, Deserialize, Clone, Control, IsController, Debug, Default, Uid)]
pub struct Transport {
    uid: Uid,

    /// The current global time signature.
    time_signature: TimeSignature,

    /// The current beats per minute.
    #[control]
    tempo: Tempo,

    #[serde(skip)]
    e: TransportEphemerals,
}
impl HandlesMidi for Transport {}
impl Transport {
    /// Returns the current [Tempo].
    pub fn tempo(&self) -> Tempo {
        self.tempo
    }

    /// Sets a new [Tempo].
    pub fn set_tempo(&mut self, tempo: Tempo) {
        self.tempo = tempo;
    }

    /// Advances the clock by the given number of frames. Returns the time range
    /// from the prior time to now.
    pub fn advance(&mut self, frames: usize) -> Range<MusicalTime> {
        // Calculate the work time range. Note that the range can be zero, which
        // will happen if frames advance faster than MusicalTime units.
        let new_frames = self.e.current_frame + frames;
        let new_time = MusicalTime::new_with_frames(self.tempo, self.e.sample_rate, new_frames);
        let length = new_time - self.e.current_time;
        let range = self.e.current_time..self.e.current_time + length;

        // If we aren't performing, then we don't advance the clock, but we do
        // give devices the appearance of time moving forward by providing them
        // a (usually) nonzero time range.
        //
        // This is another reason why devices will sometimes get the same time
        // range twice. It's also why very high sample rates will make
        // MusicalTime inaccurate for devices like an arpeggiator that depend on
        // this time source *and* are supposed to operate interactively while
        // not performing (performance is stopped, but a MIDI track is selected,
        // and the user expects to hear the arp respond normally to MIDI
        // keyboard events). TODO: define a better way for these kinds of
        // devices; maybe they need a different clock that genuinely moves
        // forward (except when the performance starts). It should share the
        // same origin as the real clock, but increases regardless of
        // performance status.
        if self.is_performing() {
            self.e.current_frame = new_frames;
            self.e.current_time = new_time;
        }
        range
    }

    #[allow(missing_docs)]
    pub fn current_time(&self) -> MusicalTime {
        self.e.current_time
    }

    /// Renders the [Transport].
    pub fn show(&self, ui: &mut Ui) {
        ui.allocate_ui(vec2(72.0, 20.0), |ui| {
            ui.set_min_width(128.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add(Label::new(
                    RichText::new(format!("{:0.2}", self.tempo)).text_style(TextStyle::Monospace),
                ));
            });
        });
        ui.allocate_ui(vec2(72.0, 20.0), |ui| {
            ui.set_min_width(128.0);
            ui.add(Label::new(
                RichText::new(format!("{}", self.e.current_time)).text_style(TextStyle::Monospace),
            ));
        });
    }

    /// The current sample rate.
    // TODO: should this be part of the Configurable trait?
    pub fn sample_rate(&self) -> SampleRate {
        self.e.sample_rate
    }
}
impl Shows for Transport {}
impl Serializable for Transport {}
impl Configurable for Transport {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.e.sample_rate = sample_rate;
    }

    fn update_tempo(&mut self, tempo: Tempo) {
        self.tempo = tempo;
    }

    fn update_time_signature(&mut self, time_signature: TimeSignature) {
        self.time_signature = time_signature;
    }
}
impl Performs for Transport {
    fn play(&mut self) {
        self.e.is_performing = true;
    }

    fn stop(&mut self) {
        self.e.is_performing = false;
    }

    fn skip_to_start(&mut self) {
        self.e.current_time = MusicalTime::default();
        self.e.current_frame = Default::default();
    }

    fn is_performing(&self) -> bool {
        self.e.is_performing
    }
}
impl Controls for Transport {
    fn update_time(&mut self, range: &Range<MusicalTime>) {
        // Nothing - we calculated the range, so we don't need to do anything with it.
        debug_assert!(
            self.e.current_time == range.end,
            "Transport::update_time() was called with the range ..{} but current_time is {}",
            range.end,
            self.e.current_time
        );
    }

    fn work(&mut self, _control_events_fn: &mut ControlEventsFn) {
        // nothing, but in the future we might want to propagate a tempo or time-sig change
    }

    fn is_finished(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advances_time_correctly_with_various_sample_rates() {
        let mut transport = Transport::default();
        transport.update_tempo(Tempo(60.0));

        let vec = vec![100, 997, 22050, 44100, 48000, 88200, 98689, 100000, 262144];
        for sample_rate in vec {
            transport.play();
            transport.update_sample_rate(SampleRate(sample_rate));

            let mut time_range_covered = 0;
            for _ in 0..transport.sample_rate().0 {
                let range = transport.advance(1);
                let delta_units = (range.end - range.start).total_units();
                time_range_covered += delta_units;
            }
            assert_eq!(time_range_covered, MusicalTime::UNITS_IN_BEAT,
            "Sample rate {} Hz: after advancing one second of frames at 60 BPM, we should have covered {} MusicalTime units",
            sample_rate, MusicalTime::UNITS_IN_BEAT);

            assert_eq!(
                transport.current_time(),
                MusicalTime::new_with_beats(1),
                "Transport should be exactly on the one-beat mark."
            );

            // We put this at the end of the loop rather than the start because
            // we'd like to test that the initial post-new state is correct
            // without first calling skip_to_start().
            transport.skip_to_start();
        }
    }
}
