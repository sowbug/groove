use crate::{
    common::MonoSample,
    effects::{
        bitcrusher::Bitcrusher,
        chorus::Chorus,
        delay::Delay,
        filter::{BiQuadFilter, FilterParams},
        gain::Gain,
        limiter::Limiter,
        mixer::Mixer,
        reverb::Reverb,
    },
    entities::BoxedEntity,
    messages::EntityMessage,
    traits::TestEffect,
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
    Gain { ceiling: f32 },
    #[serde(rename_all = "kebab-case")]
    Limiter { min: f32, max: f32 },
    #[serde(rename_all = "kebab-case")]
    Bitcrusher { bits_to_crush: u8 },
    #[serde(rename_all = "kebab-case")]
    Chorus {
        wet_dry_mix: f32,
        voices: usize,
        delay_factor: usize,
    },
    #[serde(rename_all = "kebab-case")]
    Delay { delay: f32 },
    #[serde(rename_all = "kebab-case")]
    Reverb {
        wet_dry_mix: f32,
        attenuation: f32,
        reverb_seconds: f32,
    },
    #[serde(rename = "filter-low-pass-12db")]
    FilterLowPass12db { cutoff: f32, q: f32 },
    #[serde(rename = "filter-low-pass-24db", rename_all = "kebab-case")]
    FilterLowPass24db { cutoff: f32, passband_ripple: f32 },
    #[serde(rename = "filter-high-pass-12db")]
    FilterHighPass12db { cutoff: f32, q: f32 },
    #[serde(rename = "filter-band-pass-12db")]
    FilterBandPass12db { cutoff: f32, bandwidth: f32 },
    #[serde(rename = "filter-band-stop-12db")]
    FilterBandStop12db { cutoff: f32, bandwidth: f32 },
    #[serde(rename = "filter-all-pass-12db")]
    FilterAllPass12db { cutoff: f32, q: f32 },
    #[serde(rename = "filter-peaking-eq-12db")]
    #[serde(rename_all = "kebab-case")]
    FilterPeakingEq12db { cutoff: f32, db_gain: f32 },
    #[serde(rename = "filter-low-shelf-12db")]
    #[serde(rename_all = "kebab-case")]
    FilterLowShelf12db { cutoff: f32, db_gain: f32 },
    #[serde(rename = "filter-high-shelf-12db")]
    #[serde(rename_all = "kebab-case")]
    FilterHighShelf12db { cutoff: f32, db_gain: f32 },
}

impl EffectSettings {
    pub(crate) fn instantiate(
        &self,
        sample_rate: usize,
        load_only_test_entities: bool,
    ) -> BoxedEntity {
        if load_only_test_entities {
            return BoxedEntity::TestEffect(Box::new(TestEffect::<EntityMessage>::default()));
        }
        match *self {
            EffectSettings::Test {} => {
                BoxedEntity::TestEffect(Box::new(TestEffect::<EntityMessage>::default()))
            }
            EffectSettings::Mixer {} => {
                BoxedEntity::Mixer(Box::new(Mixer::<EntityMessage>::default()))
            }
            EffectSettings::Limiter { min, max } => BoxedEntity::Limiter(Box::new(
                Limiter::new_with(min as MonoSample, max as MonoSample),
            )),
            EffectSettings::Gain { ceiling } => {
                BoxedEntity::Gain(Box::new(Gain::<EntityMessage>::new_with(ceiling)))
            }
            EffectSettings::Bitcrusher { bits_to_crush } => {
                BoxedEntity::Bitcrusher(Box::new(Bitcrusher::new_with(bits_to_crush)))
            }
            EffectSettings::FilterLowPass12db { cutoff, q } => BoxedEntity::BiQuadFilter(Box::new(
                BiQuadFilter::new_with(&FilterParams::LowPass12db { cutoff, q }, sample_rate),
            )),
            EffectSettings::FilterLowPass24db {
                cutoff,
                passband_ripple,
            } => BoxedEntity::BiQuadFilter(Box::new(BiQuadFilter::new_with(
                &FilterParams::LowPass24db {
                    cutoff,
                    passband_ripple,
                },
                sample_rate,
            ))),

            EffectSettings::FilterHighPass12db { cutoff, q } => {
                BoxedEntity::BiQuadFilter(Box::new(BiQuadFilter::new_with(
                    &FilterParams::HighPass { cutoff, q },
                    sample_rate,
                )))
            }
            EffectSettings::FilterBandPass12db { cutoff, bandwidth } => {
                BoxedEntity::BiQuadFilter(Box::new(BiQuadFilter::new_with(
                    &FilterParams::BandPass { cutoff, bandwidth },
                    sample_rate,
                )))
            }
            EffectSettings::FilterBandStop12db { cutoff, bandwidth } => {
                BoxedEntity::BiQuadFilter(Box::new(BiQuadFilter::new_with(
                    &FilterParams::BandStop { cutoff, bandwidth },
                    sample_rate,
                )))
            }
            EffectSettings::FilterAllPass12db { cutoff, q } => BoxedEntity::BiQuadFilter(Box::new(
                BiQuadFilter::new_with(&FilterParams::AllPass { cutoff, q }, sample_rate),
            )),
            EffectSettings::FilterPeakingEq12db { cutoff, db_gain } => {
                BoxedEntity::BiQuadFilter(Box::new(BiQuadFilter::new_with(
                    &FilterParams::PeakingEq { cutoff, db_gain },
                    sample_rate,
                )))
            }
            EffectSettings::FilterLowShelf12db { cutoff, db_gain } => {
                BoxedEntity::BiQuadFilter(Box::new(BiQuadFilter::new_with(
                    &FilterParams::LowShelf { cutoff, db_gain },
                    sample_rate,
                )))
            }
            EffectSettings::FilterHighShelf12db { cutoff, db_gain } => {
                BoxedEntity::BiQuadFilter(Box::new(BiQuadFilter::new_with(
                    &FilterParams::HighShelf { cutoff, db_gain },
                    sample_rate,
                )))
            }
            EffectSettings::Delay { delay } => {
                BoxedEntity::Delay(Box::new(Delay::new_with(sample_rate, delay)))
            }
            EffectSettings::Reverb {
                wet_dry_mix,
                attenuation,
                reverb_seconds,
            } => BoxedEntity::Reverb(Box::new(Reverb::new_with(
                sample_rate,
                wet_dry_mix,
                attenuation,
                reverb_seconds,
            ))),
            EffectSettings::Chorus {
                wet_dry_mix,
                voices,
                delay_factor,
            } => BoxedEntity::Chorus(Box::new(Chorus::new_with(
                sample_rate,
                wet_dry_mix,
                voices,
                delay_factor,
            ))),
        }
    }
}
