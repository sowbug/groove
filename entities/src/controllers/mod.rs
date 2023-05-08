// Copyright (c) 2023 Mike Tsao. All rights reserved.

#[cfg(feature = "iced-framework")]
pub use arpeggiator::ArpeggiatorMessage;
pub use arpeggiator::{Arpeggiator, ArpeggiatorParams};
#[cfg(feature = "iced-framework")]
pub use control_trip::ControlTripMessage;
pub use control_trip::{ControlPath, ControlStep, ControlTrip, ControlTripParams};
#[cfg(feature = "iced-framework")]
pub use lfo::LfoControllerMessage;
pub use lfo::{LfoController, LfoControllerParams};
pub use patterns::{
    Note, Pattern, PatternManager, PatternManagerParams, PatternMessage, PatternProgrammer,
};
#[cfg(feature = "iced-framework")]
pub use patterns::{PatternManagerMessage, PatternMessage};
pub use sequencers::{
    MidiSmfReader, MidiTickSequencer, MidiTickSequencerParams, Sequencer, SequencerParams,
};
#[cfg(feature = "iced-framework")]
pub use sequencers::{MidiTickSequencerMessage, SequencerMessage};

mod arpeggiator;
mod control_trip;
mod lfo;
mod patterns;
mod sequencers;

use crate::EntityMessage;
use groove_core::{
    midi::{HandlesMidi, MidiChannel},
    traits::{IsController, IsEffect, Performs, Resets, TicksWithMessages, TransformsAudio},
    BipolarNormal, ParameterType, Sample, StereoSample,
};
use groove_proc_macros::{Control, Params, Uid};

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
#[derive(Clone, Copy, Debug)]
#[cfg_attr(
    feature = "serialization",
    derive(Serialize, Deserialize),
    serde(rename = "midi-in", rename_all = "kebab-case")
)]
pub struct MidiChannelInputParams {
    pub midi_in: MidiChannel,
}
#[derive(Clone, Copy, Debug)]
#[cfg_attr(
    feature = "serialization",
    derive(Serialize, Deserialize),
    serde(rename = "midi-out", rename_all = "kebab-case")
)]
pub struct MidiChannelOutputParams {
    pub midi_out: MidiChannel,
}

/// [Timer] runs for a specified amount of time, then indicates that it's done.
/// It is useful when you need something to happen after a certain amount of
/// wall-clock time, rather than musical time.
#[derive(Debug, Control, Params, Uid)]
pub struct Timer {
    uid: usize,

    #[control]
    #[params]
    seconds: ParameterType,

    sample_rate: usize,

    has_more_work: bool,
    ticks: usize,
    is_performing: bool,
}
impl Timer {
    pub fn new_with(params: &TimerParams) -> Self {
        Self {
            uid: Default::default(),
            sample_rate: Default::default(),
            seconds: params.seconds(),

            has_more_work: Default::default(),
            ticks: Default::default(),
            is_performing: false,
        }
    }

    pub fn seconds(&self) -> ParameterType {
        self.seconds
    }

    pub fn set_seconds(&mut self, seconds: ParameterType) {
        self.seconds = seconds;
    }

    #[cfg(feature = "iced-framework")]
    pub fn update(&mut self, message: TimerMessage) {
        match message {
            TimerMessage::Timer(s) => *self = Self::new_with(s),
            _ => self.derived_update(message),
        }
    }
}
impl IsController for Timer {}
impl HandlesMidi for Timer {}
impl Resets for Timer {
    fn reset(&mut self, sample_rate: usize) {
        self.sample_rate = sample_rate;
        self.skip_to_start();
    }
}
impl TicksWithMessages for Timer {
    type Message = EntityMessage;

