// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    traits::{IsEffect, Resets, TransformsAudio},
    Normal, Sample,
};
use groove_proc_macros::{Control, Params, Uid};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Control, Params, Uid, Serialize, Deserialize)]
pub struct Gain {
    uid: usize,

    #[control]
    #[params]
    ceiling: Normal,
}
impl IsEffect for Gain {}
impl Resets for Gain {
    fn reset(&mut self, _sample_rate: usize) {}
}
impl TransformsAudio for Gain {
    fn transform_channel(&mut self, _channel: usize, input_sample: Sample) -> Sample {
        Sample(input_sample.0 * self.ceiling.value())
    }
}
impl Gain {
    pub fn new_with(params: &GainParams) -> Self {
        Self {
            uid: Default::default(),
            ceiling: params.ceiling,
        }
    }

    pub fn ceiling(&self) -> Normal {
        self.ceiling
    }

    pub fn set_ceiling(&mut self, ceiling: Normal) {
        self.ceiling = ceiling;
    }

    #[cfg(feature = "iced-framework")]
    pub fn update(&mut self, message: GainMessage) {
        match message {
            GainMessage::Gain(s) => *self = Self::new_with(s),
            _ => self.derived_update(message),
        }
    }
}

#[cfg(feature = "egui-framework")]
mod gui {
    use super::Gain;
    use eframe::egui::{DragValue, Ui};
    use groove_core::{traits::gui::Shows, Normal};

    impl Shows for Gain {
        fn show(&mut self, ui: &mut Ui) {
            let mut ceiling = self.ceiling().to_percentage();
            if ui
                .add(
                    DragValue::new(&mut ceiling)
                        .clamp_range(0.0..=100.0)
                        .fixed_decimals(2)
                        .suffix(" %"),
                )
                .changed()
            {
                self.set_ceiling(Normal::from_percentage(ceiling));
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use groove_core::{traits::Generates, StereoSample};
    use groove_toys::{ToyAudioSource, ToyAudioSourceParams};

    #[test]
    fn gain_mainline() {
        let mut gain = Gain::new_with(&GainParams {
            ceiling: Normal::new(0.5),
        });
        assert_eq!(
            gain.transform_audio(
                ToyAudioSource::new_with(&ToyAudioSourceParams {
                    level: ToyAudioSource::LOUD
                })
                .value()
            ),
            StereoSample::from(0.5)
        );
    }
}
