// Copyright (c) 2023 Mike Tsao. All rights reserved.

/// Contains widgets related to automation/control.
pub mod control;

/// Contains widgets that support Controller views.
pub mod controllers;

/// Various widgets used throughout the system.
pub mod core;

/// Contains widgets related to [Pattern](crate::mini::piano_roll::Pattern)s and
/// [PianoRoll](crate::mini::piano_roll::PianoRoll).
pub mod pattern;

/// Contains widgets that are useful as placeholders during development.
pub mod placeholder;

/// Contains widgets that help draw timeline views.
pub mod timeline;

/// Contains widgets that help draw tracks.
pub mod track;

/// A range that's useful for arranging MIDI notes along an egui axis. Note that
/// this is in reverse order, because vertically-oriented piano rolls show the
/// highest notes at the top of the screen.
pub const MIDI_NOTE_F32_RANGE: std::ops::RangeInclusive<f32> =
    groove_core::midi::MidiNote::MAX as u8 as f32..=groove_core::midi::MidiNote::MIN as u8 as f32;

/// A range that covers all MIDI note values in ascending order.
pub const MIDI_NOTE_U8_RANGE: std::ops::RangeInclusive<u8> =
    groove_core::midi::MidiNote::MIN as u8..=groove_core::midi::MidiNote::MAX as u8;
