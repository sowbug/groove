// Copyright (c) 2023 Mike Tsao. All rights reserved.

use eframe::{egui::Layout, emath::Align, epaint::vec2};
use groove_core::{
    time::{MusicalTime, SampleRate, Tempo, TimeSignature},
    traits::{gui::Shows, Configurable, ControlMessagesFn, Controls, HandlesMidi, Performs},
    Uid,
};
use groove_proc_macros::{Control, IsController, Uid};
use serde::{Deserialize, Serialize};
use std::ops::Range;

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

    /// The global time pointer within the song.
    #[serde(skip)]
    current_time: MusicalTime,

    #[serde(skip)]
    sample_rate: SampleRate,
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
        // Calculate the work time range. Note that we make sure the range is
        // length > 0 (via the 1.max()), which can mean that a caller relying on
        // us might get the same range twice if the sample rate is very high.
        let start = self.current_time;
        let units = 1.max(MusicalTime::frames_to_units(
            self.tempo,
            self.sample_rate,
            frames,
        ));
        let length = MusicalTime::new_with_units(units);
        let range = start..start + length;
        self.current_time += length;
        range
    }

    #[allow(missing_docs)]
    pub fn current_time(&self) -> MusicalTime {
        self.current_time
    }

    /// Renders the [Transport].
    pub fn show(&self, ui: &mut eframe::egui::Ui) {
        ui.allocate_ui(vec2(72.0, 20.0), |ui| {
            ui.set_min_width(128.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(format!("{:0.2}", self.tempo))
            });
        });
        ui.allocate_ui(vec2(72.0, 20.0), |ui| {
            ui.set_min_width(128.0);
            ui.label(format!("{}", self.current_time));
        });
    }
}
impl Shows for Transport {}
impl Configurable for Transport {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
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
        todo!()
    }

    fn stop(&mut self) {
        todo!()
    }

    fn skip_to_start(&mut self) {
        self.current_time = MusicalTime::default();
    }

    fn is_performing(&self) -> bool {
        todo!()
    }
}
impl Controls for Transport {
    fn update_time(&mut self, range: &Range<MusicalTime>) {
        // Nothing - we calculated the range, so we don't need to do anything with it.
        debug_assert!(
            self.current_time == range.end,
            "Transport::update_time() was called with the range ..{} but current_time is {}",
            range.end,
            self.current_time
        );
    }

    fn work(&mut self, _control_messages_fn: &mut ControlMessagesFn) {
        // nothing, but in the future we might want to propagate a tempo or time-sig change
    }

    fn is_finished(&self) -> bool {
        true
    }
}
