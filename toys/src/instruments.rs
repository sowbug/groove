// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    generators::{Envelope, EnvelopeParams, Oscillator, WaveformParams},
    midi::{note_to_frequency, HandlesMidi, MidiChannel, MidiMessage},
    time::ClockTimeUnit,
    traits::{Generates, GeneratesEnvelope, IsInstrument, Resets, Ticks},
    Dca, DcaParams, Normal, ParameterType, Sample, SampleType, StereoSample,
};
use groove_proc_macros::{Nano, Uid};
use std::{collections::VecDeque, fmt::Debug, str::FromStr};
use strum::EnumCount;
use strum_macros::{Display, EnumCount as EnumCountMacro, EnumString, FromRepr, IntoStaticStr};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

/// An [IsInstrument](groove_core::traits::IsInstrument) that uses a default
/// Oscillator to produce sound. Its "envelope" is just a boolean that responds
/// to MIDI NoteOn/NoteOff. [Controllable](groove_core::traits::Controllable) by
/// two parameters: Oscillator waveform and frequency.
#[derive(Debug, Nano, Uid)]
pub struct ToyInstrument {
    uid: usize,

    #[nano]
    fake_value: Normal,

    sample_rate: usize,
    sample: StereoSample,

    /// -1.0 is Sawtooth, 1.0 is Square, anything else is Sine.
    // #[controllable]
    // pub waveform: PhantomData<WaveformParams>, // interesting use of PhantomData
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
    pub fn new_with(sample_rate: usize, params: ToyInstrumentNano) -> Self {
        let mut r = Self {
            uid: Default::default(),
            sample_rate,
            sample: Default::default(),
            fake_value: params.fake_value(),
            oscillator: Oscillator::new_with(sample_rate),
            dca: Dca::new_with_params(DcaParams::default()),
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
        let mut r = Self::new_with(
            sample_rate,
            ToyInstrumentNano {
                fake_value: Normal::maximum(),
            },
        );
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
            WaveformParams::Sawtooth => -1.0,
            WaveformParams::Square => 1.0,
            _ => 0.0,
        }
    }

    pub fn set_fake_value(&mut self, fake_value: Normal) {
        self.fake_value = fake_value;
    }

    pub fn fake_value(&self) -> Normal {
        self.fake_value
    }

    pub fn dump_messages(&self) {
        dbg!(&self.debug_messages);
    }

    pub fn update(&mut self, message: ToyInstrumentMessage) {
        match message {
            ToyInstrumentMessage::ToyInstrument(s) => *self = Self::new_with(self.sample_rate, s),
            ToyInstrumentMessage::FakeValue(fake_value) => self.set_fake_value(fake_value),
        }
    }
}

/// Another [IsInstrument](groove_core::traits::IsInstrument) that was designed
/// for black-box debugging.
#[derive(Debug, Nano, Uid)]
pub struct ToySynth {
    uid: usize,

    #[nano]
    fake_value: Normal,

    sample_rate: usize,
    sample: StereoSample,

    // #[controllable]
    // oscillator_modulation: BipolarNormal,
    oscillator: Box<Oscillator>,
    envelope: Box<dyn GeneratesEnvelope>,
}
impl IsInstrument for ToySynth {}
impl Generates<StereoSample> for ToySynth {
    fn value(&self) -> StereoSample {
        self.sample
    }

    #[allow(unused_variables)]
    fn batch_values(&mut self, values: &mut [StereoSample]) {
        todo!()
    }
}
impl Resets for ToySynth {
    fn reset(&mut self, sample_rate: usize) {
        self.sample_rate = sample_rate;
        self.oscillator.reset(sample_rate);
    }
}
impl Ticks for ToySynth {
    fn tick(&mut self, tick_count: usize) {
        self.oscillator.tick(tick_count);
        self.envelope.tick(tick_count);
        self.sample =
            StereoSample::from(self.oscillator.value().value() * self.envelope.value().value());
    }
}
impl HandlesMidi for ToySynth {
    fn handle_midi_message(
        &mut self,
        message: &MidiMessage,
    ) -> Option<Vec<(MidiChannel, MidiMessage)>> {
        #[allow(unused_variables)]
        match message {
            MidiMessage::NoteOff { key, vel } => {
                self.envelope.trigger_release();
            }
            MidiMessage::NoteOn { key, vel } => {
                self.envelope.trigger_attack();
                self.oscillator
                    .set_frequency(note_to_frequency((*key).as_int()));
            }
            _ => todo!(),
        }
        None
    }
}
impl ToySynth {
    pub fn new_with_components(
        sample_rate: usize,
        oscillator: Box<Oscillator>,
        envelope: Box<dyn GeneratesEnvelope>,
    ) -> Self {
        Self {
            uid: Default::default(),
            sample_rate,
            fake_value: Normal::from(0.32342),
            sample: Default::default(),
            // oscillator_modulation: Default::default(),
            oscillator,
            envelope,
        }
    }

