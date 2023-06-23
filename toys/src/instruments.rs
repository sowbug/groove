// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    generators::{Envelope, EnvelopeParams, Oscillator, OscillatorParams, Waveform},
    instruments::Synthesizer,
    midi::{note_to_frequency, HandlesMidi, MidiChannel, MidiMessage},
    time::{ClockTimeUnit, SampleRate},
    traits::{
        Configurable, Generates, GeneratesEnvelope, IsInstrument, IsStereoSampleVoice, IsVoice,
        PlaysNotes, Ticks,
    },
    voices::{VoiceCount, VoiceStore},
    BipolarNormal, Dca, DcaParams, Normal, ParameterType, Sample, SampleType, StereoSample,
};
use groove_proc_macros::{Control, Params, Uid};
use std::{collections::VecDeque, fmt::Debug};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

/// An [IsInstrument](groove_core::traits::IsInstrument) that uses a default
/// Oscillator to produce sound. Its "envelope" is just a boolean that responds
/// to MIDI NoteOn/NoteOff. [Controllable](groove_core::traits::Controllable) by
/// two parameters: Oscillator waveform and frequency.
#[derive(Debug, Control, Params, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct ToyInstrument {
    uid: groove_core::Uid,

    #[control]
    #[params]
    fake_value: Normal,

    #[cfg_attr(feature = "serialization", serde(skip))]
    sample: StereoSample,

    oscillator: Oscillator,

    #[control]
    #[params]
    dca: Dca,

    #[cfg_attr(feature = "serialization", serde(skip))]
    pub is_playing: bool,
    #[cfg_attr(feature = "serialization", serde(skip))]
    pub received_count: usize,
    #[cfg_attr(feature = "serialization", serde(skip))]
    pub handled_count: usize,

    #[cfg_attr(feature = "serialization", serde(skip))]
    pub checkpoint_values: VecDeque<f32>,
    #[cfg_attr(feature = "serialization", serde(skip))]
    pub checkpoint: f32,
    #[cfg_attr(feature = "serialization", serde(skip))]
    pub checkpoint_delta: f32,
    #[cfg_attr(feature = "serialization", serde(skip))]
    pub time_unit: ClockTimeUnit,

    #[cfg_attr(feature = "serialization", serde(skip))]
    pub debug_messages: Vec<MidiMessage>,
}
impl IsInstrument for ToyInstrument {}
impl Generates<StereoSample> for ToyInstrument {
    fn value(&self) -> StereoSample {
        self.sample
    }

    fn batch_values(&mut self, values: &mut [StereoSample]) {
        for value in values {
            self.tick(1);
            *value = self.value();
        }
    }
}
impl Configurable for ToyInstrument {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.oscillator.update_sample_rate(sample_rate);
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
        _channel: MidiChannel,
        message: &MidiMessage,
        _messages_fn: &mut dyn FnMut(MidiChannel, MidiMessage),
    ) {
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
    pub fn new_with(params: &ToyInstrumentParams) -> Self {
        Self {
            uid: Default::default(),
            sample: Default::default(),
            fake_value: params.fake_value(),
            oscillator: Oscillator::new_with(&OscillatorParams::default_with_waveform(
                Waveform::Sine,
            )),
            dca: Dca::new_with(&params.dca),
            is_playing: Default::default(),
            received_count: Default::default(),
            handled_count: Default::default(),
            checkpoint_values: Default::default(),
            checkpoint: Default::default(),
            checkpoint_delta: Default::default(),
            time_unit: Default::default(),
            debug_messages: Default::default(),
        }
    }

    pub fn new_with_test_values(
        values: &[f32],
        checkpoint: f32,
        checkpoint_delta: f32,
        time_unit: ClockTimeUnit,
    ) -> Self {
        let mut r = Self::new_with(&ToyInstrumentParams {
            fake_value: Normal::maximum(),
            dca: DcaParams {
                gain: Normal::maximum(),
                pan: BipolarNormal::default(),
            },
        });
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

    pub fn set_fake_value(&mut self, fake_value: Normal) {
        self.fake_value = fake_value;
    }

    pub fn fake_value(&self) -> Normal {
        self.fake_value
    }

    pub fn dump_messages(&self) {
        dbg!(&self.debug_messages);
    }

    #[cfg(feature = "iced-framework")]
    pub fn update(&mut self, message: ToyInstrumentMessage) {
        match message {
            ToyInstrumentMessage::ToyInstrument(s) => *self = Self::new_with(s),
            _ => self.derived_update(message),
        }
    }
}

/// Another [IsInstrument](groove_core::traits::IsInstrument) that was designed
/// for black-box debugging.
#[derive(Debug, Control, Params, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct DebugSynth {
    uid: groove_core::Uid,

    #[control]
    #[params]
    fake_value: Normal,

    #[cfg_attr(feature = "serialization", serde(skip))]
    sample_rate: SampleRate,
    #[cfg_attr(feature = "serialization", serde(skip))]
    sample: StereoSample,

    // #[controllable]
    // oscillator_modulation: BipolarNormal,
    oscillator: Box<Oscillator>,
    envelope: Box<Envelope>,
}
impl IsInstrument for DebugSynth {}
impl Generates<StereoSample> for DebugSynth {
    fn value(&self) -> StereoSample {
        self.sample
    }

