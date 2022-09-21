use crate::primitives::clock::Clock;

use std::rc::Rc;
use std::{cell::RefCell, cmp::Ordering};

pub struct AutomationTrack {
    target_instrument: Rc<RefCell<dyn AutomationSink>>,
    target_param_name: String,
    cursor_beats: f32,

    last_target_param_value: f64,
    current_target_param_value: f64,
    target_param_value_delta: f64,

    automation_events: SortedVec<OrderedAutomationEvent>,
}

impl AutomationTrack {
    const CURSOR_BEGIN: f32 = 0.0;

    pub fn new(target: Rc<RefCell<dyn AutomationSink>>, target_param_name: String) -> Self {
        Self {
            target_instrument: target,
            target_param_name,
            cursor_beats: Self::CURSOR_BEGIN,
            last_target_param_value: 0.0,
            current_target_param_value: 0.0,
            target_param_value_delta: 0.0,
            automation_events: SortedVec::new(),
        }
    }

    pub fn reset_cursor(&mut self) {
        self.cursor_beats = Self::CURSOR_BEGIN;
    }

    pub fn add_pattern(&mut self, pattern: Rc<RefCell<AutomationPattern>>) {
        for point in pattern.borrow().points.clone() {
            self.automation_events.insert(OrderedAutomationEvent {
                when: self.cursor_beats,
                interpolation: pattern.borrow().interpolation.clone().unwrap_or_default(),
                target_param_value: point,
            });
            self.cursor_beats += 1.0;
        }
    }

    fn update_target(&mut self) {
        self.last_target_param_value = self.current_target_param_value;
        self.current_target_param_value += self.target_param_value_delta;
        if self.last_target_param_value != self.current_target_param_value {
            if self.target_param_name == "TODO" {
                // TODO: need to figure out where to put the mapping of param_name string
                // to the message type we send.
                todo!()
            }
            self.target_instrument.borrow_mut().handle_message(
                &super::traits::AutomationMessage::UpdatePrimaryValue {
                    value: self.current_target_param_value as f32,
                },
            );
        }
    }

    fn set_automation_envelope(&mut self, beats_per_sample: f32) {
        if self.automation_events.len() <= 1 {
            self.target_param_value_delta = 0.0;
            return;
        }

        // TODO: this code horrifies me. Playing fast and loose with ranges.
        let next_event = self.automation_events.get(1).unwrap();
        if self.current_target_param_value == next_event.target_param_value as f64 {
            self.target_param_value_delta = 0.0;
            return;
        }
        match next_event.interpolation {
            InterpolationType::Stairstep => {
                self.target_param_value_delta = 0.0;
            }
            InterpolationType::Linear => {
                self.target_param_value_delta =
                    next_event.target_param_value as f64 - self.current_target_param_value;
                self.target_param_value_delta *= beats_per_sample as f64; // TODO: just see if it works
            }
            InterpolationType::Logarithmic => todo!(),
            InterpolationType::Trigger => todo!(),
        }
    }
}

impl TimeSlicer for AutomationTrack {
    fn tick(&mut self, clock: &Clock) -> bool {
        if self.automation_events.is_empty() {
            // This is different from falling through the loop below because
            // it signals that we're done.
            return true;
        }

        self.update_target();

        while !self.automation_events.is_empty() {
            let event = self.automation_events.first().unwrap();

            if clock.beats >= event.when {
                // set new target
                self.current_target_param_value = event.target_param_value as f64;
                self.set_automation_envelope(clock.settings().beats_per_sample());

                // TODO: same issue as the similar code in Sequencer::tick().
                self.automation_events.remove_index(0);
            } else {
                break;
            }
        }
        false
    }
}

use sorted_vec::SortedVec;

use crate::{
    primitives::clock::BeatValue,
    settings::automation::{AutomationPatternSettings, InterpolationType},
};

use super::traits::{AutomationSink, TimeSlicer};

