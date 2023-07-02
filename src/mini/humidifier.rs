// Copyright (c) 2023 Mike Tsao. All rights reserved.

use std::collections::HashMap;

use groove_core::{Normal, Sample, StereoSample, Uid};
use serde::{Deserialize, Serialize};

/// Controls the wet/dry mix of arranged effects.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Humidifier {
    uid_to_humidity: HashMap<Uid, Normal>,
}
impl Humidifier {
    pub fn get_humidity_by_uid(&self, uid: &Uid) -> Normal {
        if let Some(humidity) = self.uid_to_humidity.get(uid) {
            *humidity
        } else {
            Normal::default()
        }
    }

    #[allow(dead_code)]
    pub fn set_humidity_by_uid(&mut self, uid: Uid, humidity: Normal) {
        self.uid_to_humidity.insert(uid, humidity);
    }

    pub fn transform_audio(
        &mut self,
        humidity: Normal,
        pre_effect: StereoSample,
        post_effect: StereoSample,
    ) -> StereoSample {
        StereoSample(
            self.transform_channel(humidity, 0, pre_effect.0, post_effect.0),
            self.transform_channel(humidity, 1, pre_effect.1, post_effect.1),
        )
    }

    fn transform_channel(
        &mut self,
        humidity: Normal,
        _: usize,
        pre_effect: Sample,
        post_effect: Sample,
    ) -> Sample {
        let humidity = humidity.value();
        let aridity = 1.0 - humidity;
        post_effect * humidity + pre_effect * aridity
    }
}

#[cfg(test)]
mod tests {
    use crate::mini::humidifier::Humidifier;
    use groove_core::{traits::TransformsAudio, Normal, Sample, Uid};
    use groove_toys::ToyEffect;

    #[test]
    fn lookups_work() {
        let mut wd = Humidifier::default();
        assert_eq!(
            wd.get_humidity_by_uid(&Uid(1)),
            Normal::maximum(),
            "a missing Uid should return default humidity 1.0"
        );

        let uid = Uid(1);
        wd.set_humidity_by_uid(uid, Normal::from(0.5));
        assert_eq!(
            wd.get_humidity_by_uid(&Uid(1)),
            Normal::from(0.5),
            "a non-missing Uid should return the humidity that we set"
        );
    }

    #[test]
    fn processing_works() {
        let mut humidifier = Humidifier::default();

        let mut effect = ToyEffect::default();
        assert_eq!(
            effect.transform_channel(0, Sample::MAX),
            Sample::MIN,
            "we expected ToyEffect to negate the input"
        );

        let pre_effect = Sample::MAX;
        assert_eq!(
            humidifier.transform_channel(
                Normal::maximum(),
                0,
                pre_effect,
                effect.transform_channel(0, pre_effect),
            ),
            Sample::MIN,
            "Wetness 1.0 means full effect, zero pre-effect"
        );
        assert_eq!(
            humidifier.transform_channel(
                Normal::from_percentage(50.0),
                0,
                pre_effect,
                effect.transform_channel(0, pre_effect),
            ),
            Sample::from(0.0),
            "Wetness 0.5 means even parts effect and pre-effect"
        );
        assert_eq!(
            humidifier.transform_channel(
                Normal::zero(),
                0,
                pre_effect,
                effect.transform_channel(0, pre_effect),
            ),
            pre_effect,
            "Wetness 0.0 means no change from pre-effect to post"
        );
    }
}
