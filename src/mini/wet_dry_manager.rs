// Copyright (c) 2023 Mike Tsao. All rights reserved.

use std::collections::HashMap;

use groove_core::{Normal, Sample, StereoSample, Uid};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct WetDryManager {
    uid_to_wetness: HashMap<Uid, Normal>,
}
impl WetDryManager {
    pub fn get(&self, uid: &Uid) -> Normal {
        if let Some(wetness) = self.uid_to_wetness.get(uid) {
            *wetness
        } else {
            Normal::default()
        }
    }

    #[allow(dead_code)]
    pub fn set(&mut self, uid: Uid, wetness: Normal) {
        self.uid_to_wetness.insert(uid, wetness);
    }

    pub fn transform_audio(
        &mut self,
        wetness: Normal,
        pre_effect: StereoSample,
        post_effect: StereoSample,
    ) -> StereoSample {
        StereoSample(
            self.transform_channel(wetness, 0, pre_effect.0, post_effect.0),
            self.transform_channel(wetness, 1, pre_effect.1, post_effect.1),
        )
    }

    fn transform_channel(
        &mut self,
        wetness: Normal,
        _: usize,
        pre_effect: Sample,
        post_effect: Sample,
    ) -> Sample {
        let wetness = wetness.value();
        let dryness = 1.0 - wetness;
        post_effect * wetness + pre_effect * dryness
    }
}

#[cfg(test)]
mod tests {
    use crate::mini::wet_dry_manager::WetDryManager;
    use groove_core::{traits::TransformsAudio, Normal, Sample, Uid};
    use groove_toys::ToyEffect;

    #[test]
    fn lookups_work() {
        let mut wd = WetDryManager::default();
        assert_eq!(
            wd.get(&Uid(1)),
            Normal::maximum(),
            "a missing Uid should return default wetness 1.0"
        );

        let uid = Uid(1);
        wd.set(uid, Normal::from(0.5));
        assert_eq!(
            wd.get(&Uid(1)),
            Normal::from(0.5),
            "a non-missing Uid should return the wetness that we set"
        );
    }

    #[test]
    fn processing_works() {
        let mut wd = WetDryManager::default();

        let mut effect = ToyEffect::default();
        assert_eq!(
            effect.transform_channel(0, Sample::MAX),
            Sample::MIN,
            "we expected ToyEffect to negate the input"
        );

        let pre_effect = Sample::MAX;
        assert_eq!(
            wd.transform_channel(
                Normal::maximum(),
                0,
                pre_effect,
                effect.transform_channel(0, pre_effect),
            ),
            Sample::MIN,
            "Wetness 1.0 means full effect, zero pre-effect"
        );
        assert_eq!(
            wd.transform_channel(
                Normal::from_percentage(50.0),
                0,
                pre_effect,
                effect.transform_channel(0, pre_effect),
            ),
            Sample::from(0.0),
            "Wetness 0.5 means even parts effect and pre-effect"
        );
        assert_eq!(
            wd.transform_channel(
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
