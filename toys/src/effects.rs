// Copyright (c) 2023 Mike Tsao. All rights reserved.

use eframe::egui;
use ensnare_core::{prelude::*, traits::prelude::*};
use ensnare_proc_macros::{Control, IsEffect, Params, Uid};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// An [IsEffect](groove_core::traits::IsEffect) that negates the input signal.
#[derive(Debug, Default, Control, IsEffect, Params, Uid, Serialize, Deserialize)]
pub struct ToyEffect {
    #[serde(skip)]
    uid: Uid,

    #[control]
    #[params]
    my_value: Normal,

    #[serde(skip)]
    sample_rate: SampleRate,
}
impl TransformsAudio for ToyEffect {
    fn transform_channel(&mut self, _channel: usize, input_sample: Sample) -> Sample {
        -input_sample
    }
}
impl Configurable for ToyEffect {
    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }
}
impl Serializable for ToyEffect {}

impl Displays for ToyEffect {
    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        ui.label(self.name())
    }
}
impl ToyEffect {
    pub fn my_value(&self) -> Normal {
        self.my_value
    }

    pub fn set_my_value(&mut self, my_value: Normal) {
        self.my_value = my_value;
    }
}
