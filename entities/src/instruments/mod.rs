// Copyright (c) 2023 Mike Tsao. All rights reserved.

#[cfg(feature = "iced-framework")]
pub use drumkit::DrumkitMessage;
pub use drumkit::{Drumkit, DrumkitParams};
#[cfg(feature = "iced-framework")]
pub use fm::FmSynthMessage;
pub use fm::{FmSynth, FmSynthParams, FmVoice};
#[cfg(feature = "iced-framework")]
pub use metronome::MetronomeMessage;
pub use metronome::{Metronome, MetronomeParams};
#[cfg(feature = "iced-framework")]
pub use sampler::SamplerMessage;
pub use sampler::{Sampler, SamplerParams};
#[cfg(feature = "iced-framework")]
pub use welsh::WelshSynthMessage;
pub use welsh::{LfoRouting, WelshSynth, WelshSynthParams, WelshVoice};

mod drumkit;
mod fm;
mod metronome;
mod sampler;
mod welsh;
