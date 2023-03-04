// Copyright (c) 2023 Mike Tsao. All rights reserved.

pub use effects::ToyEffect;
pub use effects::ToyEffectControlParams; // TODO: how to make this automatic? Should it be?
pub use instruments::ToyInstrument;
pub use instruments::ToyInstrumentControlParams;

mod controllers;
mod effects;
mod instruments;
