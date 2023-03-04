pub use bitcrusher::Bitcrusher;
pub use chorus::Chorus;
pub use compressor::Compressor;
pub use delay::Delay;
pub use filter::BiQuadFilter;
pub use filter::FilterParams;
pub use gain::Gain;
pub use limiter::Limiter;
pub use mixer::Mixer;
pub use reverb::Reverb;

use groove_core::{
    control::F32ControlValue,
    traits::{Controllable, HasUid, IsEffect, TransformsAudio},
    Sample,
};
use groove_macros::Control;
use std::fmt::Debug;
use std::str::FromStr;
use strum_macros::{Display, EnumString, FromRepr};

pub(crate) mod bitcrusher;
pub(crate) mod chorus;
pub(crate) mod compressor;
pub(crate) mod delay;
pub(crate) mod filter;
pub(crate) mod gain;
pub(crate) mod limiter;
pub(crate) mod mixer;
pub(crate) mod reverb;

#[derive(Control, Debug, Default)]
pub struct TestMixer {
    uid: usize,
}
impl IsEffect for TestMixer {}
impl HasUid for TestMixer {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}
impl TransformsAudio for TestMixer {
    fn transform_channel(&mut self, _channel: usize, input_sample: Sample) -> Sample {
        input_sample
    }
}
