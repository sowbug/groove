use crate::{
    clock::{BeatValue, Clock, ClockTimeUnit, TimeSignature},
    instruments::envelopes::{EnvelopeFunction, EnvelopeStep, SteppedEnvelope},
    messages::EntityMessage,
    settings::{
        controllers::{ControlPathSettings, ControlStep},
        ClockSettings,
    },
};
use core::fmt::Debug;
use groove_core::{
    midi::HandlesMidi,
    traits::{HasUid, IsController, Resets, Ticks, TicksWithMessages},
    SignalType,
};
use groove_macros::Uid;
use std::ops::Range;

/// ControlTrip, ControlPath, and ControlStep help with
/// [automation](https://en.wikipedia.org/wiki/Track_automation). Briefly, a
/// ControlTrip consists of ControlSteps stamped out of ControlPaths, and
/// ControlSteps are generic EnvelopeSteps that SteppedEnvelope uses.
///
/// A ControlTrip is one automation track, which can run as long as the whole
/// song. For now, it controls one parameter of one target.
#[derive(Debug, Uid)]
pub struct ControlTrip {
    uid: usize,
    cursor_beats: f64,
    current_value: SignalType,
    envelope: SteppedEnvelope,
    is_finished: bool,

    temp_hack_clock: Clock,
}
impl IsController<EntityMessage> for ControlTrip {}
impl HandlesMidi for ControlTrip {}
impl ControlTrip {
    const CURSOR_BEGIN: f64 = 0.0;

    pub fn new_with(clock_settings: &ClockSettings) -> Self {
        Self {
            uid: usize::default(),
            cursor_beats: Self::CURSOR_BEGIN,
            current_value: f64::MAX, // TODO we want to make sure we set the target's value at start
            envelope: SteppedEnvelope::new_with_time_unit(ClockTimeUnit::Beats),
            is_finished: true,
            temp_hack_clock: Clock::new_with(clock_settings),
        }
    }

    #[allow(dead_code)]
    pub fn reset_cursor(&mut self) {
        self.cursor_beats = Self::CURSOR_BEGIN;
    }

    // TODO: assert that these are added in time order, as SteppedEnvelope
    // currently isn't smart enough to handle out-of-order construction
    pub fn add_path(&mut self, time_signature: &TimeSignature, path: &ControlPath) {
        // TODO: this is duplicated in programmers.rs. Refactor.
        let path_note_value = if path.note_value.is_some() {
            path.note_value.as_ref().unwrap().clone()
        } else {
            time_signature.beat_value()
        };

        // If the time signature is 4/4 and the path is also quarter-notes, then
        // the multiplier is 1.0 because no correction is needed.
        //
        // If it's 4/4 and eighth notes, for example, the multiplier is 0.5,
        // because each path step represents only a half-beat.
        let path_multiplier =
            BeatValue::divisor(time_signature.beat_value()) / BeatValue::divisor(path_note_value);
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
                    end: self.cursor_beats + path_multiplier,
                },
                start_value,
                end_value,
                step_function,
            });
            self.cursor_beats += path_multiplier;
        }
        self.is_finished = false;
    }
}
impl Resets for ControlTrip {}
impl TicksWithMessages<EntityMessage> for ControlTrip {
    type Message = EntityMessage;

    fn tick(&mut self, tick_count: usize) -> (std::option::Option<Vec<Self::Message>>, usize) {
        let mut v = Vec::default();
        let mut ticks_completed = tick_count;
        for i in 0..tick_count {
            self.temp_hack_clock.tick(1);
            let has_value_changed = {
                let time = self.envelope.time_for_unit(&self.temp_hack_clock);
                let step = self.envelope.step_for_time(time);
                if step.interval.contains(&time) {
                    let value = self.envelope.value_for_step_at_time(step, time);

                    let last_value = self.current_value;
                    self.current_value = value;
                    self.is_finished = time >= step.interval.end;
                    self.current_value != last_value
                } else {
                    // This is a drastic response to a tick that's out of range. It
                    // might be better to limit it to times that are later than the
                    // covered range. We're likely to hit ControlTrips that start beyond
                    // time zero.
                    self.is_finished = true;
                    false
                }
            };
            if self.is_finished {
                ticks_completed = i;
                break;
            }
            if has_value_changed {
                // our value has changed, so let's tell the world about that.
                v.push(EntityMessage::ControlF32(self.current_value as f32));
            }
        }
        if v.is_empty() {
            (None, ticks_completed)
        } else {
            (Some(v), ticks_completed)
        }
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
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        controllers::orchestrator::Orchestrator, effects::TestEffect,
        effects::TestEffectControlParams, entities::Entity, instruments::TestInstrument,
        instruments::TestInstrumentControlParams,
    };
    use groove_core::StereoSample;

