// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{Normal, ParameterType};
use groove_entities::effects::{
    BiQuadFilter, Bitcrusher, Chorus, Compressor, Delay, Gain, Limiter, Mixer, NanoBiQuadFilter,
    NanoBitcrusher, NanoChorus, NanoCompressor, NanoDelay, NanoLimiter, NanoReverb, Reverb,
};
use groove_orchestration::Entity;
use groove_toys::ToyEffect;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EffectSettings {
    #[serde(rename_all = "kebab-case")]
    Test {},
    #[serde(rename_all = "kebab-case")]
    Mixer {},
    #[serde(rename_all = "kebab-case")]
    Gain { ceiling: f32 },
    #[serde(rename_all = "kebab-case")]
    Limiter(NanoLimiter),
    #[serde(rename_all = "kebab-case")]
    Bitcrusher(NanoBitcrusher),
    #[serde(rename_all = "kebab-case")]
    Chorus {
        wet_dry_mix: f32,
        voices: usize,
        delay_factor: usize,
    },
    #[serde(rename_all = "kebab-case")]
    Compressor {
        threshold: f32,
        ratio: f32,
        attack: f32,
        release: f32,
    },
    #[serde(rename_all = "kebab-case")]
    Delay { seconds: f64 },
    #[serde(rename_all = "kebab-case")]
    Reverb {
        wet_dry_mix: f32,
        attenuation: f64,
        seconds: f64,
    },
    #[serde(rename = "filter-low-pass-12db")]
    FilterLowPass12db { cutoff: ParameterType, q: f32 },
    #[serde(rename = "filter-low-pass-24db", rename_all = "kebab-case")]
    FilterLowPass24db {
        cutoff: ParameterType,
        passband_ripple: f32,
    },
    #[serde(rename = "filter-high-pass-12db")]
    FilterHighPass12db { cutoff: ParameterType, q: f32 },
    #[serde(rename = "filter-band-pass-12db")]
    FilterBandPass12db {
        cutoff: ParameterType,
        bandwidth: f32,
    },
    #[serde(rename = "filter-band-stop-12db")]
    FilterBandStop12db {
        cutoff: ParameterType,
        bandwidth: f32,
    },
    #[serde(rename = "filter-all-pass-12db")]
    FilterAllPass12db { cutoff: ParameterType, q: f32 },
    #[serde(rename = "filter-peaking-eq-12db")]
    #[serde(rename_all = "kebab-case")]
    FilterPeakingEq12db { cutoff: ParameterType, db_gain: f32 },
    #[serde(rename = "filter-low-shelf-12db")]
    #[serde(rename_all = "kebab-case")]
    FilterLowShelf12db { cutoff: ParameterType, db_gain: f32 },
    #[serde(rename = "filter-high-shelf-12db")]
    #[serde(rename_all = "kebab-case")]
    FilterHighShelf12db { cutoff: ParameterType, db_gain: f32 },
}

impl EffectSettings {
    pub(crate) fn instantiate(&self, sample_rate: usize, load_only_test_entities: bool) -> Entity {
        if load_only_test_entities {
            return Entity::ToyEffect(Box::new(ToyEffect::default()));
        }
        match *self {
            EffectSettings::Test {} => Entity::ToyEffect(Box::new(ToyEffect::default())),
            EffectSettings::Mixer {} => Entity::Mixer(Box::new(Mixer::default())),
            EffectSettings::Limiter(params) => {
                Entity::Limiter(Box::new(Limiter::new_with_params(params)))
            }
            EffectSettings::Gain { ceiling } => {
                Entity::Gain(Box::new(Gain::new_with(Normal::new_from_f32(ceiling))))
            }
            EffectSettings::Bitcrusher(params) => {
                Entity::Bitcrusher(Box::new(Bitcrusher::new_with_params(params)))
            }
            EffectSettings::Compressor {
                threshold,
                ratio,
                attack,
                release,
            } => Entity::Compressor(Box::new(Compressor::new_with(NanoCompressor {
                threshold: threshold.into(),
                ratio: ratio.into(),
                attack: attack.into(),
                release: release.into(),
            }))),
            EffectSettings::FilterLowPass12db { cutoff, q } => {
                Entity::BiQuadFilter(Box::new(BiQuadFilter::new_with(
                    sample_rate,
                    &NanoBiQuadFilter {
                        cutoff,
                        q: q.into(),
                    },
                )))
            }
            EffectSettings::FilterLowPass24db {
                cutoff,
                passband_ripple,
            } => Entity::BiQuadFilter(Box::new(BiQuadFilter::new_with(
                sample_rate,
                &NanoBiQuadFilter {
                    cutoff,
                    q: passband_ripple.into(),
                },
            ))),

            EffectSettings::FilterHighPass12db { cutoff, q } => {
                Entity::BiQuadFilter(Box::new(BiQuadFilter::new_with(
                    sample_rate,
                    &NanoBiQuadFilter {
                        cutoff,
                        q: q.into(),
                    },
                )))
            }
            EffectSettings::FilterBandPass12db { cutoff, bandwidth } => {
                Entity::BiQuadFilter(Box::new(BiQuadFilter::new_with(
                    sample_rate,
                    &NanoBiQuadFilter {
                        cutoff,
                        q: bandwidth.into(),
                    },
                )))
            }
            EffectSettings::FilterBandStop12db { cutoff, bandwidth } => {
                Entity::BiQuadFilter(Box::new(BiQuadFilter::new_with(
                    sample_rate,
                    &NanoBiQuadFilter {
                        cutoff,
                        q: bandwidth.into(),
                    },
                )))
            }
            EffectSettings::FilterAllPass12db { cutoff, q } => {
                Entity::BiQuadFilter(Box::new(BiQuadFilter::new_with(
                    sample_rate,
                    &NanoBiQuadFilter {
                        cutoff,
                        q: q.into(),
                    },
                )))
            }
            EffectSettings::FilterPeakingEq12db { cutoff, db_gain } => {
                Entity::BiQuadFilter(Box::new(BiQuadFilter::new_with(
                    sample_rate,
                    &NanoBiQuadFilter {
                        cutoff,
                        q: db_gain.into(),
                    },
                )))
            }
            EffectSettings::FilterLowShelf12db { cutoff, db_gain } => {
                Entity::BiQuadFilter(Box::new(BiQuadFilter::new_with(
                    sample_rate,
                    &NanoBiQuadFilter {
                        cutoff,
                        q: db_gain.into(),
                    },
                )))
            }
            EffectSettings::FilterHighShelf12db { cutoff, db_gain } => {
                Entity::BiQuadFilter(Box::new(BiQuadFilter::new_with(
                    sample_rate,
                    &NanoBiQuadFilter {
                        cutoff,
                        q: db_gain.into(),
                    },
                )))
            }
            EffectSettings::Delay { seconds } => Entity::Delay(Box::new(Delay::new_with(
                sample_rate,
                NanoDelay { seconds },
            ))),
            EffectSettings::Reverb {
                wet_dry_mix,
                attenuation,
                seconds: reverb_seconds,
            } => Entity::Reverb(Box::new(Reverb::new_with(
                sample_rate,
                NanoReverb {
                    attenuation: attenuation.into(),
                    seconds: reverb_seconds.into(),
                    wet_dry_mix: wet_dry_mix.into(),
                },
            ))),
            EffectSettings::Chorus {
                wet_dry_mix,
                voices,
                delay_factor,
            } => Entity::Chorus(Box::new(Chorus::new_with(
                sample_rate,
                NanoChorus {
                    voices,
                    delay_factor,
                    wet_dry_mix,
                },
            ))),
        }
    }
}
