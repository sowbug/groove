#![feature(trait_upcasting)]
#![allow(incomplete_features)]

#[macro_use]
extern crate num_derive;
extern crate anyhow;

pub mod common;
pub mod devices;
pub mod general_midi;
pub mod preset;
pub mod primitives;
pub mod scripting;
pub mod settings;
pub mod synthesizers;
