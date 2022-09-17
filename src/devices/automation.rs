use crate::primitives::clock::Clock;
use crate::synthesizers::welsh;

use std::rc::Rc;
use std::{cell::RefCell, cmp::Ordering};

pub struct AutomationTrack {
    target_instrument: Rc<RefCell<dyn AutomationSink>>,
    target_param_name: String,

    automation_events: SortedVec<OrderedAutomationEvent>,

    // for DeviceTrait
    needs_tick: bool,
}

impl AutomationTrack {
    pub fn new(target: Rc<RefCell<dyn AutomationSink>>, target_param_name: String) -> Self {
        Self {
            // target_instrument: Rc::new(RefCell::new(welsh::Synth::new(
            //     234,
            //     welsh::SynthPreset::by_name(&welsh::PresetName::Piano),
            // ))), //target,
            target_instrument: target,
            target_param_name,
            automation_events: SortedVec::new(),
            needs_tick: true,
        }
    }

    pub fn add_pattern(
        &mut self,
        pattern: Rc<RefCell<AutomationPattern>>,
        insertion_point: &mut f32,
    ) {
        // TODO: beat_value accumulates integer error
        for point in pattern.borrow().points.clone() {
            self.automation_events.insert(OrderedAutomationEvent {
                when: *insertion_point,
                target_param_value: point,
            });
            *insertion_point += 1.0;
        }
    }
}

impl TimeSlice for AutomationTrack {
    fn needs_tick(&self) -> bool {
        self.needs_tick
    }

    fn reset_needs_tick(&mut self) {
        self.needs_tick = true;
    }

    fn tick(&mut self, clock: &Clock) -> bool {
        self.needs_tick = false;

        if self.automation_events.is_empty() {
            // This is different from falling through the loop below because
            // it signals that we're done.
            return true;
        }
        while !self.automation_events.is_empty() {
            let event = self.automation_events.first().unwrap();

            if clock.beats >= event.when {
                // TODO: act on the automation thing
                self.target_instrument
                    .borrow_mut()
                    .handle_automation(&self.target_param_name, event.target_param_value);

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

use super::traits::{AutomationSink, TimeSlice};

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

#[derive(PartialEq, PartialOrd, Clone, Debug)]
pub struct OrderedAutomationEvent {
    when: f32,
    target_param_value: f32,
}

impl Ord for OrderedAutomationEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.when > other.when {
            return Ordering::Greater;
        }
        if self.when < other.when {
            return Ordering::Less;
        }
        return Ordering::Equal;
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
        let mut insertion_point = 0.0;
        track.add_pattern(pattern.clone(), &mut insertion_point);

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
}
