pub(crate) mod arpeggiator;
pub(crate) mod orchestrator;
pub(crate) mod sequencers;

use crate::clock::{Clock, ClockTimeUnit};
use crate::common::OldMonoSample;
use crate::instruments::envelopes::{EnvelopeFunction, EnvelopeStep, SteppedEnvelope};
use crate::messages::{EntityMessage, MessageBounds};
use crate::settings::controllers::ControlStep;
use crate::traits::{HasUid, IsController, Response, Terminates, Updateable};
use crate::TimeSignature;
use crate::{clock::BeatValue, settings::controllers::ControlPathSettings};
use core::fmt::Debug;
use crossbeam::deque::Worker;
use groove_macros::Uid;
use std::marker::PhantomData;
use std::ops::Range;

/// A Performance holds the output of an Orchestrator run.
#[derive(Debug)]
pub struct Performance {
    pub sample_rate: usize,
    pub worker: Worker<OldMonoSample>,
}

impl Performance {
    pub fn new_with(sample_rate: usize) -> Self {
        Self {
            sample_rate,
            worker: Worker::<OldMonoSample>::new_fifo(),
        }
    }
}

/// ControlTrip, ControlPath, and ControlStep help with
/// [automation](https://en.wikipedia.org/wiki/Track_automation). Briefly, a
/// ControlTrip consists of ControlSteps stamped out of ControlPaths, and
/// ControlSteps are generic EnvelopeSteps that SteppedEnvelope uses.
///
/// A ControlTrip is one automation track, which can run as long as the whole
/// song. For now, it controls one parameter of one target.
#[derive(Debug, Uid)]
pub struct ControlTrip<M: MessageBounds> {
    uid: usize,
    cursor_beats: f32,
    current_value: f32,
    envelope: SteppedEnvelope,
    is_finished: bool,

    _phantom: PhantomData<M>,
}
impl<M: MessageBounds> IsController for ControlTrip<M> {}
impl<M: MessageBounds> Updateable for ControlTrip<M> {
    default type Message = M;

    default fn update(
        &mut self,
        _clock: &Clock,
        _message: Self::Message,
    ) -> Response<Self::Message> {
        Response::none()
    }
}
impl<M: MessageBounds> Terminates for ControlTrip<M> {
    fn is_finished(&self) -> bool {
        self.is_finished
    }
}
impl<M: MessageBounds> Default for ControlTrip<M> {
    fn default() -> Self {
        Self::new()
    }
}
impl<M: MessageBounds> ControlTrip<M> {
    const CURSOR_BEGIN: f32 = 0.0;

    pub fn new() -> Self {
        Self {
            uid: usize::default(),
            cursor_beats: Self::CURSOR_BEGIN,
            current_value: f32::MAX, // TODO we want to make sure we set the target's value at start
            envelope: SteppedEnvelope::new_with_time_unit(ClockTimeUnit::Beats),
            is_finished: true,
            _phantom: Default::default(),
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

    fn tick(&mut self, clock: &Clock) -> bool {
        let time = self.envelope.time_for_unit(clock);
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
    }
}
impl Updateable for ControlTrip<EntityMessage> {
    type Message = EntityMessage;

    fn update(&mut self, clock: &Clock, message: Self::Message) -> Response<Self::Message> {
        match message {
            Self::Message::Tick => {
                if self.tick(clock) {
                    // tick() tells us that our value has changed, so let's tell
                    // the world about that.
                    return Response::single(Self::Message::ControlF32(self.current_value));
                }
            }
            _ => todo!(),
        }
        Response::none()
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
        entities::BoxedEntity,
        messages::tests::TestMessage,
        traits::{
            TestEffect, TestEffectControlParams, TestInstrument, TestInstrumentControlParams,
        },
        Orchestrator,
    };

    #[test]
    fn test_flat_step() {
        let mut clock = Clock::default();
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

        let mut o = Box::new(Orchestrator::<TestMessage>::default());
        let effect_uid = o.add(
            None,
            BoxedEntity::TestEffect(Box::new(TestEffect::<EntityMessage>::new_with_test_values(
                &[0.9, 0.1, 0.2, 0.3],
                0.0,
                1.0,
                ClockTimeUnit::Beats,
            ))),
        );
        let mut trip = ControlTrip::<EntityMessage>::default();
        trip.add_path(&clock.settings().time_signature(), &path);
        let controller_uid = o.add(None, BoxedEntity::ControlTrip(Box::new(trip)));

        // TODO: hmmm, effect with no audio source plugged into its input!
        let _ = o.connect_to_main_mixer(effect_uid);

        o.link_control(
            controller_uid,
            effect_uid,
            &TestEffectControlParams::MyValue.to_string(),
        );

        let _ = o.run(&mut clock);

        // We advance the clock one slice before checking whether the loop is
        // done, so the clock actually should be one slice beyond the number of
        // samples we actually get.
        let expected_final_sample =
            (step_vec_len as f32 * (60.0 / clock.bpm()) * clock.sample_rate() as f32).ceil()
                as usize;
        assert_eq!(clock.samples(), expected_final_sample + 1);
    }

    #[test]
    fn test_slope_step() {
        let mut clock = Clock::default();
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

        let mut o = Box::new(Orchestrator::<TestMessage>::default());
        let instrument = Box::new(TestInstrument::<EntityMessage>::new_with_test_values(
            INTERPOLATED_VALUES,
            0.0,
            0.5,
            ClockTimeUnit::Beats,
        ));
        let instrument_uid = o.add(None, BoxedEntity::TestInstrument(instrument));
        let _ = o.connect_to_main_mixer(instrument_uid);
        let mut trip = Box::new(ControlTrip::<EntityMessage>::default());
        trip.add_path(&clock.settings().time_signature(), &path);
        let controller_uid = o.add(None, BoxedEntity::ControlTrip(trip));
        o.link_control(
            controller_uid,
            instrument_uid,
            &TestInstrumentControlParams::FakeValue.to_string(),
        );

        let _ = o.run(&mut clock);

        let expected_final_sample =
            (step_vec_len as f32 * (60.0 / clock.bpm()) * clock.sample_rate() as f32).ceil()
                as usize;
        assert_eq!(clock.samples(), expected_final_sample + 1);
    }
}
