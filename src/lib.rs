#![feature(trait_upcasting)]
#![allow(incomplete_features)]

pub use crate::helpers::IOHelper;
pub use crate::orchestrator::Orchestrator;

pub(crate) mod common;
pub(crate) mod control;
pub(crate) mod effects;
pub(crate) mod helpers;
pub(crate) mod midi;
pub(crate) mod orchestrator;
pub(crate) mod patterns;
pub(crate) mod preset;
pub(crate) mod primitives;
pub(crate) mod settings;
pub(crate) mod synthesizers;
pub(crate) mod traits;

// TODO: nobody uses this, because we still declare it to avoid bit rot
// while refactoring.
pub(crate) mod scripting;
