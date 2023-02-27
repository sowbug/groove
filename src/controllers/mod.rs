pub use arpeggiator::Arpeggiator;
use midly::MidiMessage;
pub use patterns::{Note, Pattern, PatternManager, PatternMessage};
pub use sequencers::{BeatSequencer, MidiTickSequencer};

pub(crate) mod arpeggiator;
pub(crate) mod orchestrator;
pub(crate) mod patterns;
pub(crate) mod sequencers;

use crate::{
    clock::{BeatValue, Clock, ClockTimeUnit, TimeSignature},
    common::{BipolarNormal, F32ControlValue, ParameterType, SignalType, StereoSample},
    instruments::{
        envelopes::{EnvelopeFunction, EnvelopeStep, SteppedEnvelope},
        oscillators::Oscillator,
    },
    messages::EntityMessage,
    midi::{MidiChannel, MidiUtils},
    settings::{
        controllers::{ControlPathSettings, ControlStep},
        patches::WaveformType,
        ClockSettings,
    },
    traits::{
        Controllable, Generates, HandlesMidi, HasUid, IsController, Resets, Ticks,
        TicksWithMessages,
    },
};
use core::fmt::Debug;
use crossbeam::deque::Worker;
use groove_macros::{Control, Uid};
use std::str::FromStr;
use std::{collections::VecDeque, ops::Range};
use strum_macros::{Display, EnumString, FromRepr};

/// A Performance holds the output of an Orchestrator run.
#[derive(Debug)]
pub struct Performance {
    pub sample_rate: usize,
    pub worker: Worker<StereoSample>,
}