    fn tick(&mut self, tick_count: usize) -> (Option<Vec<Self::Message>>, usize) {
        let mut ticks_completed = tick_count;
        for i in 0..tick_count {
            self.has_more_work = (self.ticks as f64 / self.sample_rate as f64) < self.seconds;
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
impl Performs for Timer {
    fn play(&mut self) {
        self.is_performing = true;
    }

    fn stop(&mut self) {
        self.is_performing = false;
    }

    fn skip_to_start(&mut self) {
        self.ticks = 0;
    }
}

// TODO: needs tests!
/// [Trigger] issues a control signal after a specified amount of time.
#[derive(Debug, Control, Params, Uid)]
pub struct Trigger {
    uid: usize,

    #[control]
    #[params]
    seconds: ParameterType,

    #[control]
    #[params]
    value: f32,

    timer: Timer,
    has_triggered: bool,
    is_performing: bool,
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
                Some(vec![EntityMessage::ControlF32(self.value())]),
                ticks_completed,
            )
        } else {
            (None, ticks_completed)
        }
    }
}
impl Resets for Trigger {
    fn reset(&mut self, sample_rate: usize) {
        self.timer.reset(sample_rate)
    }
}
impl HandlesMidi for Trigger {}
impl Performs for Trigger {
    fn play(&mut self) {
        self.is_performing = true;
        self.timer.play();
    }

    fn stop(&mut self) {
        self.is_performing = false;
        self.timer.stop();
    }

    fn skip_to_start(&mut self) {
        self.has_triggered = false;
        self.timer.skip_to_start();
    }
}
impl Trigger {
    pub fn new_with(params: &TriggerParams) -> Self {
        Self {
            uid: Default::default(),
            timer: Timer::new_with(&TimerParams {
                seconds: params.seconds(),
            }),
            has_triggered: false,
            seconds: params.seconds(),
            value: params.value(),
            is_performing: false,
        }
    }

    pub fn seconds(&self) -> f64 {
        self.seconds
    }

    pub fn set_seconds(&mut self, seconds: ParameterType) {
        self.seconds = seconds;
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn set_value(&mut self, value: f32) {
        self.value = value;
    }

    #[cfg(feature = "iced-framework")]
    pub fn update(&mut self, message: TriggerMessage) {
        match message {
            TriggerMessage::Trigger(s) => *self = Self::new_with(s),
            _ => self.derived_update(message),
        }
    }
}

/// Uses an input signal as a control source.
#[derive(Control, Debug, Params, Uid)]
pub struct SignalPassthroughController {
    uid: usize,
    signal: BipolarNormal,
    has_signal_changed: bool,
    is_performing: bool,
}
impl IsController for SignalPassthroughController {}
impl Resets for SignalPassthroughController {}
impl TicksWithMessages for SignalPassthroughController {
    type Message = EntityMessage;

    fn tick(&mut self, _tick_count: usize) -> (std::option::Option<Vec<Self::Message>>, usize) {
        if !self.is_performing {
            return (None, 0);
        }

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
impl Performs for SignalPassthroughController {
    fn play(&mut self) {
        self.is_performing = true;
    }

    fn stop(&mut self) {
        self.is_performing = false;
    }

    fn skip_to_start(&mut self) {}
}
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
            signal: Default::default(),
            has_signal_changed: true,
            is_performing: false,
        }
    }

    #[cfg(feature = "iced-framework")]
    #[allow(unreachable_patterns)]
    pub fn update(&mut self, message: SignalPassthroughControllerMessage) {
        match message {
            SignalPassthroughControllerMessage::SignalPassthroughController(_s) => {
                *self = Self::new()
            }
            _ => self.derived_update(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        controllers::{Trigger, TriggerParams},
        tests::DEFAULT_SAMPLE_RATE,
    };
    use groove_core::traits::{Resets, TicksWithMessages};

    #[test]
    fn instantiate_trigger() {
        let mut trigger = Trigger::new_with(&TriggerParams {
            seconds: 1.0,
            value: 0.5,
        });
        trigger.reset(DEFAULT_SAMPLE_RATE);

        // asserting that 5 returned 5 confirms that the trigger isn't done yet.
        let (m, count) = trigger.tick(5);
        assert!(m.is_none());
        assert_eq!(count, 5);
    }
}
