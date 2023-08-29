// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::EntityFactory;
use derive_builder::Builder;
use eframe::{
    egui::Sense,
    emath::RectTransform,
    epaint::{pos2, vec2, Color32, Rect, Stroke},
};
use groove_core::{
    control::ControlValue,
    time::MusicalTime,
    traits::{
        gui::Shows, Configurable, ControlEventsFn, Controls, HandlesMidi, HasUid, Serializable,
        ThingEvent,
    },
    Uid,
};
use groove_proc_macros::{IsController, Uid};
use serde::{Deserialize, Serialize};
use std::ops::{Range, RangeInclusive};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
/// Specifies what a [ControlStep]'s path should look like.
pub enum ControlPath {
    /// No path. This step's value should be ignored.
    #[default]
    None,
    /// Stairstep. The path should be level at the [ControlStep]'s value.
    Flat,
    /// Linear. Straight line from this [ControlStep]'s value to the next one.
    Linear,
    /// Curved. Starts out changing quickly and ends up changing slowly.
    Logarithmic,
    /// Curved. Starts out changing slowly and ends up changing quickly.
    Exponential,
}

#[derive(Debug)]
pub struct ControlTripEphemerals {
    /// The time range for this work slice. This is a copy of the value passed
    /// in Controls::update_time().
    range: Range<MusicalTime>,

    /// Which step we're currently processing.
    current_step: usize,
    /// The type of path we should be following.
    current_path: ControlPath,
    /// The range of values for the current step.
    value_range: RangeInclusive<ControlValue>,
    /// The timespan of the current step.
    time_range: Range<MusicalTime>,

    /// The value that we last issued as a Control event. We keep track of this
    /// to avoid issuing consecutive identical events.
    last_published_value: f64,

    /// Whether the current_step working variables are an unknown state --
    /// either just-initialized, or the work cursor is jumping to an earlier
    /// position.
    is_current_step_clean: bool,
}
impl Default for ControlTripEphemerals {
    fn default() -> Self {
        Self {
            range: Default::default(),
            current_step: Default::default(),
            current_path: Default::default(),
            value_range: ControlValue::default()..=ControlValue::default(),
            time_range: MusicalTime::empty_range(),
            last_published_value: Default::default(),
            is_current_step_clean: Default::default(),
        }
    }
}
impl ControlTripEphemerals {
    fn reset_current_path_if_needed(&mut self) {
        if !self.is_current_step_clean {
            self.is_current_step_clean = true;
            self.current_step = Default::default();
            self.current_path = Default::default();
            self.value_range = ControlValue::default()..=ControlValue::default();
            self.time_range = MusicalTime::empty_range();
        }
    }
}

/// A [ControlTrip] is a single track of automation. It can run as long as the
/// whole song.
///
/// A trip consists of [ControlStep]s ordered by time. Each step specifies a
/// point in time, a [ControlValue], and a [ControlPath] that indicates how to
/// progress from the current [ControlStep] to the next one.
#[derive(Serialize, Deserialize, Debug, Default, IsController, Uid, Builder)]
#[builder(setter(skip), default)]
pub struct ControlTrip {
    uid: Uid,

    /// The [ControlStep]s that make up this trip. They must be in ascending
    /// time order. TODO: enforce that.
    #[builder(default, setter(each(name = "step", into)))]
    steps: Vec<ControlStep>,

