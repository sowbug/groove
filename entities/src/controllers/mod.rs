// Copyright (c) 2023 Mike Tsao. All rights reserved.

#[cfg(feature = "iced-framework")]
pub use arpeggiator::ArpeggiatorMessage;
pub use arpeggiator::{Arpeggiator, ArpeggiatorParams};
pub use calculator::{Calculator, CalculatorParams};
#[cfg(feature = "iced-framework")]
pub use control_trip::ControlTripMessage;
pub use control_trip::{ControlPath, ControlStep, ControlTrip, ControlTripParams};
#[cfg(feature = "iced-framework")]
pub use lfo::LfoControllerMessage;
pub use lfo::{LfoController, LfoControllerParams};
pub use patterns::{
    NewPattern, Note, Pattern, PatternManager, PatternManagerParams, PatternProgrammer,
};
#[cfg(feature = "iced-framework")]
pub use patterns::{PatternManagerMessage, PatternMessage};
#[cfg(feature = "iced-framework")]
pub use sequencers::{MidiTickSequencerMessage, SequencerMessage};
pub use sequencers::{Sequencer, SequencerParams};

mod arpeggiator;
mod calculator;
mod control_trip;
mod lfo;
mod patterns;
mod sequencers;

use crate::EntityMessage;
use groove_core::{
    midi::{new_note_off, new_note_on, HandlesMidi, MidiChannel},
    time::{ClockTimeUnit, MusicalTime, MusicalTimeParams},
    traits::{Controls, IsController, IsEffect, Performs, Resets, TransformsAudio},
    BipolarNormal, Normal, Sample, StereoSample,
};
use groove_proc_macros::{Control, Params, Uid};
use midly::MidiMessage;
use std::{collections::VecDeque, ops::Range};

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
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Timer {
    uid: usize,

    #[params]
    duration: MusicalTime,

    #[cfg_attr(feature = "serialization", serde(skip))]
    is_performing: bool,

    #[cfg_attr(feature = "serialization", serde(skip))]
    is_finished: bool,

    #[cfg_attr(feature = "serialization", serde(skip))]
    end_time: Option<MusicalTime>,
}
impl Timer {
    pub fn new_with(params: &TimerParams) -> Self {
        Self {
            uid: Default::default(),
            duration: MusicalTime::new_with(&params.duration),
            is_performing: false,
            is_finished: false,
            end_time: Default::default(),
        }
    }

    pub fn duration(&self) -> MusicalTime {
        self.duration
    }

    pub fn set_duration(&mut self, duration: MusicalTime) {
        self.duration = duration;
    }
}
impl IsController for Timer {}
impl HandlesMidi for Timer {}
impl Resets for Timer {
    fn reset(&mut self, _sample_rate: usize) {}
}
impl Controls for Timer {
    type Message = EntityMessage;

    fn update_time(&mut self, range: &Range<MusicalTime>) {
        if self.is_performing {
            if self.duration == MusicalTime::default() {
                // Zero-length timers fire immediately.
                self.is_finished = true;
            } else {
                if let Some(end_time) = self.end_time {
                    if range.end > end_time {
                        self.is_finished = true;
                    }
                } else {
                    // The first time we're called with an update_time() while
                    // performing, we take that as the start of the timer.
                    self.end_time = Some(range.start + self.duration);
                }
            }
        }
    }

    fn work(&mut self) -> Option<Vec<Self::Message>> {
        // All the state was computable during update_time(), so there's nothing to do here.
        None
    }

    fn is_finished(&self) -> bool {
        self.is_finished
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
        self.end_time = None;
    }

    fn is_performing(&self) -> bool {
        self.is_performing
    }
}

// TODO: needs tests!
/// [Trigger] issues a control signal after a specified amount of time.
#[derive(Debug, Control, Params, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Trigger {
    uid: usize,

    #[params]
    timer: Timer,

    #[params]
    value: f32,

    has_triggered: bool,
    is_performing: bool,
}
impl IsController for Trigger {}
impl Controls for Trigger {
    type Message = EntityMessage;

