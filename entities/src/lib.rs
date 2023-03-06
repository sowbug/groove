// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! The suite of instruments, effects, and controllers supplied with Groove.

pub use messages::EntityMessage;

pub mod controllers;
pub mod effects;
pub mod instruments;
mod messages;

use groove_core::ParameterType;
use std::{env::current_dir, path::PathBuf};

// TODO: these should be #[cfg(test)] because nobody should be assuming these
// values
const DEFAULT_SAMPLE_RATE: usize = 44100;
const DEFAULT_BPM: ParameterType = 128.0;
const DEFAULT_TIME_SIGNATURE: (usize, usize) = (4, 4);
const DEFAULT_MIDI_TICKS_PER_SECOND: usize = 960;

const ASSETS: &str = "assets";

// These functions are adapted from groove::utils::Paths. I didn't want to
// move that whole thing because I'd like most crates to remain I/O
// agnostic. TODO TODO TODO
fn cwd() -> PathBuf {
    PathBuf::from(
        current_dir()
            .ok()
            .map(PathBuf::into_os_string)
            .and_then(|exe| exe.into_string().ok())
            .unwrap(),
    )
}

pub(crate) fn asset_path() -> PathBuf {
    let mut path_buf = cwd();
    path_buf.push(ASSETS);
    path_buf
}

#[cfg(test)]
mod tests {
    use crate::cwd;
    use std::path::PathBuf;

    pub(crate) fn test_data_path() -> PathBuf {
        const TEST_DATA: &str = "test-data";
        let mut path_buf = cwd();
        path_buf.push(TEST_DATA);
        path_buf
    }
}
