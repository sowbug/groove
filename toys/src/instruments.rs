// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    generators::{Envelope, EnvelopeParams, Oscillator, OscillatorParams, Waveform},
    instruments::Synthesizer,
    midi::{note_to_frequency, HandlesMidi, MidiChannel, MidiMessage, MidiMessagesFn},
    time::{ClockTimeUnit, SampleRate},
    traits::{
        Configurable, Generates, GeneratesEnvelope, IsStereoSampleVoice, IsVoice, PlaysNotes,
        Serializable, Ticks,
    },
    voices::{VoiceCount, VoiceStore},
    BipolarNormal, Dca, DcaParams, Normal, ParameterType, Sample, SampleType, StereoSample,
};
use groove_proc_macros::{Control, IsInstrument, Params, Uid};
use std::{collections::VecDeque, fmt::Debug};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

/// An [IsInstrument](groove_core::traits::IsInstrument) that uses a default
/// Oscillator to produce sound. Its "envelope" is just a boolean that responds
/// to MIDI NoteOn/NoteOff. [Controllable](groove_core::traits::Controllable) by
/// two parameters: Oscillator waveform and frequency.
#[derive(Debug, Control, IsInstrument, Params, Uid)]
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
impl Generates<StereoSample> for ToyInstrument {
    fn value(&self) -> StereoSample {
        self.sample
    }

    fn generate_batch_values(&mut self, values: &mut [StereoSample]) {
        for value in values {
            self.tick(1);
            *value = self.value();
        }
    }
}
impl Configurable for ToyInstrument {
    fn sample_rate(&self) -> SampleRate {
        self.oscillator.sample_rate()
    }

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
        message: MidiMessage,
        _: &mut MidiMessagesFn,
    ) {
        self.debug_messages.push(message);
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
impl Serializable for ToyInstrument {}

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
#[derive(Debug, Control, IsInstrument, Params, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct DebugSynth {
    uid: groove_core::Uid,

    #[control]
    #[params]
    fake_value: Normal,

    #[cfg_attr(feature = "serialization", serde(skip))]
    sample: StereoSample,

    // #[controllable]
    // oscillator_modulation: BipolarNormal,
    oscillator: Box<Oscillator>,
    envelope: Box<Envelope>,
}
impl Serializable for DebugSynth {}
impl Generates<StereoSample> for DebugSynth {
    fn value(&self) -> StereoSample {
        self.sample
    }

    #[allow(unused_variables)]
    fn generate_batch_values(&mut self, values: &mut [StereoSample]) {
        todo!()
    }
}
impl Configurable for DebugSynth {
    fn sample_rate(&self) -> SampleRate {
        self.oscillator.sample_rate()
    }

    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
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
        message: MidiMessage,
        _: &mut MidiMessagesFn,
    ) {
        #[allow(unused_variables)]
        match message {
            MidiMessage::NoteOff { key, vel } => {
                self.envelope.trigger_release();
            }
            MidiMessage::NoteOn { key, vel } => {
                self.envelope.trigger_attack();
                self.oscillator
                    .set_frequency(note_to_frequency(key.as_int()));
            }
            _ => todo!(),
        }
    }
}
impl DebugSynth {
    pub fn new_with_components(oscillator: Box<Oscillator>, envelope: Box<Envelope>) -> Self {
        Self {
            uid: Default::default(),
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

#[derive(Debug, Default, Control, IsInstrument, Params, Uid)]
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

    // TODO: this skip is a can of worms. I don't know whether we want to
    // serialize everything, or manually reconstitute everything. Maybe the
    // right answer is to expect that every struct gets serialized, but everyone
    // should be #[serde(skip)]ing at the leaf-field level.
    #[cfg_attr(feature = "serialization", serde(skip))]
    inner: Synthesizer<ToyVoice>,

    #[cfg_attr(feature = "serialization", serde(skip))]
    max_signal: Normal,
}
impl Serializable for ToySynth {}
impl Generates<StereoSample> for ToySynth {
    fn value(&self) -> StereoSample {
        self.inner.value()
    }

    fn generate_batch_values(&mut self, values: &mut [StereoSample]) {
        self.inner.generate_batch_values(values);
        self.update_max();
    }
}
impl HandlesMidi for ToySynth {
    fn handle_midi_message(
        &mut self,
        channel: MidiChannel,
        message: MidiMessage,
        midi_messages_fn: &mut MidiMessagesFn,
    ) {
        self.inner
            .handle_midi_message(channel, message, midi_messages_fn)
    }
}
impl Ticks for ToySynth {
    fn tick(&mut self, tick_count: usize) {
        self.inner.tick(tick_count);

        self.update_max();
    }
}
impl Configurable for ToySynth {
    fn sample_rate(&self) -> SampleRate {
        self.inner.sample_rate()
    }
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
            max_signal: Normal::minimum(),
        }
    }

