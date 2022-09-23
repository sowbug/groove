use crate::primitives::clock::Clock;
use crate::settings::automation::AutomationStepType;

use std::collections::VecDeque;
use std::rc::Rc;
use std::{cell::RefCell, cmp::Ordering};

pub struct AutomationTrip {
    target_instrument: Rc<RefCell<dyn AutomationSink>>,
    //    target_param_name: String,
    cursor_beats: f32,

    current_value: f32,

    envelopes: SortedVec<AutomationEnvelope>,
    envelopes_in_place: VecDeque<AutomationEnvelope>,
}

impl AutomationTrip {
    const CURSOR_BEGIN: f32 = 0.0;

    pub fn new(target: Rc<RefCell<dyn AutomationSink>>, _target_param_name: String) -> Self {
        Self {
            target_instrument: target,
            //          target_param_name,
            cursor_beats: Self::CURSOR_BEGIN,
            current_value: f32::MAX, // TODO we want to make sure we set the target's value at start
            envelopes: SortedVec::new(),
            envelopes_in_place: VecDeque::new(),
        }
    }

    pub fn reset_cursor(&mut self) {
        self.cursor_beats = Self::CURSOR_BEGIN;
    }

    pub fn add_path(&mut self, path: Rc<RefCell<AutomationPath>>) {
        for step in path.borrow().steps.clone() {
            let (start_value, end_value) = match step {
                AutomationStepType::Flat { value } => (value, value),
                AutomationStepType::Slope { start, end } => (start, end),
            };
            self.envelopes.insert(AutomationEnvelope {
                start_beat: self.cursor_beats,
                end_beat: self.cursor_beats + 1.0,
                start_value,
                target_value: end_value,
                current_value: start_value,
            });

            self.cursor_beats += 1.0;
        }
    }

    pub fn freeze_trip_envelopes(&mut self) {
        self.envelopes_in_place = VecDeque::new();
        let i = self.envelopes.iter();
        for e in i {
            self.envelopes_in_place.push_back(*e);
        }
        self.envelopes.clear();
    }
}

impl TimeSlicer for AutomationTrip {
    fn tick(&mut self, clock: &Clock) -> bool {
        if self.envelopes_in_place.is_empty() {
            // This is different from falling through the loop below because
            // it signals that we're done.
            return true;
        }

        let mut num_to_remove: usize = 0;
        for envelope in self.envelopes_in_place.iter_mut() {
            if clock.beats < envelope.start_beat {
                break;
            }
            let last_value = self.current_value;
            self.current_value = envelope.current_value;
            if self.current_value != last_value {
                self.target_instrument
                    .borrow_mut()
                    .handle_automation_message(&AutomationMessage::UpdatePrimaryValue {
                        value: self.current_value,
                    });
            }
            if envelope.tick(clock) {
                num_to_remove += 1;
            }
        }
        if num_to_remove > 0 {
            // TODO: same issue as the similar code in Sequencer::tick().
            self.envelopes_in_place.drain(0..num_to_remove);
        }
        false
    }
}

use sorted_vec::SortedVec;

#[derive(Default, PartialEq, Clone, Copy)]
struct AutomationEnvelope {
    start_beat: f32,
    end_beat: f32,
    start_value: f32,
    target_value: f32,
    current_value: f32, // TODO: this feels more like a working value, not a struct value
}

impl TimeSlicer for AutomationEnvelope {
    fn tick(&mut self, clock: &Clock) -> bool {
        let total_length_beats = self.end_beat - self.start_beat;
        if total_length_beats != 0.0 {
            let how_far_we_have_gone_beats = clock.beats - self.start_beat;
            let percentage_done = how_far_we_have_gone_beats / total_length_beats;
            let total_length_value = self.target_value - self.start_value;
            self.current_value = self.start_value + total_length_value * percentage_done;
        } else {
            self.current_value = self.target_value;
        }

        // Are we done with all our work?
        clock.beats >= self.end_beat
    }
}

