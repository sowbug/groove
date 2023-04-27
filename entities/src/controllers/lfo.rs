// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::messages::EntityMessage;
use core::fmt::Debug;
use eframe::egui::{ComboBox, Slider};
use groove_core::{
    generators::{Oscillator, OscillatorNano, Waveform},
    midi::HandlesMidi,
    traits::{Generates, IsController, Performs, Resets, Ticks, TicksWithMessages},
    FrequencyHz, Normal, ParameterType,
};
use groove_proc_macros::{Nano, Uid};
use std::{ops::RangeInclusive, str::FromStr};
use strum::EnumCount;
use strum_macros::{Display, EnumCount as EnumCountMacro, EnumString, FromRepr, IntoStaticStr};

#[cfg(feature = "egui-framework")]
use {eframe::egui, groove_core::traits::Shows, strum::IntoEnumIterator};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

/// Uses an internal LFO as a control source.
#[derive(Debug, Nano, Uid)]
pub struct LfoController {
    uid: usize,

    #[nano]
    waveform: Waveform,
    #[nano]
    frequency: FrequencyHz,

    oscillator: Oscillator,

    is_performing: bool,
}
impl IsController for LfoController {}
impl Resets for LfoController {
    fn reset(&mut self, sample_rate: usize) {
        self.oscillator.reset(sample_rate);
    }
}
impl TicksWithMessages for LfoController {
    type Message = EntityMessage;

    fn tick(&mut self, tick_count: usize) -> (std::option::Option<Vec<Self::Message>>, usize) {
        self.oscillator.tick(tick_count);
        (
            Some(vec![EntityMessage::ControlF32(
                Normal::from(self.oscillator.value()).into(),
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
    pub fn new_with(params: LfoControllerNano) -> Self {
        Self {
            uid: Default::default(),
            oscillator: Oscillator::new_with(OscillatorNano {
                waveform: params.waveform,
                frequency: params.frequency,
                ..Default::default()
            }),
            waveform: params.waveform(),
            frequency: params.frequency(),
            is_performing: false,
        }
    }

    pub const fn frequency_range() -> RangeInclusive<ParameterType> {
        0.0..=100.0
    }

    pub fn waveform(&self) -> Waveform {
        self.waveform
    }

    pub fn set_waveform(&mut self, waveform: Waveform) {
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
            LfoControllerMessage::LfoController(s) => *self = Self::new_with(s),
            _ => self.derived_update(message),
        }
    }
}

#[cfg(feature = "egui-framework")]
impl Shows for LfoController {
    fn show(&mut self, ui: &mut egui::Ui) {
        let mut frequency = self.frequency().value();
        let mut waveform = self.waveform();
        if ui
            .add(Slider::new(&mut frequency, LfoController::frequency_range()).text("Frequency"))
            .changed()
        {
            self.set_frequency(frequency.into());
        };
        ComboBox::new(ui.next_auto_id(), "Waveform")
            .selected_text(waveform.to_string())
            .show_ui(ui, |ui| {
                for w in Waveform::iter() {
                    ui.selectable_value(&mut waveform, w, w.to_string());
                }
            });
        if waveform != self.waveform() {
            eprintln!("changed {} {}", self.waveform(), waveform);
            self.set_waveform(waveform);
        }
    }
}
