pub(crate) mod arpeggiator;
pub(crate) mod orchestrator;
pub(crate) mod sequencers;

use crate::clock::{Clock, ClockTimeUnit};
use crate::instruments::envelopes::{EnvelopeFunction, EnvelopeStep, SteppedEnvelope};
use crate::messages::{GrooveMessage, MessageBounds};
use crate::settings::controllers::ControlStep;
use crate::traits::{EvenNewerCommand, HasUid, IsController, Terminates, Updateable};
use crate::{clock::BeatValue, settings::controllers::ControlPathSettings};
use core::fmt::Debug;
use std::marker::PhantomData;
use std::ops::Range;

/// ControlTrip, ControlPath, and ControlStep help with
/// [automation](https://en.wikipedia.org/wiki/Track_automation). Briefly, a
/// ControlTrip consists of ControlSteps stamped out of ControlPaths, and
/// ControlSteps are generic EnvelopeSteps that SteppedEnvelope uses.
///
/// A ControlTrip is one automation track, which can run as long as the whole
/// song. For now, it controls one parameter of one target.
#[derive(Debug, Default)]
pub(crate) struct ControlTrip<M: MessageBounds> {
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
    ) -> EvenNewerCommand<Self::Message> {
        EvenNewerCommand::none()
    }
}
impl<M: MessageBounds> Terminates for ControlTrip<M> {
    fn is_finished(&self) -> bool {
        self.is_finished
    }
}
impl<M: MessageBounds> HasUid for ControlTrip<M> {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}
impl<M: MessageBounds> ControlTrip<M> {
    const CURSOR_BEGIN: f32 = 0.0;

    #[allow(dead_code)]
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
impl Updateable for ControlTrip<GrooveMessage> {
    type Message = GrooveMessage;

    fn update(&mut self, clock: &Clock, message: Self::Message) -> EvenNewerCommand<Self::Message> {
        match message {
            Self::Message::Tick => {
                if self.tick(clock) {
                    return EvenNewerCommand::single(Self::Message::ControlF32(
                        self.uid,
                        self.current_value,
                    ));
                }
            }
            GrooveMessage::Nop => todo!(),
            GrooveMessage::ControlF32(_, _) => todo!(),
            GrooveMessage::UpdateF32(_, _) => todo!(),
            GrooveMessage::Midi(_, _) => todo!(),
            GrooveMessage::Enable(_) => todo!(),
            GrooveMessage::PatternMessage(_, _) => todo!(),
            GrooveMessage::MutePressed(_) => todo!(),
            GrooveMessage::EnablePressed(_) => todo!(),
            GrooveMessage::ArpeggiatorChanged(_) => todo!(),
            GrooveMessage::BitcrusherValueChanged(_) => todo!(),
            GrooveMessage::FilterCutoffChangedAsF32(_) => todo!(),
            GrooveMessage::FilterCutoffChangedAsU8Percentage(_) => todo!(),
            GrooveMessage::GainLevelChangedAsString(_) => todo!(),
            GrooveMessage::GainLevelChangedAsU8Percentage(_) => todo!(),
            GrooveMessage::LimiterMinChanged(_) => todo!(),
            GrooveMessage::LimiterMaxChanged(_) => todo!(),
        }
        EvenNewerCommand::none()
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

    use super::{orchestrator::tests::Runner, *};
    use crate::{
        messages::tests::TestMessage,
        traits::{BoxedEntity, TestInstrument},
        Orchestrator,
    };

    impl Updateable for ControlTrip<TestMessage> {
        type Message = TestMessage;

        fn update(
            &mut self,
            clock: &Clock,
            message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            match message {
                Self::Message::Tick => {
                    if self.tick(clock) {
                        return EvenNewerCommand::single(Self::Message::ControlF32(
                            self.uid,
                            self.current_value,
                        ));
                    }
                }
                _ => todo!(),
            }
            EvenNewerCommand::none()
        }
    }

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

        let mut o = Box::new(Orchestrator::<TestMessage>::default());
        let target_uid = o.add(
            None,
            BoxedEntity::Instrument(Box::new(TestInstrument::<TestMessage>::default())),
        );
        let mut trip = ControlTrip::<TestMessage>::default();
        trip.add_path(&path);
        o.add(None, BoxedEntity::Controller(Box::new(trip)));

        // TODO: this is the whole point of this test, so re-enable soon!
        //
        // o.add_final_watcher(rrc(TestValueChecker::<TestMessage> {
        //     values: VecDeque::from(vec![0.9, 0.1, 0.2, 0.3]),
        //     target,
        //     checkpoint: 0.0,
        //     checkpoint_delta: 1.0,
        //     time_unit: ClockTimeUnit::Beats,
        // }));

        let mut clock = Clock::default();
        let mut r = Runner::default();
        let _ = r.run(&mut o, &mut clock);
    }

    #[test]
    fn test_slope_step() {
        let step_vec = vec![
            ControlStep::new_slope(0.0, 1.0),
            ControlStep::new_slope(1.0, 0.5),
            ControlStep::new_slope(1.0, 0.0),
            ControlStep::new_slope(0.0, 1.0),
        ];
        let _interpolated_values = vec![0.0, 0.5, 1.0, 0.75, 1.0, 0.5, 0.0, 0.5, 1.0];
        let path = ControlPath {
            note_value: Some(BeatValue::Quarter),
            steps: step_vec,
        };

        let mut o = Box::new(Orchestrator::default());
        let target_uid = o.add(
            None,
            BoxedEntity::Instrument(Box::new(TestInstrument::<TestMessage>::default())),
        );
        let mut trip = Box::new(ControlTrip::<TestMessage>::default());
        trip.add_path(&path);
        let controller_uid = o.add(None, BoxedEntity::Controller(trip));

        // TODO: this is the whole point of this test, so re-enable soon!
        //
        // o.add_final_watcher(rrc(TestValueChecker {
        //     values: VecDeque::from(interpolated_values),
        //     target,
        //     checkpoint: 0.0,
        //     checkpoint_delta: 0.5,
        //     time_unit: ClockTimeUnit::Beats,
        // }));

        let mut clock = Clock::default();
        let mut r = Runner::default();
        let _ = r.run(&mut o, &mut clock);
    }
}
