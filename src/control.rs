use crate::clock::{Clock, ClockTimeUnit};
use crate::effects::{
    arpeggiator::Arpeggiator, bitcrusher::Bitcrusher, filter::BiQuadFilter, gain::Gain,
    limiter::Limiter, mixer::Mixer,
};
use crate::settings::control::ControlStep;
use crate::traits::{MessageBounds, SinksUpdates, Terminates, WatchesClock};
use crate::{clock::BeatValue, settings::control::ControlPathSettings};
use crate::{
    envelopes::{AdsrEnvelope, EnvelopeFunction, EnvelopeStep, SteppedEnvelope},
    oscillators::Oscillator,
};
use core::fmt::Debug;
use std::fmt;
use std::ops::Range;
use std::str::FromStr;
use strum_macros::{Display, EnumString};

// https://boydjohnson.dev/blog/impl-debug-for-fn-type/ gave me enough clues to
// get through this.
pub trait SmallMessageGeneratorT: Fn(f32) -> SmallMessage {}
impl<F> SmallMessageGeneratorT for F where F: Fn(f32) -> SmallMessage {}
impl std::fmt::Debug for dyn SmallMessageGeneratorT {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SmallMessageGenerator")
    }
}
pub type SmallMessageGenerator = Box<dyn SmallMessageGeneratorT>;

/// ControlTrip, ControlPath, and ControlStep help with
/// [automation](https://en.wikipedia.org/wiki/Track_automation). Briefly, a
/// ControlTrip consists of ControlSteps stamped out of ControlPaths, and
/// ControlSteps are generic EnvelopeSteps that SteppedEnvelope uses.
///
/// A ControlTrip is one automation track, which can run as long as the whole
/// song. For now, it controls one parameter of one target.
pub(crate) struct ControlTrip {
    target_uid: usize,
    target_on_update: Option<SmallMessageGenerator>,
    cursor_beats: f32,

    current_value: f32,

    envelope: SteppedEnvelope,

    is_finished: bool,
}

impl fmt::Debug for ControlTrip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ControlTrip")
            .field("target_uid", &self.target_uid)
            .field("target_on_update", &self.target_on_update.is_some())
            .field("cursor_beats", &self.cursor_beats)
            .field("current_value", &self.current_value)
            .field("envelope", &self.envelope)
            .field("is_finished", &self.is_finished)
            .finish()
    }
}

impl ControlTrip {
    const CURSOR_BEGIN: f32 = 0.0;

    pub fn new(target_uid: usize, on_update: SmallMessageGenerator) -> Self {
        Self {
            target_uid,
            target_on_update: Some(on_update),
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
            // Beware: there's an O(N) debug validlity check in push_step(), so
            // this loop is O(N^2).
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

#[derive(Clone, Debug)]
pub enum SmallMessage {
    ValueChanged(f32),
    SecondValueChanged(f32),
    ThirdValueChanged(f32),
    FourthValueChanged(f32),
}

#[derive(Clone, Debug)]
pub enum BigMessage {
    SmallMessage(usize, SmallMessage),
}
impl WatchesClock for ControlTrip {
    fn tick(&mut self, clock: &Clock) -> Vec<BigMessage> {
        let mut messages = Vec::<BigMessage>::new();
        let time = self.envelope.time_for_unit(clock);
        let step = self.envelope.step_for_time(time);
        if step.interval.contains(&time) {
            let value = self.envelope.value_for_step_at_time(step, time);

            let last_value = self.current_value;
            self.current_value = value;
            if self.current_value != last_value {
                if let Some(f) = &self.target_on_update {
                    messages.push(BigMessage::SmallMessage(
                        self.target_uid,
                        (f)(self.current_value),
                    ));
                }
            }
            self.is_finished = time >= step.interval.end;
        } else {
            // This is a drastic response to a tick that's out of range. It
            // might be better to limit it to times that are later than the
            // covered range. We're likely to hit ControlTrips that start beyond
            // time zero.
            self.is_finished = true;
        }
        messages
    }
}

impl Terminates for ControlTrip {
    fn is_finished(&self) -> bool {
        self.is_finished
    }
}

/// A ControlPath makes it easier to construct sequences of ControlSteps. It's
/// just like a pattern in a pattern-based sequencer. ControlPaths aren't
/// required; they just make repetitive sequences less tedious to build.
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

// ############################################################################
// BEGIN code generated by util/generate-controllers.py

#[derive(Display, Debug, EnumString)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum AdsrEnvelopeControlParams {
    Note,
}

impl SinksUpdates for AdsrEnvelope {
    fn message_for(&self, param_name: &str) -> SmallMessageGenerator {
        if let Ok(param) = AdsrEnvelopeControlParams::from_str(param_name) {
            match param {
                AdsrEnvelopeControlParams::Note => return Box::new(SmallMessage::ValueChanged),
            }
        }
        panic!("unrecognized parameter name: {}", param_name);
    }

    fn update(&mut self, clock: &Clock, message: SmallMessage) {
        match message {
            SmallMessage::ValueChanged(value) => self.set_note(clock, value),
            _ => {}
        }
    }
}

#[derive(Display, Debug, EnumString)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum ArpeggiatorControlParams {
    Nothing,
}

impl SinksUpdates for Arpeggiator {
    fn message_for(&self, param_name: &str) -> SmallMessageGenerator {
        if let Ok(param) = ArpeggiatorControlParams::from_str(param_name) {
            match param {
                ArpeggiatorControlParams::Nothing => return Box::new(SmallMessage::ValueChanged),
            }
        }
        panic!("unrecognized parameter name: {}", param_name);
    }

