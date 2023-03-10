// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    generators::{Envelope, EnvelopeParams},
    Normal, ParameterType,
};
use serde::{Deserialize, Serialize};

// attack/decay/release are in time units.
// sustain is a 0..=1 percentage.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct EnvelopeSettings {
    pub attack: ParameterType,
    pub decay: ParameterType,
    pub sustain: ParameterType, // TODO: this should be a Normal
    pub release: ParameterType,
}
impl Default for EnvelopeSettings {
    fn default() -> Self {
        Self {
            attack: 0.0,
            decay: 0.0,
            sustain: 1.0,
            release: 0.0,
        }
    }
}
impl EnvelopeSettings {
    #[allow(dead_code)]
    pub const MAX: f64 = 10000.0; // TODO: what exactly does Welsh mean by "max"?

    pub fn into_params(&self) -> EnvelopeParams {
        EnvelopeParams::new_with(
            self.attack,
            self.decay,
            Normal::new(self.sustain),
            self.release,
        )
    }

    pub fn into_envelope(&self, sample_rate: usize) -> Envelope {
        Envelope::new_with(sample_rate, self.into_params())
    }
}
