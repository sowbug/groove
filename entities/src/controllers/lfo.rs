// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::messages::EntityMessage;
use core::fmt::Debug;
use groove_core::{
    generators::{Oscillator, WaveformParams},
    midi::HandlesMidi,
    traits::{Generates, IsController, Resets, Ticks, TicksWithMessages},
    FrequencyHz,
};
use groove_proc_macros::{Nano, Uid};
use std::str::FromStr;
use strum::EnumCount;
use strum_macros::{Display, EnumCount as EnumCountMacro, EnumString, FromRepr, IntoStaticStr};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

/// Uses an internal LFO as a control source.
#[derive(Debug, Nano, Uid)]
pub struct LfoController {
    uid: usize,

    #[nano]
    waveform: WaveformParams,
    #[nano]
    frequency: FrequencyHz,

    oscillator: Oscillator,
}
impl IsController for LfoController {}
impl Resets for LfoController {}
impl TicksWithMessages for LfoController {
    type Message = EntityMessage;

    fn tick(&mut self, tick_count: usize) -> (std::option::Option<Vec<Self::Message>>, usize) {
        self.oscillator.tick(tick_count);
        // TODO: opportunity to use from() to convert properly from 0..1 to -1..0
        (
            Some(vec![EntityMessage::ControlF32(
                self.oscillator.value().value() as f32,
            )]),
            0,
        )
    }
}
impl HandlesMidi for LfoController {}
impl LfoController {
    pub fn new_with(
        sample_rate: usize,
        waveform: WaveformParams,
        frequency_hz: FrequencyHz,
    ) -> Self {
        Self {
            uid: Default::default(),
            oscillator: Oscillator::new_with_waveform_and_frequency(
                sample_rate,
                waveform,
                frequency_hz,
            ),
            waveform,
            frequency: frequency_hz,
        }
    }

    pub fn new_with_params(sample_rate: usize, params: LfoControllerNano) -> Self {
        Self {
            uid: Default::default(),
            oscillator: Oscillator::new_with_waveform_and_frequency(
                sample_rate,
                WaveformParams::Sine, // TODO: undo the hack with just Sine
                params.frequency,
            ),
            waveform: params.waveform(),
            frequency: params.frequency(),
        }
    }

    pub fn waveform(&self) -> WaveformParams {
        self.waveform
    }

    pub fn set_waveform(&mut self, waveform: WaveformParams) {
        self.waveform = waveform;
        self.oscillator.set_waveform(waveform);
    }

    pub fn frequency(&self) -> FrequencyHz {
        self.frequency
    }

    pub fn set_frequency(&mut self, frequency: FrequencyHz) {
        self.frequency = frequency;
        self.oscillator.set_frequency(frequency);
    }

    pub fn update(&mut self, message: LfoControllerMessage) {
        match message {
            LfoControllerMessage::LfoController(s) => {
                *self = Self::new_with_params(self.oscillator.sample_rate(), s)
            }
            LfoControllerMessage::Waveform(waveform) => self.set_waveform(waveform),
            LfoControllerMessage::Frequency(frequency) => self.set_frequency(frequency),
        }
    }
}
