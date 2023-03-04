// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    control::F32ControlValue,
    generators::{Oscillator, Waveform},
    midi::{note_to_frequency, HandlesMidi, MidiChannel, MidiMessage},
    time::ClockTimeUnit,
    traits::{Controllable, Generates, HasUid, IsInstrument, Resets, Ticks},
    Dca, Sample, StereoSample,
};
use groove_macros::{Control, Uid};
use std::{collections::VecDeque, fmt::Debug, marker::PhantomData, str::FromStr};
use strum_macros::{Display, EnumString, FromRepr};

/// A simple implementation of IsInstrument that's useful for testing and
/// debugging. Uses a default Oscillator to produce sound, and its "envelope" is
/// just a boolean that responds to MIDI NoteOn/NoteOff.
///
/// To act as a controller target, it has two parameters: Oscillator waveform
/// and frequency.
#[derive(Control, Debug, Uid)]
pub struct ToyInstrument {
    uid: usize,
    sample_rate: usize,
    sample: StereoSample,

    /// -1.0 is Sawtooth, 1.0 is Square, anything else is Sine.
    #[controllable]
    pub waveform: PhantomData<Waveform>, // interesting use of PhantomData

    #[controllable]
    pub fake_value: f32,

    oscillator: Oscillator,
    dca: Dca,
    pub is_playing: bool,
    pub received_count: usize,
    pub handled_count: usize,

    pub checkpoint_values: VecDeque<f32>,
    pub checkpoint: f32,
    pub checkpoint_delta: f32,
    pub time_unit: ClockTimeUnit,

    pub debug_messages: Vec<MidiMessage>,
}
impl IsInstrument for ToyInstrument {}
impl Generates<StereoSample> for ToyInstrument {
    fn value(&self) -> StereoSample {
        self.sample
    }

    #[allow(unused_variables)]
    fn batch_values(&mut self, values: &mut [StereoSample]) {
        todo!()
    }
}
impl Resets for ToyInstrument {
    fn reset(&mut self, sample_rate: usize) {
        self.oscillator.reset(sample_rate);
    }
}
impl Ticks for ToyInstrument {
    fn tick(&mut self, tick_count: usize) {
        self.oscillator.tick(tick_count);
        // If we've been asked to assert values at checkpoints, do so.

        // TODODODODO
        // if !self.checkpoint_values.is_empty() && clock.time_for(&self.time_unit) >= self.checkpoint
        // {
        //     const SAD_FLOAT_DIFF: f32 = 1.0e-2;
        //     assert_approx_eq!(self.fake_value, self.checkpoint_values[0], SAD_FLOAT_DIFF);
        //     self.checkpoint += self.checkpoint_delta;
        //     self.checkpoint_values.pop_front();
        // }
        self.sample = if self.is_playing {
            self.dca
                .transform_audio_to_stereo(Sample::from(self.oscillator.value()))
        } else {
            StereoSample::SILENCE
        };
    }
}
impl HandlesMidi for ToyInstrument {
    fn handle_midi_message(
        &mut self,
        message: &MidiMessage,
    ) -> Option<Vec<(MidiChannel, MidiMessage)>> {
        self.debug_messages.push(*message);
        self.received_count += 1;

        match message {
            MidiMessage::NoteOn { key, vel: _ } => {
                self.is_playing = true;
                self.oscillator
                    .set_frequency(note_to_frequency(key.as_int()));
            }
            MidiMessage::NoteOff { key: _, vel: _ } => {
                self.is_playing = false;
            }
            _ => {}
        }
        None
    }
}
// impl TestsValues for TestInstrument {
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
//         self.fake_value
//     }

//     fn pop_checkpoint_value(&mut self) -> Option<f32> {
//         self.checkpoint_values.pop_front()
//     }
// }
impl ToyInstrument {
    pub fn new_with(sample_rate: usize) -> Self {
        let mut r = Self {
            uid: Default::default(),
            waveform: Default::default(),
            sample_rate,
            sample: Default::default(),
            fake_value: Default::default(),
            oscillator: Oscillator::new_with(sample_rate),
            dca: Default::default(),
            is_playing: Default::default(),
            received_count: Default::default(),
            handled_count: Default::default(),
            checkpoint_values: Default::default(),
            checkpoint: Default::default(),
            checkpoint_delta: Default::default(),
            time_unit: Default::default(),
            debug_messages: Default::default(),
        };
        r.sample_rate = sample_rate;

        r
    }

    pub fn new_with_test_values(
        sample_rate: usize,
        values: &[f32],
        checkpoint: f32,
        checkpoint_delta: f32,
        time_unit: ClockTimeUnit,
    ) -> Self {
        let mut r = Self::new_with(sample_rate);
        r.checkpoint_values = VecDeque::from(Vec::from(values));
        r.checkpoint = checkpoint;
        r.checkpoint_delta = checkpoint_delta;
        r.time_unit = time_unit;
        r
    }

    // TODO: when we have a more specific control param type, we can do a real
    // into/from
    #[allow(dead_code)]
    fn waveform(&self) -> f32 {
        match self.oscillator.waveform() {
            Waveform::Sawtooth => -1.0,
            Waveform::Square => 1.0,
            _ => 0.0,
        }
    }

    pub fn set_control_waveform(&mut self, value: F32ControlValue) {
        self.oscillator.set_waveform(if value.0 == -1.0 {
            Waveform::Sawtooth
        } else if value.0 == 1.0 {
            Waveform::Square
        } else {
            Waveform::Sine
        });
    }

    pub fn set_fake_value(&mut self, fake_value: f32) {
        self.fake_value = fake_value;
    }

    pub fn fake_value(&self) -> f32 {
        self.fake_value
    }

    pub fn set_control_fake_value(&mut self, fake_value: F32ControlValue) {
        self.set_fake_value(fake_value.0);
    }

    pub fn dump_messages(&self) {
        dbg!(&self.debug_messages);
    }
}