    fn update_max(&mut self) {
        let value = Normal::from(Sample::from(self.value()).0);
        if value > self.max_signal {
            self.max_signal = value;
        }
    }

    pub fn degrade_max(&mut self, factor: f64) {
        self.max_signal *= factor;
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
    fn generate_batch_values(&mut self, values: &mut [StereoSample]) {
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
    fn sample_rate(&self) -> SampleRate {
        self.oscillator.sample_rate()
    }

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
#[derive(Debug, Default, Control, IsInstrument, Params, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct ToyAudioSource {
    uid: groove_core::Uid,

    // This should be a Normal, but we use this audio source for testing edge
    // conditions. Thus we need to let it go out of range.
    #[control]
    #[params]
    level: ParameterType,

    #[cfg_attr(feature = "serialization", serde(skip))]
    sample_rate: SampleRate,
}
impl Serializable for ToyAudioSource {}
impl Generates<StereoSample> for ToyAudioSource {
    fn value(&self) -> StereoSample {
        StereoSample::from(self.level)
    }

    #[allow(unused_variables)]
    fn generate_batch_values(&mut self, values: &mut [StereoSample]) {
        todo!()
    }
}
impl Configurable for ToyAudioSource {
    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
    }
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
    use eframe::{
        egui::{self, Layout, Ui},
        emath::Align,
        epaint::{pos2, Color32, Rect, Rounding, Stroke},
    };
    use groove_core::{
        traits::{gui::Shows, HasUid},
        Normal,
    };

    fn indicator(value: Normal) -> impl egui::Widget + 'static {
        move |ui: &mut egui::Ui| indicator_ui(ui, value)
    }

    fn indicator_ui(ui: &mut egui::Ui, value: Normal) -> egui::Response {
        let desired_size = egui::vec2(2.0, 16.0);
        let (rect, response) =
            ui.allocate_exact_size(desired_size, egui::Sense::focusable_noninteractive());

        if ui.is_rect_visible(rect) {
            ui.painter().rect(
                rect,
                Rounding::default(),
                Color32::BLACK,
                Stroke {
                    width: 1.0,
                    color: Color32::DARK_GRAY,
                },
            );
            let sound_rect = Rect::from_two_pos(
                rect.left_bottom(),
                pos2(
                    rect.right(),
                    rect.bottom() - rect.height() * value.value_as_f32(),
                ),
            );
            ui.painter().rect(
                sound_rect,
                Rounding::default(),
                Color32::YELLOW,
                Stroke {
                    width: 1.0,
                    color: Color32::YELLOW,
                },
            );
        }

        response
    }

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
            let height = ui.available_height();
            ui.set_min_size(ui.available_size());
            ui.set_max_size(ui.available_size());
            if height <= 32.0 {
                self.show_small(ui);
            } else if height <= 128.0 {
                self.show_medium(ui);
            } else {
                self.show_full(ui);
            }
        }
    }
    impl ToySynth {
        fn show_small(&mut self, ui: &mut Ui) {
            ui.horizontal(|ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.label("ToySynth");
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add(indicator(self.max_signal));
                });
            });
            self.degrade_max(0.95);
        }
        fn show_medium(&mut self, ui: &mut Ui) {
            ui.label("ToySynth MEDIUM!");
            let value = Normal::from(0.5);
            ui.add(indicator(value));
        }
        fn show_full(&mut self, ui: &mut Ui) {
            ui.label("ToySynth LARGE!!!!");
            let value = Normal::from(0.8);
            ui.add(indicator(value));
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
