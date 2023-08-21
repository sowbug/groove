// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    control::ControlValue,
    time::MusicalTime,
    traits::{
        gui::Shows, Configurable, ControlEventsFn, Controls, HandlesMidi, Serializable, ThingEvent,
    },
    Normal, Uid,
};
use groove_proc_macros::{IsController, Uid};
use serde::{Deserialize, Serialize};
use std::ops::{Range, RangeInclusive};

/// A [ControlTrip] is a single track of automation. It can run as long as the
/// whole song.
#[derive(Serialize, Deserialize, Debug, Default, IsController, Uid)]
pub struct ControlTrip {
    uid: Uid,
    pub the_one_step: ControlStep, // HACK just one step for now

    #[serde(skip)]
    range: Range<MusicalTime>,
    #[serde(skip)]
    last_published_value: f64,
}
impl Shows for ControlTrip {}
impl HandlesMidi for ControlTrip {}
impl Controls for ControlTrip {
    fn update_time(&mut self, range: &Range<MusicalTime>) {
        self.range = range.clone();
    }

    fn work(&mut self, control_events_fn: &mut ControlEventsFn) {
        if self.range.start >= self.the_one_step.time_range.end
            || self.range.end <= self.the_one_step.time_range.start
        {
            return;
        }
        let current_point = self.range.start.total_units() as f64;
        let start = self.the_one_step.time_range.start.total_units() as f64;
        let end = self.the_one_step.time_range.end.total_units() as f64;
        let duration = end - start;
        let current_point = current_point - start;
        let percentage = if duration > 0.0 {
            current_point / duration
        } else {
            0.0
        };
        let current_value = self.the_one_step.value_range.start().0
            + percentage
                * (self.the_one_step.value_range.end().0 - self.the_one_step.value_range.start().0);
        if current_value != self.last_published_value {
            self.last_published_value = current_value;
            control_events_fn(
                self.uid,
                ThingEvent::Control(ControlValue::from(current_value)),
            );
        }
    }

    fn is_finished(&self) -> bool {
        self.range.start >= self.the_one_step.time_range.end
    }

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
        todo!()
    }
}
impl Configurable for ControlTrip {}
impl Serializable for ControlTrip {}
impl ControlTrip {
    pub fn new_with(value_range: RangeInclusive<Normal>, time_range: Range<MusicalTime>) -> Self {
        Self {
            uid: Default::default(),
            the_one_step: ControlStep {
                value_range,
                time_range,
            },
            ..Default::default()
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ControlStep {
    value_range: RangeInclusive<Normal>,
    time_range: Range<MusicalTime>,
}
impl Default for ControlStep {
    fn default() -> Self {
        Self {
            value_range: Normal::from(1.0)..=Normal::from(1.0),
            // The default time range is zero.
            time_range: MusicalTime::START..MusicalTime::START,
        }
    }
}

/// A [ControlAtlas] manages a group of [ControlTrip]s. (An atlas is a book of
/// maps.)
#[derive(Serialize, Deserialize, IsController, Debug, Uid, Default)]
pub struct ControlAtlas {
    uid: Uid,
    trips: Vec<ControlTrip>,
    #[serde(skip)]
    range: Range<MusicalTime>,
}
impl Shows for ControlAtlas {}
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
        self.trips.iter().all(|ct| ct.is_finished())
    }

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
impl Serializable for ControlAtlas {}
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
impl ControlAtlas {
    pub fn add_trip(&mut self, trip: ControlTrip) {
        self.trips.push(trip);
    }
}
