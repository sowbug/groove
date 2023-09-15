use eframe::egui::Ui;
use groove_core::traits::gui::Displays;

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
    pub fn new_with(midi_panel: MidiPanel, needs_audio_fn: NeedsAudioFn) -> Self {
        Self {
            audio_panel: AudioPanel::new_with(needs_audio_fn),
            midi_panel,
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

    /// The owned [MidiPanel] (mutable).
    pub fn midi_panel_mut(&mut self) -> &mut MidiPanel {
        &mut self.midi_panel
    }

    /// Asks the panel to shut down any services associated with contained panels.
    pub fn exit(&self) {
        self.audio_panel.exit();
        self.midi_panel.exit();
    }
}
impl Displays for SettingsPanel {
    fn ui(&mut self, ui: &mut Ui) -> eframe::egui::Response {
        let response =
            ui.label("Audio") | self.audio_panel.ui(ui) | ui.label("MIDI") | self.midi_panel.ui(ui);

        {
            let mut debug_on_hover = ui.ctx().debug_on_hover();
            ui.checkbox(&mut debug_on_hover, "🐛 Debug on hover")
                .on_hover_text("Show structure of the ui when you hover with the mouse");
            ui.ctx().set_debug_on_hover(debug_on_hover);
        }
        response
    }
}
