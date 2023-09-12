// Copyright (c) 2023 Mike Tsao. All rights reserved.

pub use arpeggiator::{Arpeggiator, ArpeggiatorParams};
pub use calculator::{Calculator, CalculatorParams};
pub use control_trip::{ControlPath, ControlStep, ControlTrip, ControlTripParams};
pub use lfo::{LfoController, LfoControllerParams};
pub use patterns::{
    NewPattern, Note, Pattern, PatternManager, PatternManagerParams, PatternProgrammer,
};
pub use sequencers::{Sequencer, SequencerParams};

mod arpeggiator;
mod calculator;
mod control_trip;
mod lfo;
mod patterns;
mod sequencers;

use groove_core::{
    control::ControlValue,
    midi::{new_note_off, new_note_on, HandlesMidi, MidiChannel, MidiMessagesFn},
    time::{ClockTimeUnit, MusicalTime, MusicalTimeParams, SampleRate},
    traits::{Configurable, ControlEventsFn, Controls, EntityEvent, Serializable, TransformsAudio},
    BipolarNormal, Normal, Sample, StereoSample,
};
use groove_proc_macros::{Control, IsController, IsControllerEffect, Params, Uid};
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
#[derive(Debug, Control, IsController, Params, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Timer {
    uid: groove_core::Uid,

    #[params]
    duration: MusicalTime,

    #[cfg_attr(feature = "serialization", serde(skip))]
    is_performing: bool,

    #[cfg_attr(feature = "serialization", serde(skip))]
    is_finished: bool,

    #[cfg_attr(feature = "serialization", serde(skip))]
    end_time: Option<MusicalTime>,
}
impl Serializable for Timer {}
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
impl HandlesMidi for Timer {}
impl Configurable for Timer {
    fn update_sample_rate(&mut self, _sample_rate: SampleRate) {}
}
impl Controls for Timer {
    fn update_time(&mut self, range: &Range<MusicalTime>) {
        if self.is_performing {
            if self.duration == MusicalTime::default() {
                // Zero-length timers fire immediately.
                self.is_finished = true;
            } else {
                if let Some(end_time) = self.end_time {
                    if range.contains(&end_time) {
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

    fn work(&mut self, _messages_fn: &mut ControlEventsFn) {
        // All the state was computable during update_time(), so there's nothing to do here.
    }

    fn is_finished(&self) -> bool {
        self.is_finished
    }

    fn play(&mut self) {
        self.is_performing = true;
    }

    fn stop(&mut self) {
        self.is_performing = false;
    }

    fn skip_to_start(&mut self) {
        // TODO: think how important it is for LFO oscillator to start at zero
    }

    fn set_loop(&mut self, _range: &Range<groove_core::time::PerfectTimeUnit>) {
        // TODO
    }

    fn clear_loop(&mut self) {
        // TODO
    }

    fn set_loop_enabled(&mut self, _is_enabled: bool) {
        // TODO
    }

    fn is_performing(&self) -> bool {
        self.is_performing
    }
}

// TODO: needs tests!
/// [Trigger] issues a control signal after a specified amount of time.
#[derive(Debug, Control, IsController, Params, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Trigger {
    uid: groove_core::Uid,

    #[params]
    timer: Timer,

    #[params]
    value: Normal,

    has_triggered: bool,
    is_performing: bool,
}
impl Serializable for Trigger {}
impl Controls for Trigger {
    fn update_time(&mut self, range: &Range<MusicalTime>) {
        self.timer.update_time(range)
    }

    fn work(&mut self, control_events_fn: &mut ControlEventsFn) {
        if self.timer.is_finished() && self.is_performing && !self.has_triggered {
            self.has_triggered = true;
            control_events_fn(self.uid, EntityEvent::Control(self.value().into()));
        }
    }

    fn is_finished(&self) -> bool {
        self.timer.is_finished()
    }

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
impl Configurable for Trigger {
    fn sample_rate(&self) -> SampleRate {
        self.timer.sample_rate()
    }
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.timer.update_sample_rate(sample_rate)
    }
}
impl HandlesMidi for Trigger {}
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

    pub fn value(&self) -> Normal {
        self.value
    }

    pub fn set_value(&mut self, value: Normal) {
        self.value = value;
    }
}

#[derive(Debug, Default, Clone, Copy)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub enum SignalPassthroughType {
    #[default]
    /// Maps -1.0..=1.0 to 0.0..=1.0. Min amplitude becomes 0.0, silence becomes
    /// 0.5, and max amplitude becomes 1.0.
    Compressed,

    /// Based on the absolute value of the sample. Silence is 0.0, and max
    /// amplitude of either polarity is 1.0.
    Amplitude,

    /// Based on the absolute value of the sample. Silence is 1.0, and max
    /// amplitude of either polarity is 0.0.
    AmplitudeInverted,
}

/// Uses an input signal as a control source. Transformation depends on
/// configuration. Uses the standard Sample::from(StereoSample) methodology of
/// averaging the two channels to create a single signal.
#[derive(Control, Debug, Default, IsControllerEffect, Params, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct SignalPassthroughController {
    uid: groove_core::Uid,
    passthrough_type: SignalPassthroughType,

    #[cfg_attr(feature = "serialization", serde(skip))]
    control_value: ControlValue,

    // We don't issue consecutive identical events, so we need to remember
    // whether we've sent the current value.
    #[cfg_attr(feature = "serialization", serde(skip))]
    has_value_been_issued: bool,

    #[cfg_attr(feature = "serialization", serde(skip))]
    is_performing: bool,
}
impl Serializable for SignalPassthroughController {}
impl Configurable for SignalPassthroughController {}
impl Controls for SignalPassthroughController {
    fn update_time(&mut self, _range: &Range<MusicalTime>) {
        // We can ignore because we already have our own de-duplicating logic.
    }

