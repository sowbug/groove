// Copyright (c) 2023 Mike Tsao. All rights reserved.

pub use arpeggiator::Arpeggiator;
pub use control_trip::{ControlPath, ControlStep, ControlTrip};
pub use lfo::LfoController;
pub use patterns::{Note, Pattern, PatternManager, PatternMessage, PatternProgrammer};
pub use sequencers::{MidiSmfReader, MidiTickSequencer, Sequencer};

mod arpeggiator;
mod control_trip;
mod lfo;
mod patterns;
mod sequencers;

use crate::EntityMessage;
use groove_core::{
    midi::HandlesMidi,
    traits::{IsController, IsEffect, Resets, TicksWithMessages, TransformsAudio},
    BipolarNormal, Sample, StereoSample,
};
use groove_macros::{Control, Uid};
use std::str::FromStr;
use strum_macros::{Display, EnumString, FromRepr};

/// [Timer] runs for a specified amount of time, then indicates that it's done.
/// It is useful when you need something to happen after a certain amount of
/// wall-clock time, rather than musical time.
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

// TODO: needs tests!
/// [Trigger] issues a control signal after a specified amount of time.
#[derive(Debug, Uid)]
pub struct Trigger {
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
    use crate::controllers::Trigger;
    use groove_core::traits::TicksWithMessages;

    #[test]
    fn instantiate_trigger() {
        let mut trigger = Trigger::new_with(44100, 1.0, 0.5);

        // asserting that 5 returned 5 confirms that the trigger isn't done yet.
        let (m, count) = trigger.tick(5);
        assert!(m.is_none());
        assert_eq!(count, 5);
    }
}
