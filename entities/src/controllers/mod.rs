// Copyright (c) 2023 Mike Tsao. All rights reserved.

pub use arpeggiator::{Arpeggiator, ArpeggiatorParams, ArpeggiatorParamsMessage};
pub use control_trip::{
    ControlPath, ControlStep, ControlTrip, ControlTripParams, ControlTripParamsMessage,
};
pub use lfo::{LfoController, LfoControllerParams, LfoControllerParamsMessage, WaveformParams};
pub use patterns::{
    Note, Pattern, PatternManager, PatternManagerParams, PatternManagerParamsMessage,
    PatternMessage, PatternProgrammer,
};
pub use sequencers::{
    MidiSmfReader, MidiTickSequencer, MidiTickSequencerParams, MidiTickSequencerParamsMessage,
    Sequencer, SequencerParams, SequencerParamsMessage,
};

mod arpeggiator;
mod control_trip;
mod lfo;
mod patterns;
mod sequencers;

use crate::EntityMessage;
use groove_core::{
    midi::{HandlesMidi, MidiChannel},
    traits::{IsController, IsEffect, Resets, TicksWithMessages, TransformsAudio},
    BipolarNormal, ParameterType, Sample, StereoSample,
};
use groove_macros::{Control, Synchronization, Uid};
use std::str::FromStr;
use strum::EnumCount;
use strum_macros::{
    Display, EnumCount as EnumCountMacro, EnumIter, EnumString, FromRepr, IntoStaticStr,
};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug)]
#[cfg_attr(
    feature = "serialization",
    derive(Serialize, Deserialize),
    serde(rename = "midi", rename_all = "kebab-case")
)]
pub struct MidiChannelParams {
    pub midi_in: MidiChannel,
    pub midi_out: MidiChannel,
}

#[derive(Clone, Copy, Debug, Default, Synchronization)]
#[cfg_attr(
    feature = "serialization",
    derive(Serialize, Deserialize),
    serde(rename = "timer", rename_all = "kebab-case")
)]
pub struct TimerParams {
    #[sync]
    pub seconds_to_run: ParameterType,
}
impl TimerParams {
    pub fn seconds_to_run(&self) -> f64 {
        self.seconds_to_run
    }

    pub fn set_seconds_to_run(&mut self, seconds_to_run: ParameterType) {
        self.seconds_to_run = seconds_to_run;
    }
}

/// [Timer] runs for a specified amount of time, then indicates that it's done.
/// It is useful when you need something to happen after a certain amount of
/// wall-clock time, rather than musical time.
#[derive(Debug, Uid)]
pub struct Timer {
    uid: usize,
    params: TimerParams,
    sample_rate: usize,

    has_more_work: bool,
    ticks: usize,
}
impl Timer {
    pub fn new_with(sample_rate: usize, params: TimerParams) -> Self {
        Self {
            uid: Default::default(),
            params,
            sample_rate,

            has_more_work: Default::default(),
            ticks: Default::default(),
        }
    }

    pub fn seconds_to_run(&self) -> ParameterType {
        self.params().seconds_to_run()
    }

    pub fn params(&self) -> TimerParams {
        self.params
    }

    pub fn update(&mut self, message: TimerParamsMessage) {
        self.params.update(message)
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
    type Message = EntityMessage;

    fn tick(&mut self, tick_count: usize) -> (Option<Vec<Self::Message>>, usize) {
        let mut ticks_completed = tick_count;
        for i in 0..tick_count {
            self.has_more_work =
                (self.ticks as f64 / self.sample_rate as f64) < self.params().seconds_to_run();
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

#[derive(Clone, Copy, Debug, Default, Synchronization)]
#[cfg_attr(
    feature = "serialization",
    derive(Serialize, Deserialize),
    serde(rename = "trigger", rename_all = "kebab-case")
)]
pub struct TriggerParams {
    #[sync]
    pub seconds_to_run: ParameterType,

    #[sync]
    pub value: f32,
}

impl TriggerParams {
    pub fn seconds_to_run(&self) -> f64 {
        self.seconds_to_run
    }

    pub fn set_seconds_to_run(&mut self, seconds_to_run: ParameterType) {
        self.seconds_to_run = seconds_to_run;
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn set_value(&mut self, value: f32) {
        self.value = value;
    }
}

// TODO: needs tests!
/// [Trigger] issues a control signal after a specified amount of time.
#[derive(Debug, Uid)]
pub struct Trigger {
    uid: usize,
    params: TriggerParams,

    timer: Timer,
    has_triggered: bool,
}
impl IsController for Trigger {}
impl TicksWithMessages for Trigger {
    type Message = EntityMessage;

    fn tick(&mut self, tick_count: usize) -> (Option<Vec<Self::Message>>, usize) {
        // We toss the timer's messages because we know it never returns any,
        // and we wouldn't pass them on if it did.
        let (_, ticks_completed) = self.timer.tick(tick_count);
        if ticks_completed < tick_count && !self.has_triggered {
            self.has_triggered = true;
            (
                Some(vec![EntityMessage::ControlF32(self.params.value())]),
                ticks_completed,
            )
        } else {
            (None, ticks_completed)
        }
    }
}
impl Resets for Trigger {}
impl HandlesMidi for Trigger {}
impl Trigger {
    pub fn new_with(sample_rate: usize, params: TriggerParams) -> Self {
        Self {
            uid: Default::default(),
            params,
            timer: Timer::new_with(
                sample_rate,
                TimerParams {
                    seconds_to_run: params.seconds_to_run(),
                },
            ),
            has_triggered: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Synchronization)]
#[cfg_attr(
    feature = "serialization",
    derive(Serialize, Deserialize),
    serde(rename = "signal-passthrough-controller", rename_all = "kebab-case")
)]
pub struct SignalPassthroughControllerParams {}
impl SignalPassthroughControllerParams {}

/// Uses an input signal as a control source.
#[derive(Control, Debug, Uid)]
pub struct SignalPassthroughController {
    uid: usize,
    params: SignalPassthroughControllerParams,
    signal: BipolarNormal,
    has_signal_changed: bool,
}
impl IsController for SignalPassthroughController {}
impl Resets for SignalPassthroughController {}
impl TicksWithMessages for SignalPassthroughController {
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
impl Default for SignalPassthroughController {
    fn default() -> Self {
        Self::new()
    }
}
impl SignalPassthroughController {
    pub fn new() -> Self {
        Self {
            uid: Default::default(),
            params: Default::default(),
            signal: Default::default(),
            has_signal_changed: true,
        }
    }

    pub fn params(&self) -> SignalPassthroughControllerParams {
        self.params
    }

    pub fn update(&mut self, message: SignalPassthroughControllerParamsMessage) {
        self.params.update(message)
    }
}

#[cfg(test)]
mod tests {
    use crate::controllers::{Trigger, TriggerParams};
    use groove_core::traits::TicksWithMessages;

    #[test]
    fn instantiate_trigger() {
        let mut trigger = Trigger::new_with(
            44100,
            TriggerParams {
                seconds_to_run: 1.0,
                value: 0.5,
            },
        );

        // asserting that 5 returned 5 confirms that the trigger isn't done yet.
        let (m, count) = trigger.tick(5);
        assert!(m.is_none());
        assert_eq!(count, 5);
    }
}
