pub use arpeggiator::Arpeggiator;
pub use control_trip::{ControlPath, ControlTrip};
pub use lfo::LfoController;
pub use patterns::{Note, Pattern, PatternManager, PatternMessage};
pub use sequencers::{BeatSequencer, MidiTickSequencer};

pub(crate) mod arpeggiator;
pub(crate) mod control_trip;
pub(crate) mod lfo;
pub(crate) mod orchestrator;
pub(crate) mod patterns;
pub(crate) mod sequencers;

use crate::messages::EntityMessage;
use core::fmt::Debug;
use crossbeam::deque::Worker;
use groove_core::{
    control::F32ControlValue,
    midi::{new_note_off, new_note_on, HandlesMidi, MidiChannel, MidiMessage},
    time::{Clock, ClockTimeUnit},
    traits::{
        Controllable, HasUid, IsController, IsEffect, Resets, Ticks, TicksWithMessages,
        TransformsAudio,
    },
    BipolarNormal, ParameterType, Sample, StereoSample,
};
use groove_macros::{Control, Uid};
use std::collections::VecDeque;
use std::str::FromStr;
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

    sample_rate: usize,
    midi_ticks_per_second: usize,
    bpm: ParameterType,
    clock: Clock,

    pub tempo: f32,
    is_enabled: bool,
    is_playing: bool,

    pub checkpoint_values: VecDeque<f32>,
    pub checkpoint: f32,
    pub checkpoint_delta: f32,
    pub time_unit: ClockTimeUnit,
}
impl IsController<EntityMessage> for TestController {}
impl TicksWithMessages<EntityMessage> for TestController {
    type Message = EntityMessage;

    fn tick(&mut self, tick_count: usize) -> (Option<Vec<Self::Message>>, usize) {
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
                            new_note_on(60, 127),
                        ));
                    }
                }
                TestControllerAction::NoteOff => {
                    if self.is_playing {
                        v.push(EntityMessage::Midi(
                            self.midi_channel_out,
                            new_note_off(60, 0),
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
        self.sample_rate = sample_rate;
        self.clock = Clock::new_with(sample_rate, self.bpm, self.midi_ticks_per_second);
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
    pub fn new_with(sample_rate: usize, bpm: ParameterType, midi_channel_out: MidiChannel) -> Self {
        Self::new_with_test_values(
            sample_rate,
            bpm,
            midi_channel_out,
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        )
    }

    pub fn new_with_test_values(
        sample_rate: usize,
        bpm: ParameterType,
        midi_channel_out: MidiChannel,
        values: &[f32],
        checkpoint: f32,
        checkpoint_delta: f32,
        time_unit: ClockTimeUnit,
    ) -> Self {
        Self {
            uid: Default::default(),
            midi_channel_out,
            sample_rate,
            midi_ticks_per_second: 9999,
            bpm,
            clock: Clock::new_with(sample_rate, bpm, 9999),
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
impl IsController<EntityMessage> for Timer {}
impl HandlesMidi for Timer {}
impl Resets for Timer {
    fn reset(&mut self, sample_rate: usize) {
        self.sample_rate = sample_rate;
        self.ticks = 0;
    }
}
impl TicksWithMessages<EntityMessage> for Timer {
    type Message = EntityMessage;

    fn tick(&mut self, tick_count: usize) -> (Option<Vec<Self::Message>>, usize) {
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
impl IsController<EntityMessage> for Trigger {}
impl TicksWithMessages<EntityMessage> for Trigger {
    type Message = EntityMessage;

    fn tick(&mut self, tick_count: usize) -> (Option<Vec<Self::Message>>, usize) {
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
/// Uses an input signal as a control source.
#[derive(Control, Debug, Uid)]
pub struct SignalPassthroughController {
    uid: usize,
    signal: BipolarNormal,
    has_signal_changed: bool,
}
impl IsController<EntityMessage> for SignalPassthroughController {}
impl Resets for SignalPassthroughController {}
impl TicksWithMessages<EntityMessage> for SignalPassthroughController {
    type Message = EntityMessage;

    fn tick(&mut self, _tick_count: usize) -> (std::option::Option<Vec<Self::Message>>, usize) {
        // We ignore tick_count because we know we won't send more than one
        // control signal during any batch of tick()s unless we also get
        // multiple transform_audio() calls. This is fine; it's exactly how
        // other controllers behave.
        (
            if self.has_signal_changed {
                self.has_signal_changed = false;
                Some(vec![EntityMessage::ControlF32(
                    (self.signal.value() as f32).abs() * -0.5, // TODO: deal with that transform
                )])
            } else {
                None
            },
            // We always return 0 for handled ticks because that's our signal
            // that we're OK terminating.
            0,
        )
    }
}
impl HandlesMidi for SignalPassthroughController {}
impl IsEffect for SignalPassthroughController {}
impl TransformsAudio for SignalPassthroughController {
    fn transform_audio(&mut self, input_sample: StereoSample) -> StereoSample {
        let averaged_sample: Sample = (input_sample.0 + input_sample.1) * 0.5;
        let as_bipolar_normal: BipolarNormal = averaged_sample.into();
        if self.signal != as_bipolar_normal {
            self.has_signal_changed = true;
            self.signal = as_bipolar_normal;
        }
        input_sample
    }

    fn transform_channel(&mut self, _channel: usize, _input_sample: Sample) -> Sample {
        // We've overridden transform_audio(), so nobody should be calling this
        // method.
        todo!();
    }
}
impl SignalPassthroughController {
    pub fn new() -> Self {
        Self {
            uid: Default::default(),
            signal: Default::default(),
            has_signal_changed: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
