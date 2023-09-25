// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! The suite of instruments, effects, and controllers supplied with Groove.

#[cfg(test)]
mod tests {
    use ensnare_core::core::ParameterType;

    pub(crate) const DEFAULT_SAMPLE_RATE: usize = 44100;
    pub(crate) const DEFAULT_BPM: ParameterType = 128.0;
    pub(crate) const DEFAULT_MIDI_TICKS_PER_SECOND: usize = 960;
}
