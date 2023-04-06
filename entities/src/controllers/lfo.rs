// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::messages::EntityMessage;
use core::fmt::Debug;
use groove_core::{
    generators::{Oscillator, OscillatorNano, WaveformParams},
    midi::HandlesMidi,
    traits::{Generates, IsController, Performs, Resets, Ticks, TicksWithMessages},
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

    is_performing: bool,
}
impl IsController for LfoController {}
impl Resets for LfoController {}
impl TicksWithMessages for LfoController {
    type Message = EntityMessage;

    fn tick(&mut self, tick_count: usize) -> (std::option::Option<Vec<Self::Message>>, usize) {
        self.oscillator.tick(tick_count);
        (
            Some(vec![EntityMessage::ControlF32(
                self.oscillator.value().into(),
            )]),
            0,
        )
    }
}
impl HandlesMidi for LfoController {}
impl Performs for LfoController {
    fn play(&mut self) {
        self.is_performing = true;
    }

    fn stop(&mut self) {
        self.is_performing = false;
    }

    fn skip_to_start(&mut self) {
        // TODO: think how important it is for LFO oscillator to start at zero
    }
}
impl LfoController {
    pub fn new_with(sample_rate: usize, params: LfoControllerNano) -> Self {
        Self {
            uid: Default::default(),
            oscillator: Oscillator::new_with(
                sample_rate,
                OscillatorNano {
                    waveform: params.waveform,
                    frequency: params.frequency,
                    ..Default::default()
                },
            ),
            waveform: params.waveform(),
            frequency: params.frequency(),
            is_performing: false,
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
                *self = Self::new_with(self.oscillator.sample_rate(), s)
            }
            _ => self.derived_update(message),
        }
    }
}