    #[serde(skip)]
    e: ControlTripEphemerals,
}
impl ControlTrip {
    fn update_interval(&mut self) {
        self.e.reset_current_path_if_needed();

        // Are we in the middle of handling a step?
        if self.e.time_range.contains(&self.e.range.start) {
            // Yes; all the work is configured. Let's return so we can do it.
            return;
        }

        // The current step does not contain the current work slice. Find one that does.
        match self.steps.len() {
            0 => {
                // Empty trip. Mark that we don't have a path. This is a
                // terminal state.
                self.e.current_path = ControlPath::None;
            }
            1 => {
                // This trip has only one step, indicating that we should stay
                // level at its value.
                let step = &self.steps[0];
                self.e.current_path = ControlPath::Flat;
                self.e.value_range = step.value..=step.value;

                // Mark the time range to include all time so that we'll
                // early-exit this method in future calls.
                self.e.time_range = MusicalTime::START..MusicalTime::TIME_MAX;
            }
            _ => {
                // We have multiple steps. Find the one that corresponds to the
                // current work slice. Start with the current step, build a
                // range from it, and see whether it fits.

                let (mut end_time, mut end_value) = if self.e.current_step == 0 {
                    (MusicalTime::START, self.steps[0].value)
                } else {
                    (
                        self.steps[self.e.current_step - 1].time,
                        self.steps[self.e.current_step - 1].value,
                    )
                };
                loop {
                    let is_last = self.e.current_step == self.steps.len() - 1;
                    let step = &self.steps[self.e.current_step];
                    let next_step = if !is_last {
                        self.steps[self.e.current_step + 1].clone()
                    } else {
                        ControlStep {
                            value: step.value,
                            time: MusicalTime::TIME_MAX,
                            path: ControlPath::Flat,
                        }
                    };
                    let start_time = end_time;
                    let start_value = end_value;
                    (end_time, end_value) = (next_step.time, next_step.value);

                    // Build the range. Is it the right one?
                    let step_time_range = start_time..end_time;
                    if step_time_range.contains(&self.e.range.start) {
                        // Yes, this range contains the current work slice. Set
                        // it up, and get out of here.
                        self.e.current_path = step.path;
                        self.e.time_range = step_time_range;
                        self.e.value_range = match step.path {
                            ControlPath::None => todo!(),
                            ControlPath::Flat => start_value..=start_value,
                            ControlPath::Linear => start_value..=end_value,
                            ControlPath::Logarithmic => todo!(),
                            ControlPath::Exponential => todo!(),
                        };
                        break;
                    } else {
                        // No. Continue searching.
                        debug_assert!(
                            !is_last,
                            "Something is wrong. The last step's time range should be endless."
                        );
                        self.e.current_step += 1;
                    }
                }
            }
        }
    }

    fn ui_arrangement(&mut self, ui: &mut eframe::egui::Ui, view_range: &Range<MusicalTime>) {
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::hover());
        let to_screen = RectTransform::from_to(
            Rect::from_x_y_ranges(
                view_range.start.total_units() as f32..=view_range.end.total_units() as f32,
                ControlValue::MAX.0 as f32..=ControlValue::MIN.0 as f32,
            ),
            response.rect,
        );
        let mut pos = to_screen * pos2(MusicalTime::START.total_units() as f32, 0.0);
        for step in self.steps.iter_mut() {
            let second_pos = to_screen * pos2(step.time.total_units() as f32, step.value.0 as f32);
            painter.line_segment(
                [pos, second_pos],
                Stroke {
                    width: 1.0,
                    color: Color32::YELLOW,
                },
            );
            pos = second_pos;
        }
    }
}
impl Shows for ControlTrip {}
impl HandlesMidi for ControlTrip {}
impl Controls for ControlTrip {
    fn update_time(&mut self, range: &Range<MusicalTime>) {
        if range.start < self.e.range.start {
            // The cursor is jumping around. Mark things dirty.
            self.e.is_current_step_clean = false;
        }
        self.e.range = range.clone();
        self.update_interval();
    }

    fn work(&mut self, control_events_fn: &mut ControlEventsFn) {
        // If we have no current path, then we're all done.
        if matches!(self.e.current_path, ControlPath::None) {
            return;
        }
        if self.e.range.start >= self.e.time_range.end
            || self.e.range.end <= self.e.time_range.start
        {
            self.update_interval();
        }
        let current_point = self.e.range.start.total_units() as f64;
        let start = self.e.time_range.start.total_units() as f64;
        let end = self.e.time_range.end.total_units() as f64;
        let duration = end - start;
        let current_point = current_point - start;
        let percentage = if duration > 0.0 {
            current_point / duration
        } else {
            0.0
        };
        let current_value = self.e.value_range.start().0
            + percentage * (self.e.value_range.end().0 - self.e.value_range.start().0);
        if current_value != self.e.last_published_value {
            self.e.last_published_value = current_value;
            control_events_fn(
                self.uid,
                ThingEvent::Control(ControlValue::from(current_value)),
            );
        }
    }

