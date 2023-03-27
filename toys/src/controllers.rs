// Copyright (c) 2023 Mike Tsao. All rights reserved.

use core::fmt::Debug;
use groove_core::{
    midi::{new_note_off, new_note_on, HandlesMidi, MidiChannel, MidiMessage},
    time::{Clock, ClockTimeUnit},
    traits::{IsController, MessageBounds, Resets, Ticks, TicksWithMessages},
    ParameterType,
};
use groove_macros::{Control, Uid};
use std::str::FromStr;
use std::{collections::VecDeque, marker::PhantomData};
use strum::EnumCount;
use strum_macros::{
    Display, EnumCount as EnumCountMacro, EnumIter, EnumString, FromRepr, IntoStaticStr,
};

enum TestControllerAction {
    Nothing,
    NoteOn,
    NoteOff,
}

pub trait MessageMaker: Send + Debug {
    type Message;
    fn midi(&self, channel: MidiChannel, message: MidiMessage) -> Self::Message;
}

/// An [IsController](groove_core::traits::IsController) that emits a MIDI
/// note-on event on each beat, and a note-off event on each half-beat.
#[derive(Control, Debug, Uid)]
pub struct ToyController<M: MessageBounds> {
    uid: usize,
    midi_channel_out: MidiChannel,

    sample_rate: usize,
    midi_ticks_per_second: usize,
    bpm: ParameterType,
    clock: Clock,

    #[controllable]
    pub tempo: f32,
    is_enabled: bool,
    is_playing: bool,

    pub checkpoint_values: VecDeque<f32>,
    pub checkpoint: f32,
    pub checkpoint_delta: f32,
    pub time_unit: ClockTimeUnit,

    message_maker: Box<dyn MessageMaker<Message = M>>,

    _phantom: PhantomData<M>,
}
impl<M: MessageBounds> IsController for ToyController<M> {}
impl<M: MessageBounds> TicksWithMessages for ToyController<M> {
    type Message = M;

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
                        v.push(
                            self.message_maker
                                .midi(self.midi_channel_out, new_note_on(60, 127)),
                        );
                    }
                }
                TestControllerAction::NoteOff => {
                    if self.is_playing {
                        v.push(
                            self.message_maker
                                .midi(self.midi_channel_out, new_note_off(60, 0)),
                        );
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
impl<M: MessageBounds> Resets for ToyController<M> {
    fn reset(&mut self, sample_rate: usize) {
        self.sample_rate = sample_rate;
        self.clock = Clock::new_with(sample_rate, self.bpm, self.midi_ticks_per_second);
    }
}
impl<M: MessageBounds> HandlesMidi for ToyController<M> {
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
impl<M: MessageBounds> ToyController<M> {
    pub fn new_with(
        sample_rate: usize,
        bpm: ParameterType,
        midi_channel_out: MidiChannel,
        message_maker: Box<dyn MessageMaker<Message = M>>,
    ) -> Self {
        Self::new_with_test_values(
            sample_rate,
            bpm,
            midi_channel_out,
            message_maker,
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
        message_maker: Box<dyn MessageMaker<Message = M>>,
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
            message_maker,
            _phantom: Default::default(),
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

    pub fn set_control_tempo(&mut self, tempo: groove_core::control::F32ControlValue) {
        self.tempo = tempo.0;
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