    #[test]
    fn test_flat_step() {
        let clock = Clock::default();
        let step_vec = vec![
            ControlStep::Flat { value: 0.9 },
            ControlStep::Flat { value: 0.1 },
            ControlStep::Flat { value: 0.2 },
            ControlStep::Flat { value: 0.3 },
        ];
        let step_vec_len = step_vec.len();
        let path = ControlPath {
            note_value: Some(BeatValue::Quarter),
            steps: step_vec,
        };

        let mut o = Box::new(Orchestrator::new_with_clock_settings(clock.settings()));
        let effect_uid = o.add(
            None,
            Entity::TestEffect(Box::new(TestEffect::new_with_test_values(
                &[0.9, 0.1, 0.2, 0.3],
                0.0,
                1.0,
                ClockTimeUnit::Beats,
            ))),
        );
        let mut trip = ControlTrip::new_with(clock.settings());
        trip.add_path(&clock.settings().time_signature(), &path);
        let controller_uid = o.add(None, Entity::ControlTrip(Box::new(trip)));

        // TODO: hmmm, effect with no audio source plugged into its input!
        let _ = o.connect_to_main_mixer(effect_uid);

        let _ = o.link_control(
            controller_uid,
            effect_uid,
            &TestEffectControlParams::MyValue.to_string(),
        );

        let mut sample_buffer = [StereoSample::SILENCE; 64];
        let samples = o.run(&mut sample_buffer).unwrap();

        let expected_sample_len =
            (step_vec_len as f32 * (60.0 / clock.bpm()) * clock.sample_rate() as f32).ceil()
                as usize;
        assert_eq!(samples.len(), expected_sample_len);
    }

    #[test]
    fn test_slope_step() {
        let clock = Clock::default();
        let step_vec = vec![
            ControlStep::new_slope(0.0, 1.0),
            ControlStep::new_slope(1.0, 0.5),
            ControlStep::new_slope(1.0, 0.0),
            ControlStep::new_slope(0.0, 1.0),
        ];
        let step_vec_len = step_vec.len();
        const INTERPOLATED_VALUES: &[f32] = &[0.0, 0.5, 1.0, 0.75, 1.0, 0.5, 0.0, 0.5, 1.0];
        let path = ControlPath {
            note_value: Some(BeatValue::Quarter),
            steps: step_vec,
        };

        let mut o = Box::new(Orchestrator::new_with_clock_settings(clock.settings()));
        let instrument = Box::new(TestInstrument::new_with_test_values(
            clock.sample_rate(),
            INTERPOLATED_VALUES,
            0.0,
            0.5,
            ClockTimeUnit::Beats,
        ));
        let instrument_uid = o.add(None, Entity::TestInstrument(instrument));
        let _ = o.connect_to_main_mixer(instrument_uid);
        let mut trip = Box::new(ControlTrip::new_with(clock.settings()));
        trip.add_path(&clock.settings().time_signature(), &path);
        let controller_uid = o.add(None, Entity::ControlTrip(trip));
        let _ = o.link_control(
            controller_uid,
            instrument_uid,
            &TestInstrumentControlParams::FakeValue.to_string(),
        );

        let mut sample_buffer = [StereoSample::SILENCE; 64];
        let samples = o.run(&mut sample_buffer).unwrap();

        let expected_sample_len =
            (step_vec_len as f32 * (60.0 / clock.bpm()) * clock.sample_rate() as f32).ceil()
                as usize;
        assert_eq!(samples.len(), expected_sample_len);
    }
}