    fn is_finished(&self) -> bool {
        matches!(self.e.current_path, ControlPath::None)
            || self.e.current_step + 1 == self.steps.len()
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

/// Describes a step of a [ControlTrip]. A [ControlStep] has a starting value as
/// of the specified time, and a [ControlPath] that specifies how to get from
/// the current value to the next [ControlStep]'s value.
///
/// If the first [ControlStep] in a [ControlTrip] does not start at
/// MusicalTime::START, then we synthesize a flat path, at this step's value,
/// from time zero to this step's time. Likewise, the last [ControlStep] in a
/// [ControlTrip] is always flat until MusicalTime::MAX.
#[derive(Serialize, Deserialize, Debug, Default, Builder, Clone)]
pub struct ControlStep {
    /// The initial value of this step.
    value: ControlValue,
    /// When this step begins.
    time: MusicalTime,
    /// How the step should progress to the next step. If this step is the last
    /// in a trip, then it's ControlPath::Flat.
    path: ControlPath,
}

/// A [ControlAtlas] manages a group of [ControlTrip]s. (An atlas is a book of
/// maps.)
#[derive(Serialize, Deserialize, IsController, Debug, Uid)]
pub struct ControlAtlas {
    uid: Uid,
    trips: Vec<ControlTrip>,
    #[serde(skip)]
    range: Range<MusicalTime>,
}
impl Default for ControlAtlas {
    fn default() -> Self {
        let mut r = Self {
            uid: Default::default(),
            trips: Default::default(),
            range: Default::default(),
        };
        r.add_trip(
            ControlTripBuilder::default()
                .step(
                    ControlStepBuilder::default()
                        .time(MusicalTime::DURATION_WHOLE)
                        .path(ControlPath::Flat)
                        .value(ControlValue(0.25))
                        .build()
                        .unwrap(),
                )
                .step(
                    ControlStepBuilder::default()
                        .time(MusicalTime::DURATION_WHOLE * 2)
                        .path(ControlPath::Flat)
                        .value(ControlValue(0.75))
                        .build()
                        .unwrap(),
                )
                .step(
                    ControlStepBuilder::default()
                        .time(MusicalTime::DURATION_WHOLE * 3)
                        .path(ControlPath::Flat)
                        .value(ControlValue(0.5))
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        );
        r
    }
}
impl Shows for ControlAtlas {
    fn show(&mut self, ui: &mut eframe::egui::Ui) {
        let (id, rect) = ui.allocate_space(vec2(ui.available_width(), 64.0));
        ui.allocate_ui_at_rect(rect, |ui| {
            ui.horizontal_top(|ui| {
                if ui.button("Add trip").clicked() {
                    let mut trip = ControlTripBuilder::default().build().unwrap();
                    trip.set_uid(EntityFactory::global().mint_uid());
                    self.add_trip(trip);
                }
                let mut remove_uid = None;
                for trip in self.trips.iter_mut() {
                    ui.vertical(|ui| {
                        ui.allocate_ui_at_rect(rect, |ui| {
                            trip.show(ui);
                            if ui.button("x").clicked() {
                                remove_uid = Some(trip.uid);
                            }
                        });
                    });
                    trip.show(ui);
                }
                if let Some(uid) = remove_uid {
                    self.remove_trip(uid);
                }
            });
        });
        // for trip in self.trips.iter_mut() {
        // }
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
    /// Adds the given [ControlTrip] to this atlas. TODO: specify any ordering constraints
    pub fn add_trip(&mut self, trip: ControlTrip) {
        self.trips.push(trip);
    }

    fn remove_trip(&mut self, uid: Uid) {
        self.trips.retain(|t| t.uid != uid);
    }

    pub fn ui_arrangement(&mut self, ui: &mut eframe::egui::Ui, view_range: &Range<MusicalTime>) {
        let (id, rect) = ui.allocate_space(vec2(ui.available_width(), 64.0));
        ui.allocate_ui_at_rect(rect, |ui| {
            ui.horizontal_top(|ui| {
                if ui.button("Add trip").clicked() {
                    let mut trip = ControlTripBuilder::default().build().unwrap();
                    trip.set_uid(EntityFactory::global().mint_uid());
                    self.add_trip(trip);
                }
                let mut remove_uid = None;
                for trip in self.trips.iter_mut() {
                    ui.vertical(|ui| {
                        ui.allocate_ui_at_rect(rect, |ui| {
                            trip.ui_arrangement(ui, view_range);
                            if ui.button("x").clicked() {
                                remove_uid = Some(trip.uid);
                            }
                        });
                    });
                    trip.show(ui);
                }
                if let Some(uid) = remove_uid {
                    self.remove_trip(uid);
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl ControlTrip {
        // Causes the next work() to emit a Control event, even if the value
        // matches the last event's value.
        fn debug_reset_last_value(&mut self) {
            self.e.last_published_value = f64::MAX;
        }
    }

    #[test]
    fn control_step_basics() {
        let step = ControlStepBuilder::default()
            .value(ControlValue(0.5))
            .time(MusicalTime::START + MusicalTime::DURATION_WHOLE)
            .path(ControlPath::Flat)
            .build();
        assert!(step.is_ok());
    }

    #[test]
    fn control_trip_one_step() {
        let mut ct = ControlTripBuilder::default()
            .step(ControlStep {
                value: ControlValue(0.5),
                time: MusicalTime::START + MusicalTime::DURATION_WHOLE,
                path: ControlPath::Flat,
            })
            .build()
            .unwrap();

        let range = MusicalTime::START..MusicalTime::DURATION_QUARTER;
        ct.update_time(&range);
        const MESSAGE: &str = "If there is only one control step, then the trip should remain at that step's level at all times.";
        let mut received_event = None;
        ct.work(&mut |_uid, event| {
            assert!(received_event.is_none());
            received_event = Some(event);
        });
        match received_event.unwrap() {
            ThingEvent::Control(value) => assert_eq!(value.0, 0.5, "{}", MESSAGE),
            _ => panic!(),
        }
        assert!(
            ct.is_finished(),
            "A one-step ControlTrip is always finished"
        );
    }

    #[test]
    fn control_trip_two_flat_steps() {
        let mut ct = ControlTripBuilder::default()
            .step(ControlStep {
                value: ControlValue(0.5),
                time: MusicalTime::START,
                path: ControlPath::Flat,
            })
            .step(ControlStep {
                value: ControlValue(0.75),
                time: MusicalTime::START + MusicalTime::DURATION_WHOLE,
                path: ControlPath::Flat,
            })
            .build()
            .unwrap();

        let range = MusicalTime::START..MusicalTime::DURATION_QUARTER;
        ct.update_time(&range);
        let mut received_event = None;
        ct.work(&mut |_uid, event| {
            assert!(received_event.is_none());
            received_event = Some(event);
        });
        match received_event.unwrap() {
            ThingEvent::Control(value) => assert_eq!(value.0, 0.5, "{}", "Flat step should work"),
            _ => panic!(),
        }
        assert!(!ct.is_finished());
        let range = MusicalTime::START + MusicalTime::DURATION_WHOLE
            ..MusicalTime::DURATION_WHOLE + MusicalTime::new_with_units(1);
        ct.update_time(&range);
        let mut received_event = None;
        ct.work(&mut |_uid, event| {
            assert!(received_event.is_none());
            received_event = Some(event);
        });
        match received_event.unwrap() {
            ThingEvent::Control(value) => assert_eq!(value.0, 0.75, "{}", "Flat step should work"),
            _ => panic!(),
        }
        assert!(ct.is_finished());
    }

    #[test]
    fn control_trip_linear_step() {
        let mut ct = ControlTripBuilder::default()
            .step(ControlStep {
                value: ControlValue(0.0),
                time: MusicalTime::START,
                path: ControlPath::Linear,
            })
            .step(ControlStep {
                value: ControlValue(1.0),
                time: MusicalTime::new_with_beats(2),
                path: ControlPath::Flat,
            })
            .build()
            .unwrap();

        let range = MusicalTime::new_with_beats(1)
            ..MusicalTime::new_with_beats(1) + MusicalTime::new_with_units(1);
        ct.update_time(&range);
        let mut received_event = None;
        ct.work(&mut |_uid, event| {
            assert!(received_event.is_none());
            received_event = Some(event);
        });
        match received_event.unwrap() {
            ThingEvent::Control(value) => assert_eq!(
                value.0, 0.5,
                "{}",
                "Halfway through linear 0.0..=1.0 should be 0.5"
            ),
            _ => panic!(),
        }
        assert!(!ct.is_finished());
    }

    #[test]
    fn control_trip_many_steps() {
        for i in 0..2 {
            let mut ct = ControlTripBuilder::default()
                .step(ControlStep {
                    value: ControlValue(0.1),
                    time: MusicalTime::new_with_units(10),
                    path: ControlPath::Flat,
                })
                .step(ControlStep {
                    value: ControlValue(0.2),
                    time: MusicalTime::new_with_units(20),
                    path: ControlPath::Flat,
                })
                .step(ControlStep {
                    value: ControlValue(0.3),
                    time: MusicalTime::new_with_units(30),
                    path: ControlPath::Flat,
                })
                .build()
                .unwrap();

            let mut test_values = vec![
                (0, 0.1, false),
                (5, 0.1, false),
                (10, 0.1, false),
                (11, 0.1, false),
                (20, 0.2, false),
                (21, 0.2, false),
                (30, 0.3, true),
                (31, 0.3, true),
                (9999999999, 0.3, true),
            ];
            if i == 1 {
                test_values.reverse();
            }

            for (unit, ev, finished) in test_values {
                let time = MusicalTime::new_with_units(unit);
                ct.update_time(&(time..(time + MusicalTime::new_with_units(1))));
                let mut received_event = None;
                ct.work(&mut |_uid, event| {
                    assert!(received_event.is_none());
                    received_event = Some(event);
                });
                assert!(received_event.is_some());
                match received_event.unwrap() {
                    ThingEvent::Control(value) => {
                        assert_eq!(
                            value.0, ev,
                            "{i}: Expected {ev} at {time} but got {}",
                            value.0
                        )
                    }
                    _ => panic!(),
                }
                assert_eq!(
                    ct.is_finished(),
                    finished,
                    "At time {time} expected is_finished({finished})"
                );
                ct.debug_reset_last_value();
            }
        }
    }
}
