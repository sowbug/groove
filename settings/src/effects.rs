// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_entities::effects::{
    BiQuadFilterAllPass, BiQuadFilterAllPassParams, BiQuadFilterBandPass,
    BiQuadFilterBandPassParams, BiQuadFilterBandStop, BiQuadFilterBandStopParams,
    BiQuadFilterHighPass, BiQuadFilterHighPassParams, BiQuadFilterHighShelf,
    BiQuadFilterHighShelfParams, BiQuadFilterLowPass12db, BiQuadFilterLowPass12dbParams,
    BiQuadFilterLowPass24db, BiQuadFilterLowPass24dbParams, BiQuadFilterLowShelf,
    BiQuadFilterLowShelfParams, BiQuadFilterPeakingEq, BiQuadFilterPeakingEqParams, Bitcrusher,
    BitcrusherParams, Chorus, ChorusParams, Compressor, CompressorParams, Delay, DelayParams, Gain,
    GainParams, Limiter, LimiterParams, Mixer, MixerParams, Reverb, ReverbParams,
};
use groove_orchestration::EntityObsolete;
use groove_toys::{ToyEffect, ToyEffectParams};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
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
    pub(crate) fn instantiate(&self, load_only_test_entities: bool) -> EntityObsolete {
        if load_only_test_entities {
            return EntityObsolete::ToyEffect(Box::new(ToyEffect::default()));
        }
        match self {
            EffectSettings::Toy(params) => {
                EntityObsolete::ToyEffect(Box::new(ToyEffect::new_with(&params)))
            }
            EffectSettings::Mixer(params) => {
                EntityObsolete::Mixer(Box::new(Mixer::new_with(&params)))
            }
            EffectSettings::Limiter(params) => {
                EntityObsolete::Limiter(Box::new(Limiter::new_with(&params)))
            }
            EffectSettings::Gain(params) => EntityObsolete::Gain(Box::new(Gain::new_with(&params))),
            EffectSettings::Bitcrusher(params) => {
                EntityObsolete::Bitcrusher(Box::new(Bitcrusher::new_with(&params)))
            }
            EffectSettings::Compressor(params) => {
                EntityObsolete::Compressor(Box::new(Compressor::new_with(&params)))
            }
            EffectSettings::FilterLowPass12db(params) => EntityObsolete::BiQuadFilterLowPass12db(
                Box::new(BiQuadFilterLowPass12db::new_with(&params)),
            ),
            EffectSettings::FilterLowPass24db(params) => EntityObsolete::BiQuadFilterLowPass24db(
                Box::new(BiQuadFilterLowPass24db::new_with(&params)),
            ),
            EffectSettings::FilterHighPass12db(params) => EntityObsolete::BiQuadFilterHighPass(
                Box::new(BiQuadFilterHighPass::new_with(&params)),
            ),
            EffectSettings::FilterBandPass12db(params) => EntityObsolete::BiQuadFilterBandPass(
                Box::new(BiQuadFilterBandPass::new_with(&params)),
            ),
            EffectSettings::FilterBandStop12db(params) => EntityObsolete::BiQuadFilterBandStop(
                Box::new(BiQuadFilterBandStop::new_with(&params)),
            ),
            EffectSettings::FilterAllPass12db(params) => EntityObsolete::BiQuadFilterAllPass(
                Box::new(BiQuadFilterAllPass::new_with(&params)),
            ),
            EffectSettings::FilterPeakingEq12db(params) => EntityObsolete::BiQuadFilterPeakingEq(
                Box::new(BiQuadFilterPeakingEq::new_with(&params)),
            ),
            EffectSettings::FilterLowShelf12db(params) => EntityObsolete::BiQuadFilterLowShelf(
                Box::new(BiQuadFilterLowShelf::new_with(&params)),
            ),
            EffectSettings::FilterHighShelf12db(params) => EntityObsolete::BiQuadFilterHighShelf(
                Box::new(BiQuadFilterHighShelf::new_with(&params)),
            ),
            EffectSettings::Delay(params) => {
                EntityObsolete::Delay(Box::new(Delay::new_with(&params)))
            }
            EffectSettings::Reverb(params) => {
                EntityObsolete::Reverb(Box::new(Reverb::new_with(&params)))
            }
            EffectSettings::Chorus(params) => {
                EntityObsolete::Chorus(Box::new(Chorus::new_with(&params)))
            }
        }
    }
}