    #[allow(unused_variables)]
    fn batch_values(&mut self, values: &mut [StereoSample]) {
        todo!()
    }
}
impl Configurable for DebugSynth {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.oscillator.update_sample_rate(sample_rate);
    }
}
impl Ticks for DebugSynth {
    fn tick(&mut self, tick_count: usize) {
        self.oscillator.tick(tick_count);
        self.envelope.tick(tick_count);
        self.sample =
            StereoSample::from(self.oscillator.value().value() * self.envelope.value().value());
    }
}
impl HandlesMidi for DebugSynth {
    fn handle_midi_message(
        &mut self,
        _channel: MidiChannel,
        message: &MidiMessage,
        _messages_fn: &mut dyn FnMut(MidiChannel, MidiMessage),
    ) {
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
    }
}
impl DebugSynth {
    pub fn new_with_components(oscillator: Box<Oscillator>, envelope: Box<Envelope>) -> Self {
        Self {
            uid: Default::default(),
            sample_rate: Default::default(),
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

    pub fn new() -> Self {
        Self::new_with_components(
            Box::new(Oscillator::new_with(
                &OscillatorParams::default_with_waveform(Waveform::Sine),
            )),
            Box::new(Envelope::new_with(&EnvelopeParams::safe_default())),
        )
    }

    #[cfg(feature = "iced-framework")]
    pub fn update(&mut self, message: DebugSynthMessage) {
        match message {
            DebugSynthMessage::DebugSynth(_) => *self = Self::new(),
            _ => self.derived_update(message),
        }
    }

    pub fn fake_value(&self) -> Normal {
        self.fake_value
    }

    pub fn set_fake_value(&mut self, fake_value: Normal) {
        self.fake_value = fake_value;
    }
}

#[derive(Debug, Control, Params, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct ToySynth {
    uid: groove_core::Uid,

    #[params]
    voice_count: VoiceCount,

    #[control]
    #[params]
    waveform: Waveform,

    #[control]
    #[params]
    envelope: Envelope,

    #[cfg_attr(feature = "serialization", serde(skip))]
    inner: Synthesizer<ToyVoice>,
}
impl IsInstrument for ToySynth {}
impl Generates<StereoSample> for ToySynth {
    fn value(&self) -> StereoSample {
        self.inner.value()
    }

    fn batch_values(&mut self, values: &mut [StereoSample]) {
        self.inner.batch_values(values)
    }
}
impl HandlesMidi for ToySynth {
    fn handle_midi_message(
        &mut self,
        channel: MidiChannel,
        message: &MidiMessage,
        messages_fn: &mut dyn FnMut(MidiChannel, MidiMessage),
    ) {
        self.inner
            .handle_midi_message(channel, message, messages_fn)
    }
}
impl Ticks for ToySynth {
    fn tick(&mut self, tick_count: usize) {
        self.inner.tick(tick_count)
    }
}
impl Configurable for ToySynth {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.inner.update_sample_rate(sample_rate)
    }
}
impl ToySynth {
    pub fn new_with(params: &ToySynthParams) -> Self {
        let voice_store = VoiceStore::<ToyVoice>::new_with_voice(params.voice_count(), || {
            ToyVoice::new_with(params.waveform(), &params.envelope)
        });
        Self {
            uid: Default::default(),
            voice_count: params.voice_count(),
            waveform: params.waveform(),
            envelope: Envelope::new_with(&params.envelope),
            inner: Synthesizer::<ToyVoice>::new_with(Box::new(voice_store)),
        }
    }

    #[cfg(feature = "iced-framework")]
    pub fn update(&mut self, message: ToySynthMessage) {
        match message {
            ToySynthMessage::ToySynth(_s) => todo!(),
            _ => self.derived_update(message),
        }
    }

    pub fn voice_count(&self) -> VoiceCount {
        self.voice_count
    }

    pub fn set_voice_count(&mut self, voice_count: VoiceCount) {
        self.voice_count = voice_count;
    }

    pub fn waveform(&self) -> Waveform {
        self.waveform
    }

    pub fn set_waveform(&mut self, waveform: Waveform) {
        self.waveform = waveform;
    }

