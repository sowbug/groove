use crate::{
    common::{MonoSample, Rrc},
    effects::{
        bitcrusher::Bitcrusher,
        filter::{Filter, FilterType},
        gain::Gain,
        limiter::Limiter,
    },
    traits::IsEffect,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EffectSettings {
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
            // This has more repetition than we'd expect because of
            // https://stackoverflow.com/questions/26378842/how-do-i-overcome-match-arms-with-incompatible-types-for-structs-implementing-sa
            //
            // Match arms have to return the same types, and returning a Rc<RefCell<dyn some trait>> doesn't count
            // as the same type.
            EffectSettings::Limiter { min, max } => {
                Limiter::new_wrapped_with(min as MonoSample, max as MonoSample)
            }
            EffectSettings::Gain { ceiling } => Gain::new_wrapped_with(ceiling),
            EffectSettings::Bitcrusher { bits_to_crush } => {
                Bitcrusher::new_wrapped_with(bits_to_crush)
            }
            EffectSettings::FilterLowPass12db { cutoff, q } => {
                Filter::new_wrapped_with(&FilterType::LowPass {
                    sample_rate,
                    cutoff,
                    q,
                })
            }
            EffectSettings::FilterHighPass12db { cutoff, q } => {
                Filter::new_wrapped_with(&FilterType::HighPass {
                    sample_rate,
                    cutoff,
                    q,
                })
            }
            EffectSettings::FilterBandPass12db { cutoff, bandwidth } => {
                Filter::new_wrapped_with(&FilterType::BandPass {
                    sample_rate,
                    cutoff,
                    bandwidth,
                })
            }
            EffectSettings::FilterBandStop12db { cutoff, bandwidth } => {
                Filter::new_wrapped_with(&FilterType::BandStop {
                    sample_rate,
                    cutoff,
                    bandwidth,
                })
            }
            EffectSettings::FilterAllPass12db { cutoff, q } => {
                Filter::new_wrapped_with(&FilterType::AllPass {
                    sample_rate,
                    cutoff,
                    q,
                })
            }
            EffectSettings::FilterPeakingEq12db { cutoff, db_gain } => {
                Filter::new_wrapped_with(&FilterType::PeakingEq {
                    sample_rate,
                    cutoff,
                    db_gain,
                })
            }
            EffectSettings::FilterLowShelf12db { cutoff, db_gain } => {
                Filter::new_wrapped_with(&FilterType::LowShelf {
                    sample_rate,
                    cutoff,
                    db_gain,
                })
            }
            EffectSettings::FilterHighShelf12db { cutoff, db_gain } => {
                Filter::new_wrapped_with(&FilterType::HighShelf {
                    sample_rate,
                    cutoff,
                    db_gain,
                })
            }
        }
    }
}
