// Copyright (c) 2023 Mike Tsao. All rights reserved.

use ensnare::core::{Normal, Sample};
use groove_core::traits::{Configurable, Serializable, TransformsAudio};
use groove_proc_macros::{Control, IsEffect, Params, Uid};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Control, IsEffect, Params, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Limiter {
    uid: groove_core::Uid,

    #[control]
    #[params]
    min: Normal,
    #[control]
    #[params]
    max: Normal,
}
impl Default for Limiter {
    fn default() -> Self {
        Self {
            uid: Default::default(),
            min: Normal::minimum(),
            max: Normal::maximum(),
        }
    }
}
impl Serializable for Limiter {}
impl Configurable for Limiter {}
impl TransformsAudio for Limiter {
    fn transform_channel(&mut self, _channel: usize, input_sample: Sample) -> Sample {
        let sign = input_sample.0.signum();
        Sample::from(
            input_sample
                .0
                .abs()
                .clamp(self.min.value(), self.max.value())
                * sign,
        )
    }
}
impl Limiter {
    pub fn new_with(params: &LimiterParams) -> Self {
        Self {
            min: params.min(),
            max: params.max(),
            ..Default::default()
        }
    }

    pub fn max(&self) -> Normal {
        self.max
    }

    pub fn set_max(&mut self, max: Normal) {
        self.max = max;
    }

    pub fn min(&self) -> Normal {
        self.min
    }

    pub fn set_min(&mut self, min: Normal) {
        self.min = min;
    }
}

#[cfg(feature = "egui-framework")]
mod gui {
    use super::Limiter;
    use eframe::egui::{Slider, Ui};
    use ensnare::prelude::*;
    use groove_core::traits::gui::Displays;

    impl Displays for Limiter {
        fn ui(&mut self, ui: &mut Ui) -> eframe::egui::Response {
            let mut min = self.min().to_percentage();
            let mut max = self.max().to_percentage();
            let min_response = ui.add(
                Slider::new(&mut min, 0.0..=max)
                    .suffix(" %")
                    .text("min")
                    .fixed_decimals(2),
            );
            if min_response.changed() {
                self.set_min(min.into());
            };
            let max_response = ui.add(
                Slider::new(&mut max, min..=1.0)
                    .suffix(" %")
                    .text("max")
                    .fixed_decimals(2),
            );
            if max_response.changed() {
                self.set_max(Normal::from_percentage(max).into());
            };
            min_response | max_response
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ensnare::prelude::*;
    use groove_core::traits::Generates;
    use groove_toys::{ToyAudioSource, ToyAudioSourceParams};
    use more_asserts::{assert_gt, assert_lt};

    #[test]
    fn limiter_mainline() {
        // audio sources are at or past boundaries
        assert_gt!(
            ToyAudioSource::new_with(&ToyAudioSourceParams {
                level: ToyAudioSource::TOO_LOUD
            })
            .value(),
            StereoSample::MAX
        );
        assert_eq!(
            ToyAudioSource::new_with(&ToyAudioSourceParams {
                level: ToyAudioSource::LOUD
            })
            .value(),
            StereoSample::MAX
        );
        assert_eq!(
            ToyAudioSource::new_with(&ToyAudioSourceParams {
                level: ToyAudioSource::SILENT
            })
            .value(),
            StereoSample::SILENCE
        );
        assert_eq!(
            ToyAudioSource::new_with(&ToyAudioSourceParams {
                level: ToyAudioSource::QUIET
            })
            .value(),
            StereoSample::MIN
        );
        assert_lt!(
            ToyAudioSource::new_with(&ToyAudioSourceParams {
                level: ToyAudioSource::TOO_QUIET
            })
            .value(),
            StereoSample::MIN
        );

        // Limiter clamps high and low, and doesn't change values inside the range.
        let mut limiter = Limiter::default();
        assert_eq!(
            limiter.transform_audio(
                ToyAudioSource::new_with(&ToyAudioSourceParams {
                    level: ToyAudioSource::TOO_LOUD
                })
                .value()
            ),
            StereoSample::MAX
        );
        assert_eq!(
            limiter.transform_audio(
                ToyAudioSource::new_with(&ToyAudioSourceParams {
                    level: ToyAudioSource::LOUD
                })
                .value()
            ),
            StereoSample::MAX
        );
        assert_eq!(
            limiter.transform_audio(
                ToyAudioSource::new_with(&ToyAudioSourceParams {
                    level: ToyAudioSource::SILENT
                })
                .value()
            ),
            StereoSample::SILENCE
        );
        assert_eq!(
            limiter.transform_audio(
                ToyAudioSource::new_with(&ToyAudioSourceParams {
                    level: ToyAudioSource::QUIET
                })
                .value()
            ),
            StereoSample::MIN
        );
        assert_eq!(
            limiter.transform_audio(
                ToyAudioSource::new_with(&ToyAudioSourceParams {
                    level: ToyAudioSource::TOO_QUIET
                })
                .value()
            ),
            StereoSample::MIN
        );
    }

    #[test]
    fn limiter_bias() {
        let mut limiter = Limiter::new_with(&LimiterParams {
            min: 0.2.into(),
            max: 0.8.into(),
        });
        assert_eq!(
            limiter.transform_channel(0, Sample::from(0.1)),
            Sample::from(0.2),
            "Limiter failed to clamp min {}",
            0.2
        );
        assert_eq!(
            limiter.transform_channel(0, Sample::from(0.9)),
            Sample::from(0.8),
            "Limiter failed to clamp max {}",
            0.8
        );
        assert_eq!(
            limiter.transform_channel(0, Sample::from(-0.1)),
            Sample::from(-0.2),
            "Limiter failed to clamp min {} for negative values",
            0.2
        );
        assert_eq!(
            limiter.transform_channel(0, Sample::from(-0.9)),
            Sample::from(-0.8),
            "Limiter failed to clamp max {} for negative values",
            0.8
        );
    }
}