    fn work(&mut self, control_events_fn: &mut ControlEventsFn) {
        if !self.is_performing {
            return;
        }
        if !self.has_value_been_issued {
            self.has_value_been_issued = true;
            control_events_fn(self.uid, EntityEvent::Control(self.control_value))
        }
    }

    fn is_finished(&self) -> bool {
        true
    }

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
impl HandlesMidi for SignalPassthroughController {}
impl TransformsAudio for SignalPassthroughController {
    fn transform_audio(&mut self, input_sample: StereoSample) -> StereoSample {
        let sample: Sample = input_sample.into();
        let control_value = match self.passthrough_type {
            SignalPassthroughType::Compressed => {
                let as_bipolar_normal: BipolarNormal = sample.into();
                as_bipolar_normal.into()
            }
            SignalPassthroughType::Amplitude => ControlValue(sample.0.abs()),
            SignalPassthroughType::AmplitudeInverted => ControlValue(1.0 - sample.0.abs()),
        };
        if self.control_value != control_value {
            self.has_value_been_issued = false;
            self.control_value = control_value;
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
        Default::default()
    }

    pub fn new_amplitude_passthrough_type() -> Self {
        Self {
            passthrough_type: SignalPassthroughType::Amplitude,
            ..Default::default()
        }
    }

    pub fn new_amplitude_inverted_passthrough_type() -> Self {
        Self {
            passthrough_type: SignalPassthroughType::AmplitudeInverted,
            ..Default::default()
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
#[derive(Debug, Control, IsController, Params, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct ToyController {
    uid: groove_core::Uid,

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
impl Serializable for ToyController {}
impl Controls for ToyController {
    fn update_time(&mut self, range: &Range<MusicalTime>) {
        self.time_range = range.clone();
    }

    fn work(&mut self, control_events_fn: &mut ControlEventsFn) {
        match self.what_to_do() {
            TestControllerAction::Nothing => {}
            TestControllerAction::NoteOn => {
                // This is elegant, I hope. If the arpeggiator is
                // disabled during play, and we were playing a note,
                // then we still send the off note,
                if self.is_enabled && self.is_performing {
                    self.is_playing = true;
                    control_events_fn(
                        self.uid,
                        EntityEvent::Midi(self.midi_channel_out, new_note_on(60, 127)),
                    );
                }
            }
            TestControllerAction::NoteOff => {
                if self.is_playing {
                    control_events_fn(
                        self.uid,
                        EntityEvent::Midi(self.midi_channel_out, new_note_off(60, 0)),
                    );
                }
            }
        }
    }

    fn is_finished(&self) -> bool {
        true
    }

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
impl Configurable for ToyController {
    fn update_sample_rate(&mut self, _sample_rate: SampleRate) {}
}
impl HandlesMidi for ToyController {
    fn handle_midi_message(
        &mut self,
        _channel: MidiChannel,
        message: MidiMessage,
        _: &mut MidiMessagesFn,
    ) {
        #[allow(unused_variables)]
        match message {
            MidiMessage::NoteOff { key, vel } => self.is_enabled = false,
            MidiMessage::NoteOn { key, vel } => self.is_enabled = true,
            _ => todo!(),
        }
    }
}
impl ToyController {
    pub fn new_with(params: &ToyControllerParams, midi_channel_out: MidiChannel) -> Self {
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
        _params: &ToyControllerParams,
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
            time_range: MusicalTime::empty_range(),
            last_time_handled: MusicalTime::TIME_MAX,
        }
    }

    fn what_to_do(&mut self) -> TestControllerAction {
        if !self.time_range.contains(&self.last_time_handled) {
            self.last_time_handled = self.time_range.start;
            if self.time_range.start.units() == 0 {
                if self.time_range.start.parts() == 0 {
                    return TestControllerAction::NoteOn;
                }
                if self.time_range.start.parts() == 8 {
                    return TestControllerAction::NoteOff;
                }
            }
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

#[cfg(feature = "egui-framework")]
mod gui {
    use super::{SignalPassthroughController, Timer, ToyController, Trigger};
    use eframe::egui::Ui;
    use groove_core::traits::{gui::Displays, HasUid};

    impl Displays for Timer {
        fn ui(&mut self, ui: &mut Ui) -> eframe::egui::Response {
            ui.label(self.name())
        }
    }

    impl Displays for Trigger {
        fn ui(&mut self, ui: &mut Ui) -> eframe::egui::Response {
            ui.label(self.name())
        }
    }

    impl Displays for SignalPassthroughController {
        fn ui(&mut self, ui: &mut Ui) -> eframe::egui::Response {
            ui.label(self.name())
        }
    }

    impl Displays for ToyController {
        fn ui(&mut self, ui: &mut Ui) -> eframe::egui::Response {
            ui.label(self.name())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::controllers::{TimerParams, Trigger, TriggerParams};
    use groove_core::{
        time::{MusicalTime, MusicalTimeParams, SampleRate, TimeSignature},
        traits::{Configurable, Controls},
        Normal,
    };
    use std::ops::Range;

    #[test]
    fn instantiate_trigger() {
        let ts = TimeSignature::default();
        let mut trigger = Trigger::new_with(&TriggerParams {
            timer: TimerParams {
                duration: {
                    MusicalTimeParams {
                        units: MusicalTime::bars_to_units(&ts, 1),
                    }
                },
            },
            value: Normal::from(0.5),
        });
        trigger.update_sample_rate(SampleRate::DEFAULT);
        trigger.play();

        trigger.update_time(&Range {
            start: MusicalTime::default(),
            end: MusicalTime::new_with_parts(1),
        });
        let mut count = 0;
        trigger.work(&mut |_, _| {
            count += 1;
        });
        assert_eq!(count, 0);
        assert!(!trigger.is_finished());

        trigger.update_time(&Range {
            start: MusicalTime::new_with_bars(&ts, 1),
            end: MusicalTime::new(&ts, 1, 0, 0, 1),
        });
        let mut count = 0;
        trigger.work(&mut |_, _| {
            count += 1;
        });
        assert!(count != 0);
        assert!(trigger.is_finished());
    }
}
