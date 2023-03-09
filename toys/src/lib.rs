// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! `groove-toys` contains "toy" entities that are useful for development. They
//! implement Groove traits, usually in a simple fashion. They aren't likely to
//! be useful in real music prduction.

// TODO: how to make the ControlParams export automatic? Should it be?
pub use controllers::{MessageMaker, ToyController, ToyControllerControlParams};
pub use effects::ToyEffect;
pub use effects::ToyEffectControlParams;
pub use instruments::{
    ToyAudioSource, ToyAudioSourceControlParams, ToyInstrument, ToyInstrumentControlParams,
    ToySynth, ToySynthControlParams,
};

mod controllers;
mod effects;
mod instruments;

// NOTE: The Test... entities are in the non-tests module because they're
// sometimes useful as simple real entities to substitute in for production
// ones, for example if we're trying to determine whether an entity is
// responsible for a performance issue.

// TODO: redesign this for clockless operation
// pub trait TestsValues {
//     fn check_values(&mut self, clock: &Clock) {
//         // If we've been asked to assert values at checkpoints, do so.
//         if self.has_checkpoint_values()
//             && clock.time_for(self.time_unit()) >= self.checkpoint_time()
//         {
//             const SAD_FLOAT_DIFF: f32 = 1.0e-4;
//             if let Some(value) = self.pop_checkpoint_value() {
//                 assert_approx_eq!(self.value_to_check(), value, SAD_FLOAT_DIFF);
//             }
//             self.advance_checkpoint_time();
//         }
//     }

//     fn has_checkpoint_values(&self) -> bool;
//     fn time_unit(&self) -> &ClockTimeUnit;
//     fn checkpoint_time(&self) -> f32;
//     fn advance_checkpoint_time(&mut self);
//     fn value_to_check(&self) -> f32;
//     fn pop_checkpoint_value(&mut self) -> Option<f32>;
// }
