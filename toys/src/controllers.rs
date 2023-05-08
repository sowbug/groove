// Copyright (c) 2023 Mike Tsao. All rights reserved.

use core::fmt::Debug;
use groove_core::{
    midi::{new_note_off, new_note_on, HandlesMidi, MidiChannel, MidiMessage},
    time::{Clock, ClockParams, ClockTimeUnit, TimeSignature},
    traits::{IsController, MessageBounds, Performs, Resets, Ticks, TicksWithMessages},
    ParameterType,
};
use groove_proc_macros::{Control, Nano, Params, Uid};
use std::collections::VecDeque;
use std::str::FromStr;
use strum::EnumCount;
use strum_macros::{Display, EnumCount as EnumCountMacro, EnumString, FromRepr, IntoStaticStr};

enum TestControllerAction {
    Nothing,
    NoteOn,
    NoteOff,
}

pub trait MessageMaker: Send + Debug {
    type Message;
    fn midi(&self, channel: MidiChannel, message: MidiMessage) -> Self::Message;
}

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[cfg(toy_controller_disabled)]
mod toy_controller_disabled {
    /// An [IsController](groove_core::traits::IsController) that emits a MIDI
    /// note-on event on each beat, and a note-off event on each half-beat.
    #[derive(Debug, Control, Params, Uid)]
    pub struct ToyController<M: MessageBounds> {
        uid: usize,

        #[control]
        #[params]
        bpm: ParameterType,

        midi_channel_out: MidiChannel,

        clock: Clock,

        #[control]
        #[params]
        tempo: f32,

        is_enabled: bool,
        is_playing: bool,
        is_performing: bool,

        pub checkpoint_values: VecDeque<f32>,
        pub checkpoint: f32,
        pub checkpoint_delta: f32,
        pub time_unit: ClockTimeUnit,

        message_maker: Box<dyn MessageMaker<Message = M>>,
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
                        if self.is_enabled && self.is_performing {
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
            self.clock.reset(sample_rate);
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
    impl<M: MessageBounds> Performs for ToyController<M> {
        fn play(&mut self) {
            self.is_performing = true;
        }

        fn stop(&mut self) {
            self.is_performing = false;
        }

        fn skip_to_start(&mut self) {}
    }
    impl<M: MessageBounds> ToyController<M> {
        pub fn new_with(
            params: ToyControllerParams,
            midi_channel_out: MidiChannel,
            message_maker: Box<dyn MessageMaker<Message = M>>,
        ) -> Self {
            Self::new_with_test_values(
                params,
                midi_channel_out,
                message_maker,
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
            )
        }

        pub fn new_with_test_values(
            params: ToyControllerParams,
            midi_channel_out: MidiChannel,
            message_maker: Box<dyn MessageMaker<Message = M>>,
            values: &[f32],
            checkpoint: f32,
            checkpoint_delta: f32,
            time_unit: ClockTimeUnit,
        ) -> Self {
            Self {
                uid: Default::default(),
                bpm: params.bpm(),
                tempo: params.tempo(),
                midi_channel_out,
                clock: Clock::new_with(&ClockParams {
                    bpm: params.bpm(),
                    midi_ticks_per_second: 9999,
                    time_signature: TimeSignature { top: 4, bottom: 4 },
                }),
                is_enabled: Default::default(),
                is_playing: Default::default(),
                is_performing: false,
                checkpoint_values: VecDeque::from(Vec::from(values)),
                checkpoint,
                checkpoint_delta,
                time_unit,
                message_maker,
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

        pub fn update(&mut self, message: ToyControllerMessage) {
            match message {
                ToyControllerMessage::ToyController(_) => panic!(),
                _ => self.derived_update(message),
            }
        }

        pub fn bpm(&self) -> f64 {
            self.bpm
        }

        pub fn tempo(&self) -> f32 {
            self.tempo
        }

        pub fn set_bpm(&mut self, bpm: ParameterType) {
            self.bpm = bpm;
        }

        pub fn set_tempo(&mut self, tempo: f32) {
            self.tempo = tempo;
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
}
