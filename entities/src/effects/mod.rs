// Copyright (c) 2023 Mike Tsao. All rights reserved.

pub use bitcrusher::{Bitcrusher, BitcrusherParams};
pub use chorus::{Chorus, ChorusParams};
pub use compressor::{Compressor, CompressorParams};
pub use delay::{Delay, DelayParams};
pub use filter::{
    BiQuadFilter, BiQuadFilterAllPass, BiQuadFilterAllPassParams, BiQuadFilterBandPass,
    BiQuadFilterBandPassParams, BiQuadFilterBandStop, BiQuadFilterBandStopParams,
    BiQuadFilterHighPass, BiQuadFilterHighPassParams, BiQuadFilterHighShelf,
    BiQuadFilterHighShelfParams, BiQuadFilterLowPass12db, BiQuadFilterLowPass12dbParams,
    BiQuadFilterLowPass24db, BiQuadFilterLowPass24dbParams, BiQuadFilterLowShelf,
    BiQuadFilterLowShelfParams, BiQuadFilterNone, BiQuadFilterNoneParams, BiQuadFilterPeakingEq,
    BiQuadFilterPeakingEqParams,
};
pub use gain::{Gain, GainParams};
pub use limiter::{Limiter, LimiterParams};
pub use mixer::{Mixer, MixerParams};
pub use reverb::{Reverb, ReverbParams};

#[cfg(feature = "iced-framework")]
pub use bitcrusher::BitcrusherMessage;
#[cfg(feature = "iced-framework")]
pub use chorus::ChorusMessage;
#[cfg(feature = "iced-framework")]
pub use compressor::CompressorMessage;
#[cfg(feature = "iced-framework")]
pub use delay::DelayMessage;
#[cfg(feature = "iced-framework")]
pub use filter::{
    BiQuadFilterAllPassMessage, BiQuadFilterBandPassMessage, BiQuadFilterBandStopMessage,
    BiQuadFilterHighPassMessage, BiQuadFilterHighShelfMessage, BiQuadFilterLowPass12dbMessage,
    BiQuadFilterLowPass24dbMessage, BiQuadFilterLowShelfMessage, BiQuadFilterNoneMessage,
    BiQuadFilterPeakingEqMessage,
};
#[cfg(feature = "iced-framework")]
pub use gain::{Gain, GainMessage, GainParams};
#[cfg(feature = "iced-framework")]
pub use limiter::{Limiter, LimiterMessage, LimiterParams};
#[cfg(feature = "iced-framework")]
pub use mixer::{Mixer, MixerMessage, MixerParams};
#[cfg(feature = "iced-framework")]
pub use reverb::{Reverb, ReverbMessage, ReverbParams};

pub(crate) mod bitcrusher;
pub(crate) mod chorus;
pub(crate) mod compressor;
pub(crate) mod delay;
pub(crate) mod filter;
pub(crate) mod gain;
pub(crate) mod limiter;
pub(crate) mod mixer;
pub(crate) mod reverb;
