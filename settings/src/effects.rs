// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_entities::effects::{
    BiQuadFilterLowPass24db, BiQuadFilterLowPass24dbNano, Bitcrusher, BitcrusherNano, Chorus,
    ChorusNano, Compressor, CompressorNano, Delay, DelayNano, Gain, GainNano, Limiter, LimiterNano,
    Mixer, MixerNano, Reverb, ReverbNano,
};
use groove_orchestration::Entity;
use groove_toys::{ToyEffect, ToyEffectNano};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EffectSettings {
    #[serde(rename_all = "kebab-case")]
    Toy(ToyEffectNano),
    #[serde(rename_all = "kebab-case")]
    Mixer(MixerNano),
    #[serde(rename_all = "kebab-case")]
    Gain(GainNano),
    #[serde(rename_all = "kebab-case")]
    Limiter(LimiterNano),
    #[serde(rename_all = "kebab-case")]
    Bitcrusher(BitcrusherNano),
    #[serde(rename_all = "kebab-case")]
    Chorus(ChorusNano),
    #[serde(rename_all = "kebab-case")]
    Compressor(CompressorNano),
    #[serde(rename_all = "kebab-case")]
    Delay(DelayNano),
    #[serde(rename_all = "kebab-case")]
    Reverb(ReverbNano),
    // #[serde(rename = "filter-low-pass-12db")]
    // FilterLowPass12db { cutoff: ParameterType, q: f32 },
    #[serde(rename = "filter-low-pass-24db", rename_all = "kebab-case")]
    FilterLowPass24db(BiQuadFilterLowPass24dbNano),
    // #[serde(rename = "filter-high-pass-12db")]
    // FilterHighPass12db { cutoff: ParameterType, q: f32 },
    // #[serde(rename = "filter-band-pass-12db")]
    // FilterBandPass12db {
    //     cutoff: ParameterType,
    //     bandwidth: f32,
    // },
    // #[serde(rename = "filter-band-stop-12db")]
    // FilterBandStop12db {
    //     cutoff: ParameterType,
    //     bandwidth: f32,
    // },
    // #[serde(rename = "filter-all-pass-12db")]
    // FilterAllPass12db { cutoff: ParameterType, q: f32 },
    // #[serde(rename = "filter-peaking-eq-12db")]
    // #[serde(rename_all = "kebab-case")]
    // FilterPeakingEq12db { cutoff: ParameterType, db_gain: f32 },
    // #[serde(rename = "filter-low-shelf-12db")]
    // #[serde(rename_all = "kebab-case")]
    // FilterLowShelf12db { cutoff: ParameterType, db_gain: f32 },
    // #[serde(rename = "filter-high-shelf-12db")]
    // #[serde(rename_all = "kebab-case")]
    // FilterHighShelf12db { cutoff: ParameterType, db_gain: f32 },
}

impl EffectSettings {
    pub(crate) fn instantiate(&self, sample_rate: usize, load_only_test_entities: bool) -> Entity {
        if load_only_test_entities {
            return Entity::ToyEffect(Box::new(ToyEffect::default()));
        }
        match *self {
            EffectSettings::Toy(params) => Entity::ToyEffect(Box::new(ToyEffect::new_with(params))),
            EffectSettings::Mixer(params) => Entity::Mixer(Box::new(Mixer::new_with(params))),
            EffectSettings::Limiter(params) => {
                Entity::Limiter(Box::new(Limiter::new_with_params(params)))
            }
            EffectSettings::Gain(params) => Entity::Gain(Box::new(Gain::new_with(params))),
            EffectSettings::Bitcrusher(params) => {
                Entity::Bitcrusher(Box::new(Bitcrusher::new_with(params)))
            }
            EffectSettings::Compressor(params) => {
                Entity::Compressor(Box::new(Compressor::new_with(params)))
            }
            // EffectSettings::FilterLowPass12db { cutoff, q } => {
            //     Entity::BiQuadFilter(Box::new(BiQuadFilter::new_with(
            //         sample_rate,
            //         BiQuadFilterNano {
            //             cutoff: cutoff.into(),
            //             q: q.into(),
            //         },
            //     )))
            // }
            EffectSettings::FilterLowPass24db(params) => Entity::BiQuadFilterLowPass24db(Box::new(
                BiQuadFilterLowPass24db::new_with(sample_rate, params),
            )),

            // EffectSettings::FilterHighPass12db { cutoff, q } => {
            //     Entity::BiQuadFilter(Box::new(BiQuadFilter::new_with(
            //         sample_rate,
            //         BiQuadFilterNano {
            //             cutoff: cutoff.into(),
            //             q: q.into(),
            //         },
            //     )))
            // }
            // EffectSettings::FilterBandPass12db { cutoff, bandwidth } => {
            //     Entity::BiQuadFilter(Box::new(BiQuadFilter::new_with(
            //         sample_rate,
            //         BiQuadFilterNano {
            //             cutoff: cutoff.into(),
            //             q: bandwidth.into(),
            //         },
            //     )))
            // }
            // EffectSettings::FilterBandStop12db { cutoff, bandwidth } => {
            //     Entity::BiQuadFilter(Box::new(BiQuadFilter::new_with(
            //         sample_rate,
            //         BiQuadFilterNano {
            //             cutoff: cutoff.into(),
            //             q: bandwidth.into(),
            //         },
            //     )))
            // }
            // EffectSettings::FilterAllPass12db { cutoff, q } => {
            //     Entity::BiQuadFilter(Box::new(BiQuadFilter::new_with(
            //         sample_rate,
            //         BiQuadFilterNano {
            //             cutoff: cutoff.into(),
            //             q: q.into(),
            //         },
            //     )))
            // }
            // EffectSettings::FilterPeakingEq12db { cutoff, db_gain } => {
            //     Entity::BiQuadFilter(Box::new(BiQuadFilter::new_with(
            //         sample_rate,
            //         BiQuadFilterNano {
            //             cutoff: cutoff.into(),
            //             q: db_gain.into(),
            //         },
            //     )))
            // }
            // EffectSettings::FilterLowShelf12db { cutoff, db_gain } => {
            //     Entity::BiQuadFilter(Box::new(BiQuadFilter::new_with(
            //         sample_rate,
            //         BiQuadFilterNano {
            //             cutoff: cutoff.into(),
            //             q: db_gain.into(),
            //         },
            //     )))
            // }
            // EffectSettings::FilterHighShelf12db { cutoff, db_gain } => {
            //     Entity::BiQuadFilter(Box::new(BiQuadFilter::new_with(
            //         sample_rate,
            //         BiQuadFilterNano {
            //             cutoff: cutoff.into(),
            //             q: db_gain.into(),
            //         },
            //     )))
            // }
            EffectSettings::Delay(params) => {
                Entity::Delay(Box::new(Delay::new_with(sample_rate, params)))
            }
            EffectSettings::Reverb(nano) => {
                Entity::Reverb(Box::new(Reverb::new_with(sample_rate, nano)))
            }
            EffectSettings::Chorus(nano) => {
                Entity::Chorus(Box::new(Chorus::new_with(sample_rate, nano)))
            }
        }
    }
}