    fn update(&mut self, _clock: &Clock, message: SmallMessage) {
        match message {
            SmallMessage::ValueChanged(value) => self.set_nothing(value),
            _ => {}
        }
    }
}

#[derive(Display, Debug, EnumString)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum BiQuadFilterControlParams {
    Bandwidth,
    #[strum(serialize = "cutoff", serialize = "cutoff-pct")]
    CutoffPct,
    DbGain,
    Q,
}

impl SinksUpdates for BiQuadFilter {
    fn message_for(&self, param_name: &str) -> SmallMessageGenerator {
        if let Ok(param) = BiQuadFilterControlParams::from_str(param_name) {
            match param {
                BiQuadFilterControlParams::Bandwidth => {
                    return Box::new(SmallMessage::ValueChanged)
                }
                BiQuadFilterControlParams::CutoffPct => {
                    return Box::new(SmallMessage::SecondValueChanged)
                }
                BiQuadFilterControlParams::DbGain => {
                    return Box::new(SmallMessage::ThirdValueChanged)
                }
                BiQuadFilterControlParams::Q => return Box::new(SmallMessage::FourthValueChanged),
            }
        }
        panic!("unrecognized parameter name: {}", param_name);
    }

    fn update(&mut self, _clock: &Clock, message: SmallMessage) {
        match message {
            SmallMessage::ValueChanged(value) => self.set_bandwidth(value),
            SmallMessage::SecondValueChanged(value) => self.set_cutoff_pct(value),
            SmallMessage::ThirdValueChanged(value) => self.set_db_gain(value),
            SmallMessage::FourthValueChanged(value) => self.set_q(value),
        }
    }
}

#[derive(Display, Debug, EnumString)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum BitcrusherControlParams {
    #[strum(serialize = "bits-to-crush", serialize = "bits-to-crush-pct")]
    BitsToCrushPct,
}

impl SinksUpdates for Bitcrusher {
    fn message_for(&self, param_name: &str) -> SmallMessageGenerator {
        if let Ok(param) = BitcrusherControlParams::from_str(param_name) {
            match param {
                BitcrusherControlParams::BitsToCrushPct => {
                    return Box::new(SmallMessage::ValueChanged)
                }
            }
        }
        panic!("unrecognized parameter name: {}", param_name);
    }

    fn update(&mut self, _clock: &Clock, message: SmallMessage) {
        match message {
            SmallMessage::ValueChanged(value) => self.set_bits_to_crush_pct(value),
            _ => {}
        }
    }
}

#[derive(Display, Debug, EnumString)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum GainControlParams {
    Ceiling,
}

impl SinksUpdates for Gain {
    fn message_for(&self, param_name: &str) -> SmallMessageGenerator {
        if let Ok(param) = GainControlParams::from_str(param_name) {
            match param {
                GainControlParams::Ceiling => return Box::new(SmallMessage::ValueChanged),
            }
        }
        panic!("unrecognized parameter name: {}", param_name);
    }

