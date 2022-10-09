use std::{cell::RefCell, rc::Rc};

use serde::{Deserialize, Serialize};

use crate::{
    common::MonoSample,
    effects::{
        bitcrusher::Bitcrusher,
        filter::{Filter, FilterType},
        gain::Gain,
        limiter::Limiter,
    },
    traits::IsEffect,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EffectSettings {
    Gain {
        amount: f32,
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
    pub(crate) fn instantiate(&self, sample_rate: usize) -> Rc<RefCell<dyn IsEffect>> {
        let effect: Rc<RefCell<dyn IsEffect>> = match *self {
            // This has more repetition than we'd expect because of
            // https://stackoverflow.com/questions/26378842/how-do-i-overcome-match-arms-with-incompatible-types-for-structs-implementing-sa
            //
            // Match arms have to return the same types, and returning a Rc<RefCell<dyn some trait>> doesn't count
            // as the same type.
            EffectSettings::Limiter { min, max } => Rc::new(RefCell::new(Limiter::new_with(
                min as MonoSample,
                max as MonoSample,
            ))),
            EffectSettings::Gain { amount } => Rc::new(RefCell::new(Gain::new_with(amount))),
            EffectSettings::Bitcrusher { bits_to_crush } => {
                Rc::new(RefCell::new(Bitcrusher::new_with(bits_to_crush)))
            }
            EffectSettings::FilterLowPass12db { cutoff, q } => {
                Rc::new(RefCell::new(Filter::new(&FilterType::LowPass {
                    sample_rate,
                    cutoff,
                    q,
                })))
            }
            EffectSettings::FilterHighPass12db { cutoff, q } => {
                Rc::new(RefCell::new(Filter::new(&FilterType::HighPass {
                    sample_rate,
                    cutoff,
                    q,
                })))
            }
            EffectSettings::FilterBandPass12db { cutoff, bandwidth } => {
                Rc::new(RefCell::new(Filter::new(&FilterType::BandPass {
                    sample_rate,
                    cutoff,
                    bandwidth,
                })))
            }
            EffectSettings::FilterBandStop12db { cutoff, bandwidth } => {
                Rc::new(RefCell::new(Filter::new(&FilterType::BandStop {
                    sample_rate,
                    cutoff,
                    bandwidth,
                })))
            }
            EffectSettings::FilterAllPass12db { cutoff, q } => {
                Rc::new(RefCell::new(Filter::new(&FilterType::AllPass {
                    sample_rate,
                    cutoff,
                    q,
                })))
            }
            EffectSettings::FilterPeakingEq12db { cutoff, db_gain } => {
                Rc::new(RefCell::new(Filter::new(&FilterType::PeakingEq {
                    sample_rate,
                    cutoff,
                    db_gain,
                })))
            }
            EffectSettings::FilterLowShelf12db { cutoff, db_gain } => {
                Rc::new(RefCell::new(Filter::new(&FilterType::LowShelf {
                    sample_rate,
                    cutoff,
                    db_gain,
                })))
            }
            EffectSettings::FilterHighShelf12db { cutoff, db_gain } => {
                Rc::new(RefCell::new(Filter::new(&FilterType::HighShelf {
                    sample_rate,
                    cutoff,
                    db_gain,
                })))
            }
        };
        effect
    }
}
