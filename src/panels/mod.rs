// Copyright (c) 2023 Mike Tsao. All rights reserved.

pub use audio_panel::{AudioPanel, AudioPanelEvent, NeedsAudioFn};
pub use control_panel::{ControlBar, ControlPanel, ControlPanelAction};
pub use legacy::audio_panel::AudioPanel as OldAudioPanel;
pub use legacy::preferences::Preferences;
pub use legacy::thing_browser::{EntityBrowser, EntityBrowserEvent, EntityBrowserNode};
pub use midi_panel::{MidiPanel, MidiPanelEvent};
pub use orchestrator_panel::{OrchestratorEvent, OrchestratorInput, OrchestratorPanel};
pub use palette_panel::{PaletteAction, PalettePanel};

mod audio_panel;
mod control_panel;
mod legacy;
mod midi_panel;
mod orchestrator_panel;
mod palette_panel;
