#![feature(trait_upcasting)]
#![allow(incomplete_features)]

pub mod devices;
pub mod helpers;

pub(crate) mod common;
pub(crate) mod effects;
pub(crate) mod preset;
pub(crate) mod primitives;
pub(crate) mod settings;
pub(crate) mod synthesizers;
pub(crate) mod traits;

 // TODO: nobody uses this, because we still declare it to avoid bit rot
 // while refactoring.
pub(crate) mod scripting;
