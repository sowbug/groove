use crate::clock::{Clock, ClockTimeUnit};
use crate::envelopes::{EnvelopeStep, SteppedEnvelope};
use crate::settings::control::ControlStep;
use crate::traits::{SinksControl, SinksControlParam};
use crate::traits::{Terminates, WatchesClock};

use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

/// ControlTrip, ControlPath, and ControlStep help with
/// [automation](https://en.wikipedia.org/wiki/Track_automation).
/// Briefly, a ControlTrip consists of ControlSteps stamped out
/// of ControlPaths, and ControlSteps are generic EnvelopeSteps
/// that SteppedEnvelope uses.
///
/// A ControlTrip is one automation track, which can run as long
/// as the whole song. For now, it controls one parameter of one
/// target.
#[derive(Debug)]
pub struct ControlTrip {
    target_instrument: Rc<RefCell<dyn SinksControl>>,
    cursor_beats: f32,

    current_value: f32,

    envelope: SteppedEnvelope,

    is_finished: bool,
}

impl ControlTrip {
    const CURSOR_BEGIN: f32 = 0.0;

    #[allow(unused_variables)]
    pub fn new(target: Rc<RefCell<dyn SinksControl>>, target_param_name: String) -> Self {
        Self {
            target_instrument: target,
            cursor_beats: Self::CURSOR_BEGIN,
            current_value: f32::MAX, // TODO we want to make sure we set the target's value at start
            envelope: SteppedEnvelope::new_with_time_unit(ClockTimeUnit::Beats),
            is_finished: true,
        }
    }

    pub fn reset_cursor(&mut self) {
        self.cursor_beats = Self::CURSOR_BEGIN;
    }

    // TODO: assert that these are added in time order, as SteppedEnvelope
    // currently isn't smart enough to handle out-of-order construction
    pub fn add_path(&mut self, path: Rc<RefCell<ControlPath>>) {
        for step in path.borrow().steps.clone() {
            let (start_value, end_value) = match step {
                ControlStep::Flat { value } => (value, value),
                ControlStep::Slope { start, end } => (start, end),
                #[allow(unused_variables)]
                ControlStep::Logarithmic { start, end } => todo!(),
                ControlStep::Triggered {} => todo!(),
            };
            // Beware: there's an O(N) debug validlity check in push_step(),
            // so this loop is O(N^2).
            self.envelope.push_step(EnvelopeStep {
                interval: Range {
                    start: self.cursor_beats,
                    end: self.cursor_beats + 1.0,
                },
                start_value,
                end_value,
            });
            self.cursor_beats += 1.0; // TODO: respect note_value
        }
        self.is_finished = false;
    }
}

impl WatchesClock for ControlTrip {
    fn tick(&mut self, clock: &Clock) {
        let time = self.envelope.time_for_unit(clock);
        let step = self.envelope.step_for_time(time);
        if step.interval.contains(&time) {
            let value = self.envelope.value_for_step_at_time(step, time);

            let last_value = self.current_value;
            self.current_value = value;
            if self.current_value != last_value {
                self.target_instrument.borrow_mut().handle_control(
                    clock,
                    &SinksControlParam::Primary {
                        value: self.current_value,
                    },
                );
            }
            self.is_finished = time >= step.interval.end;
            return;
        }

        // This is a drastic response to a tick that's out of range.
        // It might be better to limit it to times that are later than
        // the covered range. We're likely to hit ControlTrips that
        // start beyond time zero.
        self.is_finished = true;
    }
}

impl Terminates for ControlTrip {
    fn is_finished(&self) -> bool {
        self.is_finished
    }
}

use crate::{clock::BeatValue, settings::control::ControlPathSettings};

/// A ControlPath makes it easier to construct sequences of ControlSteps.
/// It's just like a pattern in a pattern-based sequencer. ControlPaths
/// aren't required; they just make repetitive sequences less tedious
/// to build.
#[derive(Clone, Debug, Default)]
pub struct ControlPath {
    pub note_value: Option<BeatValue>,
    pub steps: Vec<ControlStep>,
}

impl ControlPath {
    pub(crate) fn from_settings(settings: &ControlPathSettings) -> Self {
        Self {
            note_value: settings.note_value.clone(),
            steps: settings.steps.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use crate::{
        clock::WatchedClock,
        common::MonoSample,
        utils::tests::{TestMidiSink, TestOrchestrator, TestValueChecker},
    };

    use super::*;

    #[test]
    fn test_flat_step() {
        let step_vec = vec![
            ControlStep::Flat { value: 0.9 },
            ControlStep::Flat { value: 0.1 },
            ControlStep::Flat { value: 0.2 },
            ControlStep::Flat { value: 0.3 },
        ];
        let sequence = Rc::new(RefCell::new(ControlPath {
            note_value: Some(BeatValue::Quarter),
            steps: step_vec,
        }));
        let target = Rc::new(RefCell::new(TestMidiSink::new()));
        let target_param_name = String::from("value");
        let target_weak = Rc::clone(&target);
        let trip = Rc::new(RefCell::new(ControlTrip::new(
            target_weak,
            target_param_name,
        )));
        trip.borrow_mut().add_path(Rc::clone(&sequence));

        let mut clock = WatchedClock::new();
        clock.add_watcher(trip);

        clock.add_watcher(Rc::new(RefCell::new(TestValueChecker {
            values: VecDeque::from(vec![0.9, 0.1, 0.2, 0.3]),
            target,
            checkpoint: 0.0,
            checkpoint_delta: 1.0,
            time_unit: ClockTimeUnit::Beats,
        })));

        let mut samples_out = Vec::<MonoSample>::new();
        let mut o = TestOrchestrator::new();
        o.start(&mut clock, &mut samples_out);
    }

    #[test]
    fn test_slope_step() {
        let step_vec = vec![
            ControlStep::new_slope(0.0, 1.0),
            ControlStep::new_slope(1.0, 0.5),
            ControlStep::new_slope(1.0, 0.0),
            ControlStep::new_slope(0.0, 1.0),
        ];
        let interpolated_values = vec![0.0, 0.5, 1.0, 0.75, 1.0, 0.5, 0.0, 0.5, 1.0];
        let path = Rc::new(RefCell::new(ControlPath {
            note_value: Some(BeatValue::Quarter),
            steps: step_vec,
        }));
        let target = Rc::new(RefCell::new(TestMidiSink::new()));
        let target_param_name = String::from("value");
        let target_trip_clone = Rc::clone(&target);
        let trip = Rc::new(RefCell::new(ControlTrip::new(
            target_trip_clone,
            target_param_name,
        )));
        trip.borrow_mut().add_path(path);

        let mut clock = WatchedClock::new();
        clock.add_watcher(trip);

        let target = Rc::clone(&target);
        clock.add_watcher(Rc::new(RefCell::new(TestValueChecker {
            values: VecDeque::from(interpolated_values),
            target,
            checkpoint: 0.0,
            checkpoint_delta: 0.5,
            time_unit: ClockTimeUnit::Beats,
        })));

        let mut samples_out = Vec::<MonoSample>::new();
        let mut o = TestOrchestrator::new();
        o.start(&mut clock, &mut samples_out);
    }
}
