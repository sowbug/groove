use eframe::egui::Ui;
use groove_core::traits::gui::Shows;

use super::{AudioPanel, MidiPanel, NeedsAudioFn};

/// [SettingsPanel] displays preferences/settings.
#[derive(Debug)]
pub struct SettingsPanel {
    audio_panel: AudioPanel,
    midi_panel: MidiPanel,

    is_open: bool,
}
impl SettingsPanel {
    /// Creates a new [SettingsPanel].
    pub fn new_with(needs_audio_fn: NeedsAudioFn) -> Self {
        Self {
            audio_panel: AudioPanel::new_with(needs_audio_fn),
            midi_panel: Default::default(),
            is_open: Default::default(),
        }
    }

    /// Whether the panel is currently visible.
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// Toggle visibility.
    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
    }

    /// The owned [AudioPanel].
    pub fn audio_panel(&self) -> &AudioPanel {
        &self.audio_panel
    }

    /// The owned [MidiPanel].
    pub fn midi_panel(&self) -> &MidiPanel {
        &self.midi_panel
    }

    /// Asks the panel to shut down any services associated with contained panels.
    pub fn exit(&self) {
        self.audio_panel.exit();
        self.midi_panel.exit();
    }
}
impl Shows for SettingsPanel {
    fn show(&mut self, ui: &mut Ui) {
        ui.label("Audio");
        self.audio_panel.show(ui);
        ui.label("MIDI");
        self.midi_panel.show(ui);
    }
}
