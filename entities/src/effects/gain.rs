// Copyright (c) 2023 Mike Tsao. All rights reserved.

use ensnare::core::{Normal, Sample};
use groove_core::traits::{Configurable, Serializable, TransformsAudio};
use groove_proc_macros::{Control, IsEffect, Params, Uid};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Control, IsEffect, Params, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Gain {
    uid: groove_core::Uid,

    #[control]
    #[params]
    ceiling: Normal,
}
impl Serializable for Gain {}
impl Configurable for Gain {}
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
}

#[cfg(feature = "egui-framework")]
mod gui {
    use super::Gain;
    use eframe::egui::{DragValue, Ui};
    use ensnare::core::Normal;
    use groove_core::traits::gui::Displays;

    impl Displays for Gain {
        fn ui(&mut self, ui: &mut Ui) -> eframe::egui::Response {
            let mut ceiling = self.ceiling().to_percentage();
            let response = ui.add(
                DragValue::new(&mut ceiling)
                    .clamp_range(0.0..=100.0)
                    .fixed_decimals(2)
                    .suffix(" %"),
            );
            if response.changed() {
                self.set_ceiling(Normal::from_percentage(ceiling));
            };
            response
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ensnare::core::StereoSample;
    use groove_core::traits::Generates;
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
