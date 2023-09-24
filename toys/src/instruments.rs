// Copyright (c) 2023 Mike Tsao. All rights reserved.

use eframe::{
    egui::{self, Layout, Ui},
    emath::Align,
    epaint::{pos2, Color32, Rect, Rounding, Stroke},
};
use ensnare_core::{
    generators::{Envelope, EnvelopeParams, Oscillator, OscillatorParams, Waveform},
    instruments::Synthesizer,
    midi::prelude::*,
    modulators::{Dca, DcaParams},
    prelude::*,
    traits::{prelude::*, GeneratesEnvelope},
    voices::{VoiceCount, VoiceStore},
};
use ensnare_proc_macros::{Control, IsInstrument, Params, Uid};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

/// Another [IsInstrument](groove_core::traits::IsInstrument) that was designed
/// for black-box debugging.
#[derive(Debug, Control, IsInstrument, Params, Uid, Serialize, Deserialize)]
pub struct DebugSynth {
    uid: Uid,

    #[control]
    #[params]
    fake_value: Normal,

    #[serde(skip)]
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
                self.oscillator.set_frequency(key.into());
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

    pub fn new() -> Self {
        Self::new_with_components(
            Box::new(Oscillator::new_with(
                &OscillatorParams::default_with_waveform(Waveform::Sine),
            )),
            Box::new(Envelope::new_with(&EnvelopeParams::safe_default())),
        )
    }

    pub fn fake_value(&self) -> Normal {
        self.fake_value
    }

    pub fn set_fake_value(&mut self, fake_value: Normal) {
        self.fake_value = fake_value;
    }
}

impl Displays for DebugSynth {
    fn ui(&mut self, ui: &mut Ui) -> egui::Response {
        ui.label(self.name())
    }
}