    fn update_time(&mut self, range: &Range<MusicalTime>) {
        self.timer.update_time(range)
    }

    fn work(&mut self) -> Option<Vec<Self::Message>> {
        if self.timer.is_finished() && self.is_performing && !self.has_triggered {
            self.has_triggered = true;
            Some(vec![EntityMessage::ControlF32(self.value())])
        } else {
            None
        }
    }

    fn is_finished(&self) -> bool {
        self.timer.is_finished()
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

    fn is_performing(&self) -> bool {
        self.is_performing
    }
}
impl Trigger {
    pub fn new_with(params: &TriggerParams) -> Self {
        Self {
            uid: Default::default(),
            timer: Timer::new_with(&params.timer),
            value: params.value(),
            has_triggered: false,
            is_performing: false,
        }
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn set_value(&mut self, value: f32) {
        self.value = value;
    }
}

/// Uses an input signal as a control source.
#[derive(Control, Debug, Params, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct SignalPassthroughController {
    uid: usize,
    #[cfg_attr(feature = "serialization", serde(skip))]
    signal: BipolarNormal,
    #[cfg_attr(feature = "serialization", serde(skip))]
    has_signal_changed: bool,
    #[cfg_attr(feature = "serialization", serde(skip))]
    is_performing: bool,
}
impl IsController for SignalPassthroughController {}
impl Resets for SignalPassthroughController {}
impl Controls for SignalPassthroughController {
    type Message = EntityMessage;

    fn update_time(&mut self, _range: &Range<MusicalTime>) {
        // We can ignore because we already have our own de-duplicating logic.
    }

    fn work(&mut self) -> Option<Vec<Self::Message>> {
        if !self.is_performing {
            return None;
        }
        if self.has_signal_changed {
            self.has_signal_changed = false;
            let normal: Normal = self.signal.into();
            Some(vec![EntityMessage::ControlF32(normal.value_as_f32())])
        } else {
            None
        }
    }

    fn is_finished(&self) -> bool {
        true
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

    fn is_performing(&self) -> bool {
        self.is_performing
    }
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

enum TestControllerAction {
    Nothing,
    NoteOn,
    NoteOff,
}

// #[cfg(feature = "serialization")]
// use serde::{Deserialize, Serialize};

/// An [IsController](groove_core::traits::IsController) that emits a MIDI
/// note-on event on each beat, and a note-off event on each half-beat.
#[derive(Debug, Control, Params, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct ToyController {
    uid: usize,

    #[cfg_attr(feature = "serialization", serde(skip))]
    midi_channel_out: MidiChannel,

    #[cfg_attr(feature = "serialization", serde(skip))]
    is_enabled: bool,
    #[cfg_attr(feature = "serialization", serde(skip))]
    is_playing: bool,
    #[cfg_attr(feature = "serialization", serde(skip))]
    is_performing: bool,

    #[cfg_attr(feature = "serialization", serde(skip))]
    pub checkpoint_values: VecDeque<f32>,
    #[cfg_attr(feature = "serialization", serde(skip))]
    pub checkpoint: f32,
    #[cfg_attr(feature = "serialization", serde(skip))]
    pub checkpoint_delta: f32,
    #[cfg_attr(feature = "serialization", serde(skip))]
    pub time_unit: ClockTimeUnit,

    #[cfg_attr(feature = "serialization", serde(skip))]
    time_range: Range<MusicalTime>,

    #[cfg_attr(feature = "serialization", serde(skip))]
    last_time_handled: MusicalTime,
}
impl IsController for ToyController {}
impl Controls for ToyController {
    type Message = EntityMessage;

    fn update_time(&mut self, range: &Range<MusicalTime>) {
        self.time_range = range.clone();
    }

