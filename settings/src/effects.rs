// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_entities::effects::{
    BiQuadFilterAllPass, BiQuadFilterAllPassParams, BiQuadFilterBandPass, BiQuadFilterBandPassParams,
    BiQuadFilterBandStop, BiQuadFilterBandStopParams, BiQuadFilterHighPass, BiQuadFilterHighPassParams,
    BiQuadFilterHighShelf, BiQuadFilterHighShelfParams, BiQuadFilterLowPass12db,
    BiQuadFilterLowPass12dbParams, BiQuadFilterLowPass24db, BiQuadFilterLowPass24dbParams,
    BiQuadFilterLowShelf, BiQuadFilterLowShelfParams, BiQuadFilterPeakingEq,
    BiQuadFilterPeakingEqParams, Bitcrusher, BitcrusherParams, Chorus, ChorusParams, Compressor,
    CompressorParams, Delay, DelayParams, Gain, GainParams, Limiter, LimiterParams, Mixer, MixerParams,
    Reverb, ReverbParams,
};
use groove_orchestration::Entity;
use groove_toys::{ToyEffect, ToyEffectParams};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EffectSettings {
    #[serde(rename_all = "kebab-case")]
    Toy(ToyEffectParams),
    #[serde(rename_all = "kebab-case")]
    Mixer(MixerParams),
    #[serde(rename_all = "kebab-case")]
    Gain(GainParams),
    #[serde(rename_all = "kebab-case")]
    Limiter(LimiterParams),
    #[serde(rename_all = "kebab-case")]
    Bitcrusher(BitcrusherParams),
    #[serde(rename_all = "kebab-case")]
    Chorus(ChorusParams),
    #[serde(rename_all = "kebab-case")]
    Compressor(CompressorParams),
    #[serde(rename_all = "kebab-case")]
    Delay(DelayParams),
    #[serde(rename_all = "kebab-case")]
    Reverb(ReverbParams),
    #[serde(rename = "filter-low-pass-12db")]
    FilterLowPass12db(BiQuadFilterLowPass12dbParams),
    #[serde(rename = "filter-low-pass-24db")]
    FilterLowPass24db(BiQuadFilterLowPass24dbParams),
    #[serde(rename = "filter-high-pass-12db")]
    FilterHighPass12db(BiQuadFilterHighPassParams),
    #[serde(rename = "filter-band-pass-12db")]
    FilterBandPass12db(BiQuadFilterBandPassParams),
    #[serde(rename = "filter-band-stop-12db")]
    FilterBandStop12db(BiQuadFilterBandStopParams),
    #[serde(rename = "filter-all-pass-12db")]
    FilterAllPass12db(BiQuadFilterAllPassParams),
    #[serde(rename = "filter-peaking-eq-12db")]
    FilterPeakingEq12db(BiQuadFilterPeakingEqParams),
    #[serde(rename = "filter-low-shelf-12db")]
    FilterLowShelf12db(BiQuadFilterLowShelfParams),
    #[serde(rename = "filter-high-shelf-12db")]
    FilterHighShelf12db(BiQuadFilterHighShelfParams),
}

impl EffectSettings {
    pub(crate) fn instantiate(&self, load_only_test_entities: bool) -> Entity {
        if load_only_test_entities {
            return Entity::ToyEffect(Box::new(ToyEffect::default()));
        }
        match self {
            EffectSettings::Toy(params) => {
                Entity::ToyEffect(Box::new(ToyEffect::new_with(params.clone())))
            }
            EffectSettings::Mixer(params) => {
                Entity::Mixer(Box::new(Mixer::new_with(params.clone())))
            }
            EffectSettings::Limiter(params) => {
                Entity::Limiter(Box::new(Limiter::new_with(params.clone())))
            }
            EffectSettings::Gain(params) => Entity::Gain(Box::new(Gain::new_with(params.clone()))),
            EffectSettings::Bitcrusher(params) => {
                Entity::Bitcrusher(Box::new(Bitcrusher::new_with(params.clone())))
            }
            EffectSettings::Compressor(params) => {
                Entity::Compressor(Box::new(Compressor::new_with(params.clone())))
            }
            EffectSettings::FilterLowPass12db(params) => Entity::BiQuadFilterLowPass12db(Box::new(
                BiQuadFilterLowPass12db::new_with(params.clone()),
            )),
            EffectSettings::FilterLowPass24db(params) => Entity::BiQuadFilterLowPass24db(Box::new(
                BiQuadFilterLowPass24db::new_with(params.clone()),
            )),
            EffectSettings::FilterHighPass12db(params) => Entity::BiQuadFilterHighPass(Box::new(
                BiQuadFilterHighPass::new_with(params.clone()),
            )),
            EffectSettings::FilterBandPass12db(params) => Entity::BiQuadFilterBandPass(Box::new(
                BiQuadFilterBandPass::new_with(params.clone()),
            )),
            EffectSettings::FilterBandStop12db(params) => Entity::BiQuadFilterBandStop(Box::new(
                BiQuadFilterBandStop::new_with(params.clone()),
            )),
            EffectSettings::FilterAllPass12db(params) => {
                Entity::BiQuadFilterAllPass(Box::new(BiQuadFilterAllPass::new_with(params.clone())))
            }
            EffectSettings::FilterPeakingEq12db(params) => Entity::BiQuadFilterPeakingEq(Box::new(
                BiQuadFilterPeakingEq::new_with(params.clone()),
            )),
            EffectSettings::FilterLowShelf12db(params) => Entity::BiQuadFilterLowShelf(Box::new(
                BiQuadFilterLowShelf::new_with(params.clone()),
            )),
            EffectSettings::FilterHighShelf12db(params) => Entity::BiQuadFilterHighShelf(Box::new(
                BiQuadFilterHighShelf::new_with(params.clone()),
            )),
            EffectSettings::Delay(params) => {
                Entity::Delay(Box::new(Delay::new_with(params.clone())))
            }
            EffectSettings::Reverb(params) => {
                Entity::Reverb(Box::new(Reverb::new_with(params.clone())))
            }
            EffectSettings::Chorus(params) => {
                Entity::Chorus(Box::new(Chorus::new_with(params.clone())))
            }
        }
    }
}
