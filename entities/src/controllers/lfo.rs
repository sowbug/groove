// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::messages::EntityMessage;
use core::fmt::Debug;
use groove_core::{
    generators::{Oscillator, WaveformParams},
    midi::HandlesMidi,
    traits::{Generates, IsController, Resets, Ticks, TicksWithMessages},
    ParameterType,
};
use groove_proc_macros::{Control, Synchronization, Uid};
use std::str::FromStr;
use strum::EnumCount;
use strum_macros::{
    Display, EnumCount as EnumCountMacro, EnumIter, EnumString, FromRepr, IntoStaticStr,
};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Synchronization)]
#[cfg_attr(
    feature = "serialization",
    derive(Serialize, Deserialize),
    serde(rename = "lfo", rename_all = "kebab-case")
)]
pub struct LfoControllerParams {
    #[sync]
    pub waveform: WaveformParams,
    #[sync]
    pub frequency: ParameterType,
}

/// Uses an internal LFO as a control source.
#[derive(Control, Debug, Uid)]
pub struct LfoController {
    uid: usize,
    params: LfoControllerParams,
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
        frequency_hz: ParameterType,
    ) -> Self {
        Self {
            uid: Default::default(),
            params: LfoControllerParams {
                waveform: WaveformParams::Sine, // TODO
                frequency: frequency_hz,
            },
            oscillator: Oscillator::new_with_waveform_and_frequency(
                sample_rate,
                waveform,
                frequency_hz,
            ),
        }
    }

    pub fn new_with_params(sample_rate: usize, params: LfoControllerParams) -> Self {
        Self {
            uid: Default::default(),
            params,
            oscillator: Oscillator::new_with_waveform_and_frequency(
                sample_rate,
                WaveformParams::Sine, // TODO: undo the hack with just Sine
                params.frequency,
            ),
        }
    }

    pub fn waveform(&self) -> WaveformParams {
        self.params.waveform()
    }

    pub fn set_waveform(&mut self, waveform: WaveformParams) {
        self.params.set_waveform(waveform);
        self.oscillator.set_waveform(waveform);
    }

    pub fn frequency(&self) -> ParameterType {
        self.params.frequency()
    }

    pub fn set_frequency(&mut self, frequency_hz: ParameterType) {
        // TODO: can we just hand params to oscillator and keep one copy?
        self.params.set_frequency(frequency_hz);
        self.oscillator.set_frequency(frequency_hz);
    }

    pub fn params(&self) -> LfoControllerParams {
        self.params
    }

    pub fn update(&mut self, message: LfoControllerParamsMessage) {
        self.params.update(message)
    }
}
