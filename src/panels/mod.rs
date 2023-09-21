// Copyright (c) 2023 Mike Tsao. All rights reserved.

pub use audio_panel::{audio_settings, AudioPanel, AudioPanelEvent, AudioSettings, NeedsAudioFn};
pub use control_panel::{ControlPanel, ControlPanelAction};
#[cfg(obsolete)]
pub use legacy::{
    preferences::Preferences,
    thing_browser::{EntityBrowser, EntityBrowserEvent, EntityBrowserNode},
};
pub use midi_panel::{midi_settings, MidiPanel, MidiPanelEvent, MidiSettings};
pub use orchestrator_panel::{OrchestratorEvent, OrchestratorInput, OrchestratorPanel};
pub use palette_panel::{PaletteAction, PalettePanel};

mod audio_panel;
mod control_panel;
#[cfg(obsolete)]
mod legacy;
mod midi_panel;
mod orchestrator_panel;
mod palette_panel;
