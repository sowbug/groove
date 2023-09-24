// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! Fundamental structs and traits.

use eframe::egui::Slider;
use ensnare_core::{prelude::*, traits::prelude::*};
use ensnare_proc_macros::{Control, Params};
use serde::{Deserialize, Serialize};

/// This struct doesn't do anything. It exists only to let the doc system know
/// what the name of the project is.
pub struct Groove;

/// Handles automation, or real-time automatic control of one entity's
/// parameters by another entity's output.
pub mod control;
/// Knows about [MIDI](https://en.wikipedia.org/wiki/MIDI).
pub mod midi;
/// Handles digital-audio, wall-clock, and musical time.
pub mod time;
/// Contains various helper functions that keep different parts of the system
/// consistent.
pub mod util;

pub const SAMPLE_BUFFER_SIZE: usize = 64;

#[cfg(test)]
mod tests {
    use ensnare_core::modulators::{Dca, DcaParams};

    use super::*;
}