    // pub fn oscillator_modulation(&self) -> BipolarNormal {
    //     self.oscillator.frequency_modulation()
    // }

    // pub fn set_oscillator_modulation(&mut self, oscillator_modulation: BipolarNormal) {
    //     self.oscillator_modulation = oscillator_modulation;
    //     self.oscillator
    //         .set_frequency_modulation(oscillator_modulation);
    // }

    pub fn new_with(sample_rate: usize) -> Self {
        Self::new_with_components(
            sample_rate,
            Box::new(Oscillator::new_with(sample_rate)),
            Box::new(Envelope::new_with(
                sample_rate,
                EnvelopeParams::new_with(0.0, 0.0, Normal::maximum(), 0.0),
            )),
        )
    }

    pub fn update(&mut self, message: ToySynthMessage) {
        match message {
            ToySynthMessage::ToySynth(_) => *self = Self::new_with(self.sample_rate),
            ToySynthMessage::FakeValue(fake_value) => self.set_fake_value(fake_value),
        }
    }

    pub fn fake_value(&self) -> Normal {
        self.fake_value
    }

    pub fn set_fake_value(&mut self, fake_value: Normal) {
        self.fake_value = fake_value;
    }
}

/// Produces a constant audio signal. Used for ensuring that a known signal
/// value gets all the way through the pipeline.
#[derive(Debug, Default, Nano, Uid)]
pub struct ToyAudioSource {
    uid: usize,

    // This should be a Normal, but we use this audio source for testing edge
    // conditions. Thus we need to let it go out of range.
    #[nano]
    level: ParameterType,
}
impl IsInstrument for ToyAudioSource {}
impl Generates<StereoSample> for ToyAudioSource {
    fn value(&self) -> StereoSample {
        StereoSample::from(self.level)
    }

    #[allow(unused_variables)]
    fn batch_values(&mut self, values: &mut [StereoSample]) {
        todo!()
    }
}
impl Resets for ToyAudioSource {
    fn reset(&mut self, _sample_rate: usize) {}
}
impl Ticks for ToyAudioSource {
    fn tick(&mut self, _tick_count: usize) {}
}
impl HandlesMidi for ToyAudioSource {}
#[allow(dead_code)]
impl ToyAudioSource {
    pub const TOO_LOUD: SampleType = 1.1;
    pub const LOUD: SampleType = 1.0;
    pub const SILENT: SampleType = 0.0;
    pub const QUIET: SampleType = -1.0;
    pub const TOO_QUIET: SampleType = -1.1;

    pub fn new_with(params: ToyAudioSourceNano) -> Self {
        Self {
            level: params.level(),
            ..Default::default()
        }
    }

    pub fn update(&mut self, message: ToyAudioSourceMessage) {
        match message {
            ToyAudioSourceMessage::ToyAudioSource(s) => *self = Self::new_with(s),
            ToyAudioSourceMessage::Level(level) => self.set_level(level),
        }
    }

    pub fn level(&self) -> f64 {
        self.level
    }

    pub fn set_level(&mut self, level: ParameterType) {
        self.level = level;
    }
}

#[cfg(test)]
pub mod tests {
    use crate::{instruments::ToyInstrumentNano, ToyInstrument};
    use groove_core::{
        traits::{Generates, Ticks},
        Normal,
    };
    use rand::random;

    const DEFAULT_SAMPLE_RATE: usize = 44100;

    // TODO: restore tests that test basic trait behavior, then figure out how
    // to run everyone implementing those traits through that behavior. For now,
    // this one just tests that a generic instrument doesn't panic when accessed
    // for non-consecutive time slices.
    #[test]
    fn test_sources_audio_random_access() {
        let mut instrument = ToyInstrument::new_with(
            DEFAULT_SAMPLE_RATE,
            ToyInstrumentNano {
                fake_value: Normal::from(0.42),
            },
        );
        for _ in 0..100 {
            instrument.tick(random::<usize>() % 10);
            let _ = instrument.value();
        }
    }
}
