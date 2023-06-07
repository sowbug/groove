// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    traits::{Configurable, IsEffect, TransformsAudio},
    Sample,
};
use groove_proc_macros::{Control, Params, Uid};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Control, Params, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Mixer {
    uid: groove_core::Uid,
}
impl IsEffect for Mixer {}
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

    #[cfg(feature = "iced-framework")]
    #[allow(unreachable_patterns)]
    pub fn update(&mut self, message: MixerMessage) {
        match message {
            MixerMessage::Mixer(s) => *self = Self::new_with(s),
            _ => self.derived_update(message),
        }
    }
}

#[cfg(feature = "egui-framework")]
mod gui {
    use super::Mixer;
    use eframe::egui::Ui;
    use groove_core::traits::gui::Shows;

    impl Shows for Mixer {
        fn show(&mut self, ui: &mut Ui) {
            ui.label("I don't have anything!");
        }
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
