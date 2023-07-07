// Copyright (c) 2023 Mike Tsao. All rights reserved.

pub use arrangement_view::ArrangementView;
pub use audio_panel::{AudioPanel, AudioPanelEvent, MiniAudioPanel, NeedsAudioFn};
pub use control_panel::{ControlBar, ControlPanel, ControlPanelAction};
pub use legacy::preferences::Preferences;
pub use legacy::thing_browser::{ThingBrowser, ThingBrowserEvent, ThingBrowserNode};
pub use midi_panel::{MidiPanel, MidiPanelEvent};
pub use orchestrator_panel::{MiniOrchestratorEvent, MiniOrchestratorInput, OrchestratorPanel};
pub use palette_panel::{PaletteAction, PalettePanel};
pub use settings_panel::SettingsPanel;

mod arrangement_view;
mod audio_panel;
mod control_panel;
mod legacy;
mod midi_panel;
mod orchestrator_panel;
mod palette_panel;
mod settings_panel;
