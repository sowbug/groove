use crate::clock::{Clock, ClockTimeUnit};
use crate::common::WW;
use crate::effects::bitcrusher::Bitcrusher;
use crate::effects::limiter::Limiter;
use crate::effects::mixer::Mixer;
use crate::effects::{filter::Filter, gain::Gain};
use crate::envelopes::{AdsrEnvelope, EnvelopeStep, SteppedEnvelope};
use crate::oscillators::Oscillator;
use crate::settings::control::ControlStep;
use crate::traits::{MakesControlSink, SinksControl, Terminates, WatchesClock};

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
    target: Box<dyn SinksControl>,
    cursor_beats: f32,

    current_value: f32,

    envelope: SteppedEnvelope,

    is_finished: bool,
}

impl ControlTrip {
    const CURSOR_BEGIN: f32 = 0.0;

    #[allow(unused_variables)]
    pub fn new(
        //        target: Rc<RefCell<dyn SinksControl>>,
        target: Box<dyn SinksControl>,
    ) -> Self {
        Self {
            target,
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
                self.target.handle_control(clock, self.current_value);
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

#[derive(Debug)]
pub struct FilterQController {
    target: WW<Filter>,
}
impl SinksControl for FilterQController {
    fn handle_control(&mut self, _clock: &Clock, value: f32) {
        if let Some(target) = self.target.upgrade() {
            target.borrow_mut().set_q(value);
        }
    }
}

#[derive(Debug)]
pub struct FilterCutoffController {
    target: WW<Filter>,
}
impl SinksControl for FilterCutoffController {
    fn handle_control(&mut self, _clock: &Clock, value: f32) {
        if let Some(target) = self.target.upgrade() {
            target.borrow_mut().set_cutoff(value);
        }
    }
}
impl MakesControlSink for Filter {
    fn make_control_sink(&self) -> Option<Box<dyn SinksControl>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(FilterCutoffController {
                target: self.me.clone(),
            }))
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct AdsrEnvelopeNoteController {
    target: WW<AdsrEnvelope>,
}
impl SinksControl for AdsrEnvelopeNoteController {
    fn handle_control(&mut self, clock: &Clock, value: f32) {
        if let Some(target) = self.target.upgrade() {
            target.borrow_mut().handle_note_event(clock, value == 1.0);
        }
    }
}
impl MakesControlSink for AdsrEnvelope {
    fn make_control_sink(&self) -> Option<Box<dyn SinksControl>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(AdsrEnvelopeNoteController {
                target: self.me.clone(),
            }))
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct GainLevelController {
    target: WW<Gain>,
}
impl SinksControl for GainLevelController {
    fn handle_control(&mut self, _clock: &Clock, value: f32) {
        if let Some(target) = self.target.upgrade() {
            target.borrow_mut().set_level(value);
        }
    }
}
impl MakesControlSink for Gain {
    fn make_control_sink(&self) -> Option<Box<dyn SinksControl>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(GainLevelController {
                target: self.me.clone(),
            }))
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct OscillatorFrequencyController {
    target: WW<Oscillator>,
}
impl SinksControl for OscillatorFrequencyController {
    fn handle_control(&mut self, _clock: &Clock, value: f32) {
        if let Some(target) = self.target.upgrade() {
            target.borrow_mut().set_frequency(value);
        }
    }
}
impl MakesControlSink for Oscillator {
    fn make_control_sink(&self) -> Option<Box<dyn SinksControl>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(OscillatorFrequencyController {
                target: self.me.clone(),
            }))
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct LimiterMinLevelController {
    target: WW<Limiter>,
}
impl SinksControl for LimiterMinLevelController {
    fn handle_control(&mut self, _clock: &Clock, value: f32) {
        if let Some(target) = self.target.upgrade() {
            target.borrow_mut().set_min(value);
        }
    }
}
#[derive(Debug)]
pub struct LimiterMaxLevelController {
    target: WW<Limiter>,
}
impl SinksControl for LimiterMaxLevelController {
    fn handle_control(&mut self, _clock: &Clock, value: f32) {
        if let Some(target) = self.target.upgrade() {
            target.borrow_mut().set_max(value);
        }
    }
}
impl MakesControlSink for Limiter {
    fn make_control_sink(&self) -> Option<Box<dyn SinksControl>> {
        if self.me.strong_count() != 0 {
            // TODO match all values!
            Some(Box::new(LimiterMinLevelController {
                target: self.me.clone(),
            }))
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct BitcrusherBitCountController {
    target: WW<Bitcrusher>,
}
impl SinksControl for BitcrusherBitCountController {
    fn handle_control(&mut self, _clock: &Clock, value: f32) {
        if let Some(target) = self.target.upgrade() {
            target.borrow_mut().set_bits_to_crush(value as u8); // TODO: are we only (0.0..=1.0)?
        }
    }
}
impl MakesControlSink for Bitcrusher {
    fn make_control_sink(&self) -> Option<Box<dyn SinksControl>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(BitcrusherBitCountController {
                target: self.me.clone(),
            }))
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct MixerController {
    target: WW<Mixer>,
}
impl SinksControl for MixerController {
    fn handle_control(&mut self, _clock: &Clock, _value: f32) {
        if let Some(_) = self.target.upgrade() {
            // Mixer doesn't have any adjustable parameters!
        }
    }
}
impl MakesControlSink for Mixer {
    fn make_control_sink(&self) -> Option<Box<dyn SinksControl>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(MixerController {
                target: self.me.clone(),
            }))
        } else {
            None
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

        let mut clock = WatchedClock::new();
        let target = TestMidiSink::new_wrapped();
        if let Some(target_control_sink) = target.borrow().make_control_sink() {
            let trip = Rc::new(RefCell::new(ControlTrip::new(target_control_sink)));
            trip.borrow_mut().add_path(Rc::clone(&sequence));

            clock.add_watcher(trip);
        }

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

        let mut clock = WatchedClock::new();
        let target = TestMidiSink::new_wrapped();
        if let Some(target_control_sink) = target.borrow().make_control_sink() {
            let trip = Rc::new(RefCell::new(ControlTrip::new(target_control_sink)));
            trip.borrow_mut().add_path(path);
            clock.add_watcher(trip);
        }

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
