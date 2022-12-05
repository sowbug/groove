use crate::{
    common::MonoSample,
    effects::{
        bitcrusher::Bitcrusher,
        delay::Delay,
        filter::{BiQuadFilter, FilterParams},
        gain::Gain,
        limiter::Limiter,
        mixer::Mixer,
    },
    messages::EntityMessage,
    traits::{IsEffect, TestEffect},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EffectSettings {
    #[serde(rename_all = "kebab-case")]
    Test {},
    #[serde(rename_all = "kebab-case")]
    Mixer {},
    #[serde(rename_all = "kebab-case")]
    Gain {
        ceiling: f32,
    },
    #[serde(rename_all = "kebab-case")]
    Limiter {
        min: f32,
        max: f32,
    },
    #[serde(rename_all = "kebab-case")]
    Bitcrusher {
        bits_to_crush: u8,
    },
    #[serde(rename_all = "kebab-case")]
    Delay {
        delay: f32,
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
    pub(crate) fn instantiate(
        &self,
        sample_rate: usize,
        load_only_test_entities: bool,
    ) -> Box<dyn IsEffect<Message = EntityMessage, ViewMessage = EntityMessage>> {
        if load_only_test_entities {
            return Box::new(TestEffect::<EntityMessage>::default());
        }
        match *self {
            EffectSettings::Test {} => Box::new(TestEffect::<EntityMessage>::default()),
            EffectSettings::Mixer {} => Box::new(Mixer::<EntityMessage>::default()),
            EffectSettings::Limiter { min, max } => {
                Box::new(Limiter::new_with(min as MonoSample, max as MonoSample))
            }
            EffectSettings::Gain { ceiling } => Box::new(Gain::<EntityMessage>::new_with(ceiling)),
            EffectSettings::Bitcrusher { bits_to_crush } => {
                Box::new(Bitcrusher::new_with(bits_to_crush))
            }
            EffectSettings::FilterLowPass12db { cutoff, q } => Box::new(BiQuadFilter::new_with(
                &FilterParams::LowPass { cutoff, q },
                sample_rate,
            )),
            EffectSettings::FilterHighPass12db { cutoff, q } => Box::new(BiQuadFilter::new_with(
                &FilterParams::HighPass { cutoff, q },
                sample_rate,
            )),
            EffectSettings::FilterBandPass12db { cutoff, bandwidth } => Box::new(
                BiQuadFilter::new_with(&FilterParams::BandPass { cutoff, bandwidth }, sample_rate),
            ),
            EffectSettings::FilterBandStop12db { cutoff, bandwidth } => Box::new(
                BiQuadFilter::new_with(&FilterParams::BandStop { cutoff, bandwidth }, sample_rate),
            ),
            EffectSettings::FilterAllPass12db { cutoff, q } => Box::new(BiQuadFilter::new_with(
                &FilterParams::AllPass { cutoff, q },
                sample_rate,
            )),
            EffectSettings::FilterPeakingEq12db { cutoff, db_gain } => Box::new(
                BiQuadFilter::new_with(&FilterParams::PeakingEq { cutoff, db_gain }, sample_rate),
            ),
            EffectSettings::FilterLowShelf12db { cutoff, db_gain } => Box::new(
                BiQuadFilter::new_with(&FilterParams::LowShelf { cutoff, db_gain }, sample_rate),
            ),
            EffectSettings::FilterHighShelf12db { cutoff, db_gain } => Box::new(
                BiQuadFilter::new_with(&FilterParams::HighShelf { cutoff, db_gain }, sample_rate),
            ),
            EffectSettings::Delay { delay } => Box::new(Delay::new_with(sample_rate, delay)),
        }
    }
}