impl Performance {
    pub fn new_with(sample_rate: usize) -> Self {
        Self {
            sample_rate,
            worker: Worker::<StereoSample>::new_fifo(),
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
pub struct ControlTrip {
    uid: usize,
    cursor_beats: f64,
    current_value: SignalType,
    envelope: SteppedEnvelope,
    is_finished: bool,

    temp_hack_clock: Clock,
}
impl IsController for ControlTrip {}
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
impl TicksWithMessages for ControlTrip {
    fn tick(&mut self, tick_count: usize) -> (std::option::Option<Vec<EntityMessage>>, usize) {
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

#[derive(Control, Debug, Uid)]
pub struct LfoController {
    uid: usize,
    oscillator: Oscillator,
}
impl IsController for LfoController {}
impl Resets for LfoController {}
impl TicksWithMessages for LfoController {
    fn tick(&mut self, tick_count: usize) -> (std::option::Option<Vec<EntityMessage>>, usize) {
        self.oscillator.tick(tick_count);
        // TODO: opportunity to use from() to convert properly from 0..1 to -1..0
        (
            Some(vec![EntityMessage::ControlF32(
                BipolarNormal::from(self.oscillator.value()).value() as f32,
            )]),
            0,
        )
    }
}
impl HandlesMidi for LfoController {}
impl LfoController {
    pub fn new_with(
        clock_settings: &ClockSettings,
        waveform: WaveformType,
        frequency_hz: ParameterType,
    ) -> Self {
        Self {
            uid: Default::default(),
            oscillator: Oscillator::new_with_type_and_frequency(
                clock_settings.sample_rate(),
                waveform,
                frequency_hz as f32,
            ),
        }
    }
}

#[derive(Display, Debug, EnumString)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum TestControllerControlParams {
    Tempo,
}

enum TestControllerAction {
    Nothing,
    NoteOn,
    NoteOff,
}

#[derive(Debug)]
pub struct TestController {
    uid: usize,
    midi_channel_out: MidiChannel,

    clock_settings: ClockSettings,
    clock: Clock,

    pub tempo: f32,
    is_enabled: bool,
    is_playing: bool,

    pub checkpoint_values: VecDeque<f32>,
    pub checkpoint: f32,
    pub checkpoint_delta: f32,
    pub time_unit: ClockTimeUnit,
}
impl IsController for TestController {}
impl TicksWithMessages for TestController {
    fn tick(&mut self, tick_count: usize) -> (Option<Vec<EntityMessage>>, usize) {
        let mut v = Vec::default();
        for _ in 0..tick_count {
            self.clock.tick(1);
            // TODO self.check_values(clock);

            match self.what_to_do() {
                TestControllerAction::Nothing => {}
                TestControllerAction::NoteOn => {
                    // This is elegant, I hope. If the arpeggiator is
                    // disabled during play, and we were playing a note,
                    // then we still send the off note,
                    if self.is_enabled {
                        self.is_playing = true;
                        v.push(EntityMessage::Midi(
                            self.midi_channel_out,
                            MidiUtils::new_note_on(60, 127),
                        ));
                    }
                }
                TestControllerAction::NoteOff => {
                    if self.is_playing {
                        v.push(EntityMessage::Midi(
                            self.midi_channel_out,
                            MidiUtils::new_note_off(60, 0),
                        ));
                    }
                }
            }
        }
        if v.is_empty() {
            (None, 0)
        } else {
            (Some(v), 0)
        }
    }
}
impl Resets for TestController {
    fn reset(&mut self, sample_rate: usize) {
        self.clock_settings.set_sample_rate(sample_rate);
        self.clock = Clock::new_with(&self.clock_settings);
    }
}
impl HandlesMidi for TestController {
    fn handle_midi_message(
        &mut self,
        message: &MidiMessage,
    ) -> Option<Vec<(MidiChannel, MidiMessage)>> {
        #[allow(unused_variables)]
        match message {
            MidiMessage::NoteOff { key, vel } => self.is_enabled = false,
            MidiMessage::NoteOn { key, vel } => self.is_enabled = true,
            _ => todo!(),
        }
        None
    }
}
impl HasUid for TestController {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid
    }
}
impl TestController {
    pub fn new_with(clock_settings: &ClockSettings, midi_channel_out: MidiChannel) -> Self {
        Self::new_with_test_values(
            clock_settings,
            midi_channel_out,
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        )
    }

    pub fn new_with_test_values(
        clock_settings: &ClockSettings,
        midi_channel_out: MidiChannel,
        values: &[f32],
        checkpoint: f32,
        checkpoint_delta: f32,
        time_unit: ClockTimeUnit,
    ) -> Self {
        Self {
            uid: Default::default(),
            midi_channel_out,
            clock_settings: clock_settings.clone(),
            clock: Clock::new_with(clock_settings),
            tempo: Default::default(),
            is_enabled: Default::default(),
            is_playing: Default::default(),
            checkpoint_values: VecDeque::from(Vec::from(values)),
            checkpoint,
            checkpoint_delta,
            time_unit,
        }
    }

    fn what_to_do(&self) -> TestControllerAction {
        let beat_slice_start = self.clock.beats();
        let beat_slice_end = self.clock.next_slice_in_beats();
        let next_exact_beat = beat_slice_start.floor();
        let next_exact_half_beat = next_exact_beat + 0.5;
        if next_exact_beat >= beat_slice_start && next_exact_beat < beat_slice_end {
            return TestControllerAction::NoteOn;
        }
        if next_exact_half_beat >= beat_slice_start && next_exact_half_beat < beat_slice_end {
            return TestControllerAction::NoteOff;
        }
        TestControllerAction::Nothing
    }
}
// impl TestsValues for TestController {
//     fn has_checkpoint_values(&self) -> bool {
//         !self.checkpoint_values.is_empty()
//     }

//     fn time_unit(&self) -> &ClockTimeUnit {
//         &self.time_unit
//     }

//     fn checkpoint_time(&self) -> f32 {
//         self.checkpoint
//     }

//     fn advance_checkpoint_time(&mut self) {
//         self.checkpoint += self.checkpoint_delta;
//     }

//     fn value_to_check(&self) -> f32 {
//         self.tempo
//     }

//     fn pop_checkpoint_value(&mut self) -> Option<f32> {
//         self.checkpoint_values.pop_front()
//     }
// }

/// Timer Terminates (in the Terminates trait sense) after a specified amount of time.
#[derive(Debug, Uid)]
pub struct Timer {
    uid: usize,
    sample_rate: usize,
    time_to_run_seconds: f32,

    has_more_work: bool,
    ticks: usize,
}
impl Timer {
    pub fn new_with(sample_rate: usize, time_to_run_seconds: f32) -> Self {
        Self {
            uid: Default::default(),
            sample_rate,
            time_to_run_seconds,

            has_more_work: Default::default(),
            ticks: Default::default(),
        }
    }

    pub fn time_to_run_seconds(&self) -> f32 {
        self.time_to_run_seconds
    }
}
impl IsController for Timer {}
impl HandlesMidi for Timer {}
impl Resets for Timer {
    fn reset(&mut self, sample_rate: usize) {
        self.sample_rate = sample_rate;
        self.ticks = 0;
    }
}
impl TicksWithMessages for Timer {
    fn tick(&mut self, tick_count: usize) -> (Option<Vec<EntityMessage>>, usize) {
        let mut ticks_completed = tick_count;
        for i in 0..tick_count {
            self.has_more_work =
                (self.ticks as f32 / self.sample_rate as f32) < self.time_to_run_seconds;
            if self.has_more_work {
                self.ticks += 1;
            } else {
                ticks_completed = i;
                break;
            }
        }
        (None, ticks_completed)
    }
}

/// Trigger issues a ControlF32 message after a specified amount of time.
///
/// TODO: needs tests!
#[derive(Debug, Uid)]
pub(crate) struct Trigger {
    uid: usize,
    value: f32,

    timer: Timer,
    has_triggered: bool,
}
impl IsController for Trigger {}
impl TicksWithMessages for Trigger {
    fn tick(&mut self, tick_count: usize) -> (Option<Vec<EntityMessage>>, usize) {
        // We toss the timer's messages because we know it never returns any,
        // and we wouldn't pass them on if it did.
        let (_, ticks_completed) = self.timer.tick(tick_count);
        if ticks_completed < tick_count && !self.has_triggered {
            self.has_triggered = true;
            (
                Some(vec![EntityMessage::ControlF32(self.value)]),
                ticks_completed,
            )
        } else {
            (None, ticks_completed)
        }
    }
}
impl Resets for Trigger {}
impl HandlesMidi for Trigger {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        controllers::orchestrator::Orchestrator, effects::TestEffect,
        effects::TestEffectControlParams, entities::Entity, instruments::TestInstrument,
        instruments::TestInstrumentControlParams,
    };

    impl Trigger {
        pub fn new_with(sample_rate: usize, time_to_trigger_seconds: f32, value: f32) -> Self {
            Self {
                uid: Default::default(),
                value,
                timer: Timer::new_with(sample_rate, time_to_trigger_seconds),
                has_triggered: false,
            }
        }
    }

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
