// Copyright (c) 2023 Mike Tsao. All rights reserved.

// TODO: how to make the ControlParams export automatic? Should it be?
pub use controllers::{MessageMaker, ToyController, ToyControllerControlParams};
pub use effects::{ToyEffect, ToyEffectControlParams};
pub use instruments::{
    ToyAudioSource, ToyAudioSourceControlParams, ToyInstrument, ToyInstrumentControlParams,
    ToySynth, ToySynthControlParams,
};

mod controllers;
mod effects;
mod instruments;