#[derive(Clone)]
pub struct AutomationPattern {
    pub note_value: Option<BeatValue>,
    pub interpolation: Option<InterpolationType>,
    pub points: Vec<f32>,
}

impl AutomationPattern {
    pub(crate) fn from_settings(settings: &AutomationPatternSettings) -> Self {
        Self {
            note_value: settings.note_value.clone(),
            interpolation: settings.interpolation.clone(),
            points: settings.points.clone(),
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct OrderedAutomationEvent {
    when: f32,
    interpolation: InterpolationType,
    target_param_value: f32,
}

// TODO: test these. I'm not sure they're right.
// better TODO: avoid floats!
impl Ord for OrderedAutomationEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.when > other.when {
            return Ordering::Greater;
        }
        if self.when < other.when {
            return Ordering::Less;
        }
        Ordering::Equal
    }
}

impl PartialOrd for OrderedAutomationEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.when > other.when {
            return Some(Ordering::Greater);
        }
        if self.when < other.when {
            return Some(Ordering::Less);
        }
        Some(Ordering::Equal)
    }
}

impl Eq for OrderedAutomationEvent {}

#[cfg(test)]
mod tests {
    use crate::devices::tests::NullDevice;
    use crate::settings::automation::InterpolationType;

    use super::*;

    #[test]
    fn test_stairstep_automation() {
        let pattern = Rc::new(RefCell::new(AutomationPattern {
            note_value: Some(BeatValue::Quarter),
            interpolation: Some(InterpolationType::Stairstep),
            points: vec![0.0, 0.1, 0.2, 0.3],
        }));
        let target = Rc::new(RefCell::new(NullDevice::new()));
        let target_param_name = String::from("value");
        let mut track = AutomationTrack::new(target.clone(), target_param_name);
        track.add_pattern(pattern.clone());

        // TODO: I want a way at this point to tell how long the clock needs
        // to run by asking the pattern, or maybe the track, what its length
        // is in some useful unit.

        assert_eq!(target.borrow().value, 0.0f32);
        let mut clock = Clock::new_test();
        let mut current_pattern_point: usize = 0;
        loop {
            let mut done = true;
            done = track.tick(&clock) && done;
            if clock.beats as usize == current_pattern_point {
                let point = pattern.borrow().points[current_pattern_point];
                assert_eq!(target.borrow().value, point);
                current_pattern_point += 1;
            }
            clock.tick();
            if done {
                break;
            }
        }
        assert_eq!(target.borrow().value, 0.3f32);
    }

    #[test]
    fn test_linear_automation() {
        let pattern_vec = vec![0.0, 1.0, 0.5, 0.0];
        let pattern_interpolated_vec = vec![0.0, 0.5, 1.0, 0.75, 0.5, 0.25, 0.0];
        let pattern = Rc::new(RefCell::new(AutomationPattern {
            note_value: Some(BeatValue::Quarter),
            interpolation: Some(InterpolationType::Linear),
            points: pattern_vec.clone(),
        }));
        let target = Rc::new(RefCell::new(NullDevice::new()));
        let target_param_name = String::from("value");
        let mut track = AutomationTrack::new(target.clone(), target_param_name);
        track.add_pattern(pattern.clone());

        assert_eq!(target.borrow().value, 0.0f32);
        let mut clock = Clock::new_test();
        let mut current_pattern_point = 0.0;
        let mut expected_value = 0.0;
        loop {
            let mut done = true;
            done = track.tick(&clock) && done;
            if clock.beats >= current_pattern_point {
                expected_value = pattern_interpolated_vec[(current_pattern_point * 2.0) as usize];
                assert!((target.borrow().value - expected_value).abs() < 0.01f32);
                current_pattern_point += 0.5;
            }
            clock.tick();
            if done {
                break;
            }
        }
        assert!((target.borrow().value - expected_value).abs() < 0.001f32);
    }
}
