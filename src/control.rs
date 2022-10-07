use crate::clock::Clock;
use crate::envelopes::{EnvelopeStep, EnvelopeTimeUnit, SteppedEnvelope};
use crate::settings::control::ControlStepType;
use crate::traits::WatchesClock;
use crate::traits::{SinksControl, SinksControlParam};

use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

#[derive(Debug)]
pub struct ControlTrip {
    target_instrument: Rc<RefCell<dyn SinksControl>>,
    cursor_beats: f32,

    current_value: f32,

    envelope: SteppedEnvelope,
}

impl ControlTrip {
    const CURSOR_BEGIN: f32 = 0.0;

    #[allow(unused_variables)]
    pub fn new(target: Rc<RefCell<dyn SinksControl>>, target_param_name: String) -> Self {
        Self {
            target_instrument: target,
            cursor_beats: Self::CURSOR_BEGIN,
            current_value: f32::MAX, // TODO we want to make sure we set the target's value at start
            envelope: SteppedEnvelope::new_with_time_unit(EnvelopeTimeUnit::Beats),
        }
    }

    pub fn reset_cursor(&mut self) {
        self.cursor_beats = Self::CURSOR_BEGIN;
    }

    // TODO: assert that these are added in time order
    pub fn add_path(&mut self, path: Rc<RefCell<ControlPath>>) {
        for step in path.borrow().steps.clone() {
            let (start_value, end_value) = match step {
                ControlStepType::Flat { value } => (value, value),
                ControlStepType::Slope { start, end } => (start, end),
            };
            self.envelope.push_step(EnvelopeStep {
                interval: Range {
                    start: self.cursor_beats,
                    end: self.cursor_beats + 1.0,
                },
                start_value,
                end_value,
            });
            self.cursor_beats += 1.0;
        }
    }
}

impl WatchesClock for ControlTrip {
    fn tick(&mut self, clock: &Clock) -> bool {
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
            return time >= step.interval.end;
        }

        // This is a drastic response to a tick that's out of range.
        // It might be better to limit it to times that are later than
        // the covered range. We're likely to hit ControlTrips that
        // start beyond time zero.
        true
    }
}

use crate::{clock::BeatValue, settings::control::ControlPathSettings};

#[derive(Clone, Debug, Default)]
pub struct ControlPath {
    pub note_value: Option<BeatValue>,
    pub steps: Vec<ControlStepType>,
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
    use assert_approx_eq::assert_approx_eq;

    use crate::traits::tests::NullDevice;

    use super::*;

    // TODO: I want a way at this point to tell how long the clock needs
    // to run by asking the path, or maybe the trip, what its length
    // is in some useful unit.

    // TODO: a mini orchestrator that ticks until a certain condition is met

    #[test]
    fn test_flat_step() {
        let step_vec = vec![
            ControlStepType::Flat { value: 0.9 },
            ControlStepType::Flat { value: 0.1 },
            ControlStepType::Flat { value: 0.2 },
            ControlStepType::Flat { value: 0.3 },
        ];
        let step_count = step_vec.len();
        let sequence = Rc::new(RefCell::new(ControlPath {
            note_value: Some(BeatValue::Quarter),
            steps: step_vec,
        }));
        let target = Rc::new(RefCell::new(NullDevice::new()));
        let target_param_name = String::from("value");
        let target_weak = Rc::clone(&target);
        let mut trip = ControlTrip::new(target_weak, target_param_name);
        trip.add_path(Rc::clone(&sequence));

        assert_eq!(target.borrow().value, 0.0f32);

        let mut clock = Clock::new_test();
        let mut step_index: usize = 0;
        let mut expected_value = f32::MAX;
        loop {
            let mut done = true;

            // Let the trip do its work.
            done = trip.tick(&clock) && done;

            // Have we reached a new beat? If yes, we need to update the expected value.
            if clock.beats() as usize == step_index {
                // But only if we have a new step. If not, the old expected value stays.
                if step_index < step_count {
                    let step = &sequence.borrow().steps[step_index];
                    match step {
                        ControlStepType::Flat { value } => {
                            expected_value = *value;
                        }
                        _ => panic!(),
                    }
                }
                step_index += 1;
            }

            // Make sure the value is correct for every time slice.
            assert_eq!(target.borrow().value, expected_value);
            if done {
                break;
            }

            clock.tick();
        }
        assert_eq!(target.borrow().value, 0.3);
    }

    #[test]
    fn test_slope_step() {
        const SAD_FLOAT_DIFF: f32 = 1.0e-2;
        let step_vec = vec![
            ControlStepType::new_slope(0.0, 1.0),
            ControlStepType::new_slope(1.0, 0.5),
            ControlStepType::new_slope(1.0, 0.0),
            ControlStepType::new_slope(0.0, 1.0),
        ];
        let interpolated_values = vec![0.0, 0.5, 1.0, 0.75, 1.0, 0.5, 0.0, 0.5, 1.0];
        let path = Rc::new(RefCell::new(ControlPath {
            note_value: Some(BeatValue::Quarter),
            steps: step_vec,
        }));
        let target = Rc::new(RefCell::new(NullDevice::new()));
        let target_param_name = String::from("value");
        let target_trip_clone = Rc::clone(&target);
        let mut trip = ControlTrip::new(target_trip_clone, target_param_name);
        trip.add_path(path);

        let mut clock = Clock::new_test();
        let mut current_pattern_point = 0.0;
        let mut expected_value = 0.0;
        loop {
            let mut done = true;
            done = trip.tick(&clock) && done;
            if clock.beats() >= current_pattern_point {
                expected_value = interpolated_values[(current_pattern_point * 2.0) as usize];
                assert_approx_eq!(target.borrow().value, expected_value, SAD_FLOAT_DIFF);
                current_pattern_point += 0.5;
            }
            clock.tick();
            if done {
                break;
            }
        }
        assert_approx_eq!(target.borrow().value, expected_value, SAD_FLOAT_DIFF);
    }
}
