// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    time::MusicalTime,
    traits::{gui::Shows, Configurable, ControlEventsFn, Controls, HandlesMidi, Performs},
    Uid,
};
use groove_entities::controllers::ControlTrip;
use groove_proc_macros::{IsController, Uid};
use serde::{Deserialize, Serialize};
use std::ops::Range;

/// A [ControlAtlas] manages a group of [ControlTrip]s. (An atlas is a book of
/// maps.)
#[derive(Serialize, Deserialize, IsController, Debug, Uid)]
pub struct ControlAtlas {
    uid: Uid,
    trips: Vec<ControlTrip>,
    #[serde(skip)]
    range: Range<MusicalTime>,
}
impl Shows for ControlAtlas {}
impl Performs for ControlAtlas {
    fn play(&mut self) {
        todo!()
    }

    fn stop(&mut self) {
        todo!()
    }

    fn skip_to_start(&mut self) {
        todo!()
    }

    fn is_performing(&self) -> bool {
        false
    }
}
impl HandlesMidi for ControlAtlas {}
impl Controls for ControlAtlas {
    fn update_time(&mut self, range: &Range<MusicalTime>) {
        self.range = range.clone();
        self.trips.iter_mut().for_each(|t| t.update_time(range));
    }

    fn work(&mut self, control_events_fn: &mut ControlEventsFn) {
        self.trips
            .iter_mut()
            .for_each(|t| t.work(control_events_fn));
    }

    fn is_finished(&self) -> bool {
        self.trips.iter().all(|t| t.is_finished())
    }
}
impl Configurable for ControlAtlas {
    fn update_sample_rate(&mut self, sample_rate: groove_core::time::SampleRate) {
        self.trips
            .iter_mut()
            .for_each(|t| t.update_sample_rate(sample_rate));
    }

    fn update_tempo(&mut self, tempo: groove_core::time::Tempo) {
        self.trips.iter_mut().for_each(|t| t.update_tempo(tempo));
    }

    fn update_time_signature(&mut self, time_signature: groove_core::time::TimeSignature) {
        self.trips
            .iter_mut()
            .for_each(|t| t.update_time_signature(time_signature));
    }
}