impl PartialOrd for AutomationEnvelope {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.start_beat > other.start_beat {
            return Some(Ordering::Greater);
        }
        if self.start_beat < other.start_beat {
            return Some(Ordering::Less);
        }
        Some(Ordering::Equal)
    }
}

impl Ord for AutomationEnvelope {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.start_beat > other.start_beat {
            return Ordering::Greater;
        }
        if self.start_beat < other.start_beat {
            return Ordering::Less;
        }
        Ordering::Equal
    }
}

impl Eq for AutomationEnvelope {}

use crate::{primitives::clock::BeatValue, settings::automation::AutomationPathSettings};

use super::traits::{AutomationMessage, AutomationSink, TimeSlicer};

#[derive(Clone)]
pub struct AutomationPath {
    pub note_value: Option<BeatValue>,
    pub steps: Vec<AutomationStepType>,
}

impl AutomationPath {
    pub(crate) fn from_settings(settings: &AutomationPathSettings) -> Self {
        Self {
            note_value: settings.note_value.clone(),
            steps: settings.steps.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::devices::tests::NullDevice;
    use assert_approx_eq::assert_approx_eq;

    use super::*;

    // TODO: I want a way at this point to tell how long the clock needs
    // to run by asking the path, or maybe the trip, what its length
    // is in some useful unit.

    // TODO: a mini orchestrator that ticks until a certain condition is met

    #[test]
    fn test_flat_step_automation() {
        let step_vec = vec![
            AutomationStepType::Flat { value: 0.9 },
            AutomationStepType::Flat { value: 0.1 },
            AutomationStepType::Flat { value: 0.2 },
            AutomationStepType::Flat { value: 0.3 },
        ];
        let step_count = step_vec.len();
        let sequence = Rc::new(RefCell::new(AutomationPath {
            note_value: Some(BeatValue::Quarter),
            steps: step_vec,
        }));
        let target = Rc::new(RefCell::new(NullDevice::new()));
        let target_param_name = String::from("value");
        let mut trip = AutomationTrip::new(target.clone(), target_param_name);
        trip.add_path(sequence.clone());
        trip.freeze_trip_envelopes(); // TODO I hate this method

        assert_eq!(target.borrow().value, 0.0f32);

        let mut clock = Clock::new_test();
        let mut step_index: usize = 0;
        let mut expected_value = f32::MAX;
        loop {
            let mut done = true;

            // Let the trip do its work.
            done = trip.tick(&clock) && done;

            // Have we reached a new beat? If yes, we need to update the expected value.
            if clock.beats as usize == step_index {
                // But only if we have a new step. If not, the old expected value stays.
                if step_index < step_count {
                    let step = &sequence.borrow().steps[step_index];
                    match step {
                        AutomationStepType::Flat { value } => {
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
    fn test_slope_step_automation() {
        const SAD_FLOAT_DIFF: f32 = 1.0e-2;
        let step_vec = vec![
            AutomationStepType::new_slope(0.0, 1.0),
            AutomationStepType::new_slope(1.0, 0.5),
            AutomationStepType::new_slope(1.0, 0.0),
            AutomationStepType::new_slope(0.0, 1.0),
        ];
        let interpolated_values = vec![0.0, 0.5, 1.0, 0.75, 1.0, 0.5, 0.0, 0.5, 1.0];
        let path = Rc::new(RefCell::new(AutomationPath {
            note_value: Some(BeatValue::Quarter),
            steps: step_vec.clone(),
        }));
        let target = Rc::new(RefCell::new(NullDevice::new()));
        let target_param_name = String::from("value");
        let mut trip = AutomationTrip::new(target.clone(), target_param_name);
        trip.add_path(path.clone());
        trip.freeze_trip_envelopes();

        assert_eq!(target.borrow().value, 0.0f32); // what good is this?

        let mut clock = Clock::new_test();
        let mut current_pattern_point = 0.0;
        let mut expected_value = 0.0;
        loop {
            let mut done = true;
            done = trip.tick(&clock) && done;
            if clock.beats >= current_pattern_point {
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