    pub fn envelope(&self) -> &Envelope {
        &self.envelope
    }

    pub fn set_envelope(&mut self, envelope: Envelope) {
        self.envelope = envelope;
    }
}

#[derive(Debug, Default)]
struct ToyVoice {
    oscillator: Oscillator,
    envelope: Envelope,
    value: StereoSample,
}
impl IsStereoSampleVoice for ToyVoice {}
impl IsVoice<StereoSample> for ToyVoice {}
impl PlaysNotes for ToyVoice {
    fn is_playing(&self) -> bool {
        !self.envelope.is_idle()
    }

    fn note_on(&mut self, key: u8, _velocity: u8) {
        self.envelope.trigger_attack();
        self.oscillator.set_frequency(note_to_frequency(key));
    }

    fn aftertouch(&mut self, _velocity: u8) {
        todo!()
    }

    fn note_off(&mut self, _velocity: u8) {
        self.envelope.trigger_release()
    }
}
impl Generates<StereoSample> for ToyVoice {
    fn value(&self) -> StereoSample {
        self.value
    }

    #[allow(unused_variables)]
    fn batch_values(&mut self, values: &mut [StereoSample]) {
        todo!()
    }
}
impl Ticks for ToyVoice {
    fn tick(&mut self, tick_count: usize) {
        self.oscillator.tick(tick_count);
        self.envelope.tick(tick_count);
        self.value =
            StereoSample::from(self.oscillator.value().value() * self.envelope.value().value());
    }
}
impl Configurable for ToyVoice {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.oscillator.update_sample_rate(sample_rate);
        self.envelope.update_sample_rate(sample_rate);
    }
}
impl ToyVoice {
    fn new_with(waveform: Waveform, envelope: &EnvelopeParams) -> Self {
        Self {
            oscillator: Oscillator::new_with(&OscillatorParams::default_with_waveform(waveform)),
            envelope: Envelope::new_with(envelope),
            value: Default::default(),
        }
    }
}

/// Produces a constant audio signal. Used for ensuring that a known signal
/// value gets all the way through the pipeline.
#[derive(Debug, Default, Control, Params, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct ToyAudioSource {
    uid: groove_core::Uid,

    // This should be a Normal, but we use this audio source for testing edge
    // conditions. Thus we need to let it go out of range.
    #[control]
    #[params]
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
impl Configurable for ToyAudioSource {
    fn update_sample_rate(&mut self, _sample_rate: SampleRate) {}
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

    pub fn new_with(params: &ToyAudioSourceParams) -> Self {
        Self {
            level: params.level(),
            ..Default::default()
        }
    }

    #[cfg(feature = "iced-framework")]
    pub fn update(&mut self, message: ToyAudioSourceMessage) {
        match message {
            ToyAudioSourceMessage::ToyAudioSource(s) => *self = Self::new_with(s),
            _ => self.derived_update(message),
        }
    }

    pub fn level(&self) -> f64 {
        self.level
    }

    pub fn set_level(&mut self, level: ParameterType) {
        self.level = level;
    }
}

#[cfg(feature = "egui-framework")]
mod gui {
    use super::{DebugSynth, ToyAudioSource, ToyInstrument, ToySynth};
    use eframe::egui::Ui;
    use groove_core::traits::{gui::Shows, HasUid};

    impl Shows for ToyInstrument {
        fn show(&mut self, ui: &mut Ui) {
            ui.label(self.name());
        }
    }

    impl Shows for DebugSynth {
        fn show(&mut self, ui: &mut Ui) {
            ui.label(self.name());
        }
    }
    impl Shows for ToySynth {
        fn show(&mut self, ui: &mut Ui) {
            ui.label(self.name());
        }
    }

    impl Shows for ToyAudioSource {
        fn show(&mut self, ui: &mut Ui) {
            ui.label(self.name());
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::{instruments::ToyInstrumentParams, ToyInstrument};
    use groove_core::{
        traits::{Generates, Ticks},
        DcaParams, Normal,
    };

    // TODO: restore tests that test basic trait behavior, then figure out how
    // to run everyone implementing those traits through that behavior. For now,
    // this one just tests that a generic instrument doesn't panic when accessed
    // for non-consecutive time slices.
    #[test]
    fn sources_audio_random_access() {
        let mut instrument = ToyInstrument::new_with(&ToyInstrumentParams {
            fake_value: Normal::from(0.42),
            dca: DcaParams {
                gain: Default::default(),
                pan: Default::default(),
            },
        });
        let mut rng = oorandom::Rand32::new(0);

        for _ in 0..100 {
            instrument.tick(rng.rand_range(1..10) as usize);
            let _ = instrument.value();
        }
    }
}
