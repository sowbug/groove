use crate::{
    common::{MonoSample, Rrc},
    effects::{
        bitcrusher::Bitcrusher,
        filter::{BiQuadFilter, FilterParams},
        gain::Gain,
        limiter::Limiter,
        mixer::Mixer,
    },
    traits::IsEffect,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EffectSettings {
    Mixer {},
    Gain {
        ceiling: f32,
    },
    Limiter {
        min: f32,
        max: f32,
    },
    #[serde(rename_all = "kebab-case")]
    Bitcrusher {
        bits_to_crush: u8,
    },
    #[serde(rename = "filter-low-pass-12db")]
    FilterLowPass12db {
        cutoff: f32,
        q: f32,
    },
    #[serde(rename = "filter-high-pass-12db")]
    FilterHighPass12db {
        cutoff: f32,
        q: f32,
    },
    #[serde(rename = "filter-band-pass-12db")]
    FilterBandPass12db {
        cutoff: f32,
        bandwidth: f32,
    },
    #[serde(rename = "filter-band-stop-12db")]
    FilterBandStop12db {
        cutoff: f32,
        bandwidth: f32,
    },
    #[serde(rename = "filter-all-pass-12db")]
    FilterAllPass12db {
        cutoff: f32,
        q: f32,
    },
    #[serde(rename = "filter-peaking-eq-12db")]
    #[serde(rename_all = "kebab-case")]
    FilterPeakingEq12db {
        cutoff: f32,
        db_gain: f32,
    },
    #[serde(rename = "filter-low-shelf-12db")]
    #[serde(rename_all = "kebab-case")]
    FilterLowShelf12db {
        cutoff: f32,
        db_gain: f32,
    },
    #[serde(rename = "filter-high-shelf-12db")]
    #[serde(rename_all = "kebab-case")]
    FilterHighShelf12db {
        cutoff: f32,
        db_gain: f32,
    },
}

impl EffectSettings {
    pub(crate) fn instantiate(&self, sample_rate: usize) -> Rrc<dyn IsEffect> {
        match *self {
            EffectSettings::Mixer {} => Mixer::new_wrapped(),
            EffectSettings::Limiter { min, max } => {
                Limiter::new_wrapped_with(min as MonoSample, max as MonoSample)
            }
            EffectSettings::Gain { ceiling } => Gain::new_wrapped_with(ceiling),
            EffectSettings::Bitcrusher { bits_to_crush } => {
                Bitcrusher::new_wrapped_with(bits_to_crush)
            }
            EffectSettings::FilterLowPass12db { cutoff, q } => {
                BiQuadFilter::new_wrapped_with(&FilterParams::LowPass { cutoff, q }, sample_rate)
            }
            EffectSettings::FilterHighPass12db { cutoff, q } => {
                BiQuadFilter::new_wrapped_with(&FilterParams::HighPass { cutoff, q }, sample_rate)
            }
            EffectSettings::FilterBandPass12db { cutoff, bandwidth } => {
                BiQuadFilter::new_wrapped_with(
                    &FilterParams::BandPass { cutoff, bandwidth },
                    sample_rate,
                )
            }
            EffectSettings::FilterBandStop12db { cutoff, bandwidth } => {
                BiQuadFilter::new_wrapped_with(
                    &FilterParams::BandStop { cutoff, bandwidth },
                    sample_rate,
                )
            }
            EffectSettings::FilterAllPass12db { cutoff, q } => {
                BiQuadFilter::new_wrapped_with(&FilterParams::AllPass { cutoff, q }, sample_rate)
            }
            EffectSettings::FilterPeakingEq12db { cutoff, db_gain } => {
                BiQuadFilter::new_wrapped_with(
                    &FilterParams::PeakingEq { cutoff, db_gain },
                    sample_rate,
                )
            }
            EffectSettings::FilterLowShelf12db { cutoff, db_gain } => {
                BiQuadFilter::new_wrapped_with(
                    &FilterParams::LowShelf { cutoff, db_gain },
                    sample_rate,
                )
            }
            EffectSettings::FilterHighShelf12db { cutoff, db_gain } => {
                BiQuadFilter::new_wrapped_with(
                    &FilterParams::HighShelf { cutoff, db_gain },
                    sample_rate,
                )
            }
        }
    }
}