    fn work(&mut self) -> Option<Vec<Self::Message>> {
        let mut v = Vec::default();
        match self.what_to_do() {
            TestControllerAction::Nothing => {}
            TestControllerAction::NoteOn => {
                // This is elegant, I hope. If the arpeggiator is
                // disabled during play, and we were playing a note,
                // then we still send the off note,
                if self.is_enabled && self.is_performing {
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
        if v.is_empty() {
            None
        } else {
            Some(v)
        }
    }

    fn is_finished(&self) -> bool {
        true
    }
}
impl Resets for ToyController {
    fn reset(&mut self, _sample_rate: usize) {}
}
impl HandlesMidi for ToyController {
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
impl Performs for ToyController {
    fn play(&mut self) {
        self.is_performing = true;
    }

    fn stop(&mut self) {
        self.is_performing = false;
    }

    fn skip_to_start(&mut self) {}

    fn is_performing(&self) -> bool {
        self.is_performing
    }
}
impl ToyController {
    pub fn new_with(params: ToyControllerParams, midi_channel_out: MidiChannel) -> Self {
        Self::new_with_test_values(
            params,
            midi_channel_out,
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        )
    }

    pub fn new_with_test_values(
        params: ToyControllerParams,
        midi_channel_out: MidiChannel,
        values: &[f32],
        checkpoint: f32,
        checkpoint_delta: f32,
        time_unit: ClockTimeUnit,
    ) -> Self {
        Self {
            uid: Default::default(),
            midi_channel_out,
            is_enabled: Default::default(),
            is_playing: Default::default(),
            is_performing: false,
            checkpoint_values: VecDeque::from(Vec::from(values)),
            checkpoint,
            checkpoint_delta,
            time_unit,
            time_range: MusicalTime::end_of_time_range(),
            last_time_handled: MusicalTime::end_of_time(),
        }
    }

    fn what_to_do(&mut self) -> TestControllerAction {
        if !self.time_range.contains(&self.last_time_handled) {
            self.last_time_handled = self.time_range.start;
            if self.time_range.start.subparts() == 0 {
                if self.time_range.start.parts() == 0 {
                    return TestControllerAction::NoteOn;
                }
                if self.time_range.start.parts() == 8 {
                    return TestControllerAction::NoteOn;
                }
            }
        }
        TestControllerAction::Nothing
    }

    #[cfg(feature = "iced-framework")]
    pub fn update(&mut self, message: ToyControllerMessage) {
        match message {
            ToyControllerMessage::ToyController(_) => panic!(),
            _ => self.derived_update(message),
        }
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

#[cfg(test)]
mod tests {
    use std::ops::Range;

    use crate::{
        controllers::{TimerParams, Trigger, TriggerParams},
        tests::DEFAULT_SAMPLE_RATE,
    };
    use groove_core::{
        time::{MusicalTime, MusicalTimeParams},
        traits::{Controls, Performs, Resets},
    };

    #[test]
    fn instantiate_trigger() {
        let mut trigger = Trigger::new_with(&TriggerParams {
            timer: TimerParams {
                duration: {
                    MusicalTimeParams {
                        bars: 1,
                        beats: 0,
                        parts: 0,
                        subparts: 0,
                    }
                },
            },
            value: 0.5,
        });
        trigger.reset(DEFAULT_SAMPLE_RATE);
        trigger.play();

        trigger.update_time(&Range {
            start: MusicalTime::new(0, 0, 0, 0),
            end: MusicalTime::new(0, 0, 0, 1),
        });
        let m = trigger.work();
        assert!(m.is_none());
        assert!(!trigger.is_finished());

        trigger.update_time(&Range {
            start: MusicalTime::new(1, 0, 0, 0),
            end: MusicalTime::new(1, 0, 0, 1),
        });
        let m = trigger.work();
        assert!(m.is_some());
        assert!(trigger.is_finished());
    }
}