    fn update(&mut self, _clock: &Clock, message: SmallMessage) {
        match message {
            SmallMessage::ValueChanged(value) => self.set_ceiling(value),
            _ => {}
        }
    }
}

#[derive(Display, Debug, EnumString)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum LimiterControlParams {
    Max,
    Min,
}

impl SinksUpdates for Limiter {
    fn message_for(&self, param_name: &str) -> SmallMessageGenerator {
        if let Ok(param) = LimiterControlParams::from_str(param_name) {
            match param {
                LimiterControlParams::Max => return Box::new(SmallMessage::ValueChanged),
                LimiterControlParams::Min => return Box::new(SmallMessage::SecondValueChanged),
            }
        }
        panic!("unrecognized parameter name: {}", param_name);
    }

    fn update(&mut self, _clock: &Clock, message: SmallMessage) {
        match message {
            SmallMessage::ValueChanged(value) => self.set_max(value),
            SmallMessage::SecondValueChanged(value) => self.set_min(value),
            _ => {}
        }
    }
}

#[derive(Display, Debug, EnumString)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum MixerControlParams {}

impl<M: MessageBounds> SinksUpdates for Mixer<M> {
    fn message_for(&self, param_name: &str) -> SmallMessageGenerator {
        panic!("unrecognized parameter name: {}", param_name);
    }

    fn update(&mut self, _clock: &Clock, _message: SmallMessage) {}
}

#[derive(Display, Debug, EnumString)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum OscillatorControlParams {
    Frequency,
}

impl SinksUpdates for Oscillator {
    fn message_for(&self, param_name: &str) -> SmallMessageGenerator {
        if let Ok(param) = OscillatorControlParams::from_str(param_name) {
            match param {
                OscillatorControlParams::Frequency => return Box::new(SmallMessage::ValueChanged),
            }
        }
        panic!("unrecognized parameter name: {}", param_name);
    }

    fn update(&mut self, _clock: &Clock, message: SmallMessage) {
        match message {
            SmallMessage::ValueChanged(value) => self.set_frequency(value),
            _ => {}
        }
    }
}

// END code generated by util/generate-controllers.py
// ############################################################################

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use crate::{
        clock::WatchedClock,
        common::{rrc, rrc_downgrade},
        messages::tests::TestMessage,
        utils::tests::{OldTestOrchestrator, TestMidiSink, TestValueChecker},
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
        let mut o = OldTestOrchestrator::new();
        let target = TestMidiSink::new_wrapped();
        const UID: usize = 42;
        o.updateables
            .insert(UID, rrc_downgrade::<TestMidiSink<TestMessage>>(&target));
        let trip = rrc(ControlTrip::new(UID, Box::new(SmallMessage::ValueChanged)));
        trip.borrow_mut().add_path(&path);
        clock.add_watcher(trip);

        // TODO: this is the whole point of this test, so re-enable soon!
        //
        // o.add_final_watcher(rrc(TestValueChecker::<TestMessage> {
        //     values: VecDeque::from(vec![0.9, 0.1, 0.2, 0.3]),
        //     target,
        //     checkpoint: 0.0,
        //     checkpoint_delta: 1.0,
        //     time_unit: ClockTimeUnit::Beats,
        // }));

        let _ = o.run_until_completion(&mut clock);
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
        let mut o = OldTestOrchestrator::new();
        let target = TestMidiSink::new_wrapped();
        const UID: usize = 42;
        o.updateables
            .insert(UID, rrc_downgrade::<TestMidiSink<TestMessage>>(&target));
        let trip = rrc(ControlTrip::new(UID, Box::new(SmallMessage::ValueChanged)));
        trip.borrow_mut().add_path(&path);
        clock.add_watcher(trip);

        // TODO: this is the whole point of this test, so re-enable soon!
        //
        // o.add_final_watcher(rrc(TestValueChecker {
        //     values: VecDeque::from(interpolated_values),
        //     target,
        //     checkpoint: 0.0,
        //     checkpoint_delta: 0.5,
        //     time_unit: ClockTimeUnit::Beats,
        // }));

        let _ = o.run_until_completion(&mut clock);
    }
}
