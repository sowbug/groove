// Copyright (c) 2023 Mike Tsao. All rights reserved.

use eframe::egui::Ui;
use ensnare_core::{prelude::*, traits::prelude::*};
use ensnare_proc_macros::{Control, IsEffect, Params, Uid};
use serde::{Deserialize, Serialize};

// TODO: I don't think Mixer needs to exist.
#[derive(Debug, Default, Control, IsEffect, Params, Uid, Serialize, Deserialize)]
pub struct Mixer {
    uid: Uid,
}
impl Serializable for Mixer {}
impl Configurable for Mixer {}
impl TransformsAudio for Mixer {
    fn transform_channel(&mut self, _channel: usize, input_sample: Sample) -> Sample {
        // This is a simple pass-through because it's the job of the
        // infrastructure to provide a sum of all inputs as the input.
        // Eventually this might turn into a weighted mixer, or we might handle
        // that by putting `Gain`s in front.
        input_sample
    }
}
impl Mixer {
    pub fn new_with(_params: &MixerParams) -> Self {
        Self {
            ..Default::default()
        }
    }
}
impl Displays for Mixer {
    fn ui(&mut self, ui: &mut Ui) -> eframe::egui::Response {
        ui.label("I don't have anything!")
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn mixer_mainline() {
        // This could be replaced with a test, elsewhere, showing that
        // Orchestrator's gather_audio() method can gather audio.
    }
}
