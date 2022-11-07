use crate::clock::{Clock, ClockTimeUnit};
use crate::common::{wrc_clone, Ww};
use crate::effects::arpeggiator::Arpeggiator;
use crate::effects::bitcrusher::Bitcrusher;
use crate::effects::limiter::Limiter;
use crate::effects::mixer::Mixer;
use crate::effects::{filter::BiQuadFilter, gain::Gain};
use crate::envelopes::{AdsrEnvelope, EnvelopeFunction, EnvelopeStep, SteppedEnvelope};
use crate::oscillators::Oscillator;
use crate::settings::control::ControlStep;
use crate::traits::{MakesControlSink, SinksControl, Terminates, WatchesClock};
use std::ops::Range;

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
    pub fn new(target: Box<dyn SinksControl>) -> Self {
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
    pub fn add_path(&mut self, path: &ControlPath) {
        for step in path.steps.clone() {
            let (start_value, end_value, step_function) = match step {
                ControlStep::Flat { value } => (value, value, EnvelopeFunction::Linear),
                ControlStep::Slope { start, end } => (start, end, EnvelopeFunction::Linear),
                ControlStep::Logarithmic { start, end } => {
                    (start, end, EnvelopeFunction::Logarithmic)
                }
                ControlStep::Exponential { start, end } => {
                    (start, end, EnvelopeFunction::Exponential)
                }
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
                step_function,
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
pub struct FilterCutoffController {
    target: Ww<BiQuadFilter>,
}
impl SinksControl for FilterCutoffController {
    fn handle_control(&mut self, _clock: &Clock, value: f32) {
        if let Some(target) = self.target.upgrade() {
            target.borrow_mut().set_cutoff_pct(value);
        }
    }
}

#[derive(Debug)]
pub struct FilterQController {
    target: Ww<BiQuadFilter>,
}
impl SinksControl for FilterQController {
    fn handle_control(&mut self, _clock: &Clock, value: f32) {
        if let Some(target) = self.target.upgrade() {
            target.borrow_mut().set_q(value);
        }
    }
}

// TODO: I guess I haven't gotten around to implementing these yet for Filter
#[derive(Debug)]
pub struct FilterBandwidthController {
    target: Ww<BiQuadFilter>,
}
impl SinksControl for FilterBandwidthController {
    fn handle_control(&mut self, _clock: &Clock, _value: f32) {
        if let Some(_target) = self.target.upgrade() {
            //            target.borrow_mut().set_bandwidth(value);
        }
    }
}

#[derive(Debug)]
pub struct FilterDbGainController {
    target: Ww<BiQuadFilter>,
}
impl SinksControl for FilterDbGainController {
    fn handle_control(&mut self, _clock: &Clock, _value: f32) {
        if let Some(_target) = self.target.upgrade() {
            //            target.borrow_mut().set_db_gain(value);
        }
    }
}

impl MakesControlSink for BiQuadFilter {
    fn make_control_sink(&self, param_name: &str) -> Option<Box<dyn SinksControl>> {
        if self.me.strong_count() != 0 {
            match param_name {
                Self::CONTROL_PARAM_CUTOFF => Some(Box::new(FilterCutoffController {
                    target: wrc_clone(&self.me),
                })),
                Self::CONTROL_PARAM_Q => Some(Box::new(FilterQController {
                    target: wrc_clone(&self.me),
                })),
                Self::CONTROL_PARAM_BANDWIDTH => Some(Box::new(FilterBandwidthController {
                    target: wrc_clone(&self.me),
                })),
                Self::CONTROL_PARAM_DB_GAIN => Some(Box::new(FilterDbGainController {
                    target: wrc_clone(&self.me),
                })),
                _ => None,
            }
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct AdsrEnvelopeNoteController {
    target: Ww<AdsrEnvelope>,
}
impl SinksControl for AdsrEnvelopeNoteController {
    fn handle_control(&mut self, clock: &Clock, value: f32) {
        if let Some(target) = self.target.upgrade() {
            target.borrow_mut().handle_note_event(clock, value == 1.0);
        }
    }
}
impl MakesControlSink for AdsrEnvelope {
    fn make_control_sink(&self, param_name: &str) -> Option<Box<dyn SinksControl>> {
        if self.me.strong_count() != 0 {
            match param_name {
                Self::CONTROL_PARAM_NOTE => Some(Box::new(AdsrEnvelopeNoteController {
                    target: wrc_clone(&self.me),
                })),
                _ => None,
            }
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct GainLevelController {
    target: Ww<Gain>,
}
impl SinksControl for GainLevelController {
    fn handle_control(&mut self, _clock: &Clock, value: f32) {
        if let Some(target) = self.target.upgrade() {
            target.borrow_mut().set_level(value);
        }
    }
}
impl MakesControlSink for Gain {
    fn make_control_sink(&self, param_name: &str) -> Option<Box<dyn SinksControl>> {
        if self.me.strong_count() != 0 {
            match param_name {
                Self::CONTROL_PARAM_CEILING => Some(Box::new(GainLevelController {
                    target: wrc_clone(&self.me),
                })),
                _ => None,
            }
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct OscillatorFrequencyController {
    target: Ww<Oscillator>,
}
impl SinksControl for OscillatorFrequencyController {
    fn handle_control(&mut self, _clock: &Clock, value: f32) {
        if let Some(target) = self.target.upgrade() {
            target.borrow_mut().set_frequency(value);
        }
    }
}
impl MakesControlSink for Oscillator {
    fn make_control_sink(&self, param_name: &str) -> Option<Box<dyn SinksControl>> {
        if self.me.strong_count() != 0 {
            match param_name {
                Self::CONTROL_PARAM_FREQUENCY => Some(Box::new(OscillatorFrequencyController {
                    target: wrc_clone(&self.me),
                })),
                _ => None,
            }
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct LimiterMinLevelController {
    target: Ww<Limiter>,
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
    target: Ww<Limiter>,
}
impl SinksControl for LimiterMaxLevelController {
    fn handle_control(&mut self, _clock: &Clock, value: f32) {
        if let Some(target) = self.target.upgrade() {
            target.borrow_mut().set_max(value);
        }
    }
}
impl MakesControlSink for Limiter {
    fn make_control_sink(&self, param_name: &str) -> Option<Box<dyn SinksControl>> {
        if self.me.strong_count() != 0 {
            match param_name {
                Self::CONTROL_PARAM_MIN => Some(Box::new(LimiterMinLevelController {
                    target: wrc_clone(&self.me),
                })),
                Self::CONTROL_PARAM_MAX => Some(Box::new(LimiterMaxLevelController {
                    target: wrc_clone(&self.me),
                })),
                _ => None,
            }
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct BitcrusherBitCountController {
    target: Ww<Bitcrusher>,
}
impl SinksControl for BitcrusherBitCountController {
    fn handle_control(&mut self, _clock: &Clock, value: f32) {
        if let Some(target) = self.target.upgrade() {
            target.borrow_mut().set_bits_to_crush(value as u8); // TODO: are we only (0.0..=1.0)?
        }
    }
}
impl MakesControlSink for Bitcrusher {
    fn make_control_sink(&self, param_name: &str) -> Option<Box<dyn SinksControl>> {
        if self.me.strong_count() != 0 {
            match param_name {
                Self::CONTROL_PARAM_BITS_TO_CRUSH => Some(Box::new(BitcrusherBitCountController {
                    target: wrc_clone(&self.me),
                })),
                _ => None,
            }
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct MixerController {
    target: Ww<Mixer>,
}
impl SinksControl for MixerController {
    fn handle_control(&mut self, _clock: &Clock, _value: f32) {
        if self.target.upgrade().is_some() {
            // Mixer doesn't have any adjustable parameters!
        }
    }
}
impl MakesControlSink for Mixer {
    fn make_control_sink(&self, _param_name: &str) -> Option<Box<dyn SinksControl>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(MixerController {
                target: wrc_clone(&self.me),
            }))
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct ArpeggiatorNothingController {
    target: Ww<Arpeggiator>,
}
impl SinksControl for ArpeggiatorNothingController {
    fn handle_control(&mut self, _clock: &Clock, value: f32) {
        if let Some(target) = self.target.upgrade() {
            target.borrow_mut().set_nothing(value);
        }
    }
}
impl MakesControlSink for Arpeggiator {
    fn make_control_sink(&self, _param_name: &str) -> Option<Box<dyn SinksControl>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(ArpeggiatorNothingController {
                target: wrc_clone(&self.me),
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
        common::{rrc, MonoSample},
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
        let path = ControlPath {
            note_value: Some(BeatValue::Quarter),
            steps: step_vec,
        };

        let mut clock = WatchedClock::new();
        let target = TestMidiSink::new_wrapped();
        if let Some(target_control_sink) = target
            .borrow()
            .make_control_sink(TestMidiSink::CONTROL_PARAM_DEFAULT)
        {
            let trip = rrc(ControlTrip::new(target_control_sink));
            trip.borrow_mut().add_path(&path);
            clock.add_watcher(trip);
        }

        clock.add_watcher(rrc(TestValueChecker {
            values: VecDeque::from(vec![0.9, 0.1, 0.2, 0.3]),
            target,
            checkpoint: 0.0,
            checkpoint_delta: 1.0,
            time_unit: ClockTimeUnit::Beats,
        }));

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
        let path = ControlPath {
            note_value: Some(BeatValue::Quarter),
            steps: step_vec,
        };

        let mut clock = WatchedClock::new();
        let target = TestMidiSink::new_wrapped();
        if let Some(target_control_sink) = target
            .borrow()
            .make_control_sink(TestMidiSink::CONTROL_PARAM_DEFAULT)
        {
            let trip = rrc(ControlTrip::new(target_control_sink));
            trip.borrow_mut().add_path(&path);
            clock.add_watcher(trip);
        }

        clock.add_watcher(rrc(TestValueChecker {
            values: VecDeque::from(interpolated_values),
            target,
            checkpoint: 0.0,
            checkpoint_delta: 0.5,
            time_unit: ClockTimeUnit::Beats,
        }));

        let mut samples_out = Vec::<MonoSample>::new();
        let mut o = TestOrchestrator::new();
        o.start(&mut clock, &mut samples_out);
    }
}
