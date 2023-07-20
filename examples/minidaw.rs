// Copyright (c) 2023 Mike Tsao. All rights reserved.

#![deny(rustdoc::broken_intra_doc_links)]

use crossbeam_channel::Select;
use eframe::{
    egui::{
        self, warn_if_debug_build, Button, Context, Direction, FontData, FontDefinitions, Layout,
        ScrollArea, TextStyle, Ui,
    },
    emath::{Align, Align2},
    epaint::{FontFamily, FontId},
    CreationContext,
};
use egui_toast::{Toast, ToastOptions, Toasts};
use groove::{
    app_version,
    egui_widgets::{
        AudioPanelEvent, ControlPanel, ControlPanelAction, MidiPanelEvent, MiniOrchestratorEvent,
        MiniOrchestratorInput, NeedsAudioFn, OrchestratorPanel, PaletteAction, PalettePanel,
        SettingsPanel,
    },
    mini::{register_mini_factory_entities, DragDropManager, EntityFactory, Key, MiniOrchestrator},
};
use groove_core::{
    time::SampleRate,
    traits::{gui::Shows, Configurable},
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

// Rules for communication among app components
//
// - If it's in the same thread, don't be fancy. Example: the app owns the
//   control bar, and the control bar always runs in the UI thread. The app
//   should talk directly to the control bar (update transport), and the control
//   bar can pass back an enum saying what happened (play button was pressed).
// - If it's updated rarely but displayed frequently, the struct should push it
//   to the app, and the app should cache it. Example: BPM is displayed in the
//   control bar, so we're certain to need it on every redraw, but it rarely
//   changes (unless it's automated). Orchestrator should define a channel
//   message, and the app should handle it when it's received. (This is
//   currently a not-great example, because we're cloning [Transport] on each
//   cycle.)
// - If it's updated more often than the UI framerate, let the UI pull it
//   directly from the struct. Example: an LFO signal or a real-time spectrum
//   analysis. These should be APIs directly on the struct, and we'll leave it
//   up to the app to lock the struct and get what it needs.

#[derive(Clone, Debug)]
enum MenuBarAction {
    Quit,
    ProjectNew,
    ProjectOpen,
    ProjectSave,
    TrackNewMidi,
    TrackNewAudio,
    TrackNewSend,
    TrackDuplicate,
    TrackDelete,
    TrackRemoveSelectedPatterns,
    TrackAddThing(Key),
    ComingSoon,
}

#[derive(Debug)]
struct MenuBarItem {
    name: String,
    children: Option<Vec<MenuBarItem>>,
    action: Option<MenuBarAction>,
    enabled: bool,
}
impl MenuBarItem {
    fn node(name: &str, children: Vec<MenuBarItem>) -> Self {
        Self {
            name: name.to_string(),
            children: Some(children),
            action: None,
            enabled: true,
        }
    }
    fn leaf(name: &str, action: MenuBarAction, enabled: bool) -> Self {
        Self {
            name: name.to_string(),
            children: None,
            action: Some(action),
            enabled,
        }
    }
    fn show(&self, ui: &mut Ui) -> Option<MenuBarAction> {
        let mut action = None;
        if let Some(children) = self.children.as_ref() {
            ui.menu_button(&self.name, |ui| {
                for child in children.iter() {
                    if let Some(a) = child.show(ui) {
                        action = Some(a);
                    }
                }
            });
        } else if let Some(action_to_perform) = &self.action {
            if ui
                .add_enabled(self.enabled, Button::new(&self.name))
                .clicked()
            {
                ui.close_menu();
                action = Some(action_to_perform.clone());
            }
        }
        action
    }
}

#[derive(Debug)]
struct MenuBar {
    factory: Arc<EntityFactory>,
}
impl MenuBar {
    pub fn new_with(factory: Arc<EntityFactory>) -> Self {
        Self { factory }
    }

    fn show_with_action(&mut self, ui: &mut Ui, is_track_selected: bool) -> Option<MenuBarAction> {
        let mut action = None;

        // Menus should look like menus, not buttons
        ui.style_mut().visuals.button_frame = false;

        ui.horizontal(|ui| {
            let mut device_submenus = Vec::default();
            if is_track_selected {
                device_submenus.push(MenuBarItem::node("New", self.new_entity_menu()));
            }
            device_submenus.extend(vec![
                MenuBarItem::leaf("Shift Left", MenuBarAction::ComingSoon, true),
                MenuBarItem::leaf("Shift Right", MenuBarAction::ComingSoon, true),
                MenuBarItem::leaf("Move Up", MenuBarAction::ComingSoon, true),
                MenuBarItem::leaf("Move Down", MenuBarAction::ComingSoon, true),
            ]);
            let menus = vec![
                MenuBarItem::node(
                    "Project",
                    vec![
                        MenuBarItem::leaf("New", MenuBarAction::ProjectNew, true),
                        MenuBarItem::leaf("Open", MenuBarAction::ProjectOpen, true),
                        MenuBarItem::leaf("Save", MenuBarAction::ProjectSave, true),
                        MenuBarItem::leaf("Quit", MenuBarAction::Quit, true),
                    ],
                ),
                MenuBarItem::node(
                    "Track",
                    vec![
                        MenuBarItem::leaf("New MIDI", MenuBarAction::TrackNewMidi, true),
                        MenuBarItem::leaf("New Audio", MenuBarAction::TrackNewAudio, true),
                        MenuBarItem::leaf("New Send", MenuBarAction::TrackNewSend, true),
                        MenuBarItem::leaf(
                            "Duplicate",
                            MenuBarAction::TrackDuplicate,
                            is_track_selected,
                        ),
                        MenuBarItem::leaf("Delete", MenuBarAction::TrackDelete, is_track_selected),
                        MenuBarItem::leaf(
                            "Remove Selected Patterns",
                            MenuBarAction::TrackRemoveSelectedPatterns,
                            true,
                        ), // TODO: enable only if some patterns selected
                    ],
                ),
                MenuBarItem::node("Device", device_submenus),
                MenuBarItem::node(
                    "Control",
                    vec![
                        MenuBarItem::leaf("Connect", MenuBarAction::ComingSoon, true),
                        MenuBarItem::leaf("Disconnect", MenuBarAction::ComingSoon, true),
                    ],
                ),
            ];
            for item in menus.iter() {
                if let Some(a) = item.show(ui) {
                    action = Some(a);
                }
            }
        });
        action
    }

    fn new_entity_menu(&self) -> Vec<MenuBarItem> {
        vec![MenuBarItem::node(
            "Things",
            self.factory
                .keys()
                .iter()
                .map(|k| {
                    MenuBarItem::leaf(
                        &k.to_string(),
                        MenuBarAction::TrackAddThing(k.clone()),
                        true,
                    )
                })
                .collect(),
        )]
    }
}

struct MiniDaw {
    mini_orchestrator: Arc<Mutex<MiniOrchestrator>>,

    menu_bar: MenuBar,
    control_panel: ControlPanel,
    orchestrator_panel: OrchestratorPanel,
    palette_panel: PalettePanel,
    settings_panel: SettingsPanel,

    exit_requested: bool,
    drag_drop_manager: Arc<Mutex<DragDropManager>>,

    toasts: Toasts,
}
impl MiniDaw {
    pub const FONT_REGULAR: &str = "font-regular";
    pub const FONT_BOLD: &str = "font-bold";
    pub const FONT_MONO: &str = "font-mono";
    pub const APP_NAME: &str = "MiniDAW";
    pub const DEFAULT_PROJECT_NAME: &str = "Untitled";

    pub fn new(cc: &CreationContext) -> Self {
        Self::initialize_fonts(cc);
        Self::initialize_style(&cc.egui_ctx);

        let mut entity_factory = EntityFactory::default();
        register_mini_factory_entities(&mut entity_factory);
        let factory = Arc::new(entity_factory);

        let drag_drop_manager = Arc::new(Mutex::new(DragDropManager::default()));
        let orchestrator_panel = OrchestratorPanel::new_with(Arc::clone(&factory));
        let mini_orchestrator = Arc::clone(orchestrator_panel.orchestrator());

        let mini_orchestrator_for_fn = Arc::clone(&mini_orchestrator);
        let needs_audio: NeedsAudioFn = Box::new(move |audio_queue, samples_requested| {
            if let Ok(mut o) = mini_orchestrator_for_fn.lock() {
                o.render_and_enqueue(samples_requested, audio_queue);
            }
        });

        let mut r = Self {
            mini_orchestrator,
            menu_bar: MenuBar::new_with(Arc::clone(&factory)),
            control_panel: Default::default(),
            orchestrator_panel,
            palette_panel: PalettePanel::new_with(factory, Arc::clone(&drag_drop_manager)),
            settings_panel: SettingsPanel::new_with(Box::new(needs_audio)),

            exit_requested: Default::default(),
            drag_drop_manager,

            toasts: Toasts::new()
                .anchor(Align2::RIGHT_BOTTOM, (-10.0, -10.0))
                .direction(Direction::BottomUp),
        };
        r.spawn_channel_watcher(cc.egui_ctx.clone());
        r
    }

    fn initialize_fonts(cc: &CreationContext) {
        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            Self::FONT_REGULAR.to_owned(),
            FontData::from_static(include_bytes!("../res/fonts/jost/static/Jost-Regular.ttf")),
        );
        fonts.font_data.insert(
            Self::FONT_BOLD.to_owned(),
            FontData::from_static(include_bytes!("../res/fonts/jost/static/Jost-Bold.ttf")),
        );
        fonts.font_data.insert(
            Self::FONT_MONO.to_owned(),
            FontData::from_static(include_bytes!("../res/fonts/cousine/Cousine-Regular.ttf")),
        );
        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, Self::FONT_REGULAR.to_owned());
        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .insert(0, Self::FONT_MONO.to_owned());
        fonts
            .families
            .entry(FontFamily::Name(Self::FONT_BOLD.into()))
            .or_default()
            .insert(0, Self::FONT_BOLD.to_owned());

        cc.egui_ctx.set_fonts(fonts);
    }

    fn initialize_style(ctx: &Context) {
        let mut style = (*ctx.style()).clone();

        // Disabled this because it stops text from getting highlighted when the
        // user hovers over a widget.
        //
        // style.visuals.override_text_color = Some(Color32::LIGHT_GRAY);

        style.text_styles = [
            (
                TextStyle::Heading,
                FontId::new(16.0, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(16.0, FontFamily::Proportional)),
            (
                TextStyle::Monospace,
                FontId::new(14.0, FontFamily::Monospace),
            ),
            (
                TextStyle::Button,
                FontId::new(16.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Small,
                FontId::new(14.0, FontFamily::Proportional),
            ),
        ]
        .into();

        ctx.set_style(style);
    }

    fn handle_message_channels(&mut self) {
        // As long as any channel had a message in it, we'll keep handling them.
        // We don't expect a giant number of messages; otherwise we'd worry
        // about blocking the UI.
        loop {
            if !(self.handle_midi_panel_channel()
                || self.handle_audio_panel_channel()
                || self.handle_mini_orchestrator_channel())
            {
                break;
            }
        }
    }

    fn handle_midi_panel_channel(&mut self) -> bool {
        if let Ok(m) = self.settings_panel.midi_panel().receiver().try_recv() {
            match m {
                MidiPanelEvent::Midi(channel, message) => {
                    self.orchestrator_panel
                        .send_to_service(MiniOrchestratorInput::Midi(channel, message));
                }
                MidiPanelEvent::SelectInput(_) => {
                    // TODO: save selection in prefs
                }
                MidiPanelEvent::SelectOutput(_) => {
                    // TODO: save selection in prefs
                }
                MidiPanelEvent::PortsRefreshed => {
                    // TODO: remap any saved preferences to ports that we've found
                }
            }
            true
        } else {
            false
        }
    }

    fn handle_audio_panel_channel(&mut self) -> bool {
        if let Ok(m) = self.settings_panel.audio_panel().receiver().try_recv() {
            match m {
                AudioPanelEvent::InterfaceChanged => {
                    self.update_orchestrator_audio_interface_config();
                }
            }
            true
        } else {
            false
        }
    }

    fn handle_mini_orchestrator_channel(&mut self) -> bool {
        if let Ok(m) = self.orchestrator_panel.receiver().try_recv() {
            match m {
                MiniOrchestratorEvent::Tempo(_tempo) => {
                    // This is (usually) an acknowledgement that Orchestrator
                    // got our request to change, so we don't need to do
                    // anything.
                }
                MiniOrchestratorEvent::Quit => {
                    eprintln!("MiniOrchestratorEvent::Quit")
                }
                MiniOrchestratorEvent::Loaded(path, title) => {
                    self.toasts.add(Toast {
                        kind: egui_toast::ToastKind::Success,
                        text: format!(
                            "Loaded {} from {}",
                            if let Some(title) = title {
                                title
                            } else {
                                Self::DEFAULT_PROJECT_NAME.to_string()
                            },
                            path.display()
                        )
                        .into(),
                        options: ToastOptions::default()
                            .duration_in_seconds(2.0)
                            .show_progress(false),
                    });
                }
                MiniOrchestratorEvent::LoadError(path, error) => {
                    self.toasts.add(Toast {
                        kind: egui_toast::ToastKind::Error,
                        text: format!("Error loading {}: {}", path.display(), error).into(),
                        options: ToastOptions::default().duration_in_seconds(5.0),
                    });
                }
                MiniOrchestratorEvent::Saved(path) => {
                    // TODO: this should happen only if the save operation was
                    // explicit. Autosaves should be invisible.
                    self.toasts.add(Toast {
                        kind: egui_toast::ToastKind::Success,
                        text: format!("Saved to {}", path.display()).into(),
                        options: ToastOptions::default()
                            .duration_in_seconds(1.0)
                            .show_progress(false),
                    });
                }
                MiniOrchestratorEvent::SaveError(path, error) => {
                    self.toasts.add(Toast {
                        kind: egui_toast::ToastKind::Error,
                        text: format!("Error saving {}: {}", path.display(), error).into(),
                        options: ToastOptions::default().duration_in_seconds(5.0),
                    });
                }
                MiniOrchestratorEvent::New => {
                    // No special UI needed for this.
                }
            }
            true
        } else {
            false
        }
    }

    // Watches certain channels and asks for a repaint, which triggers the
    // actual channel receiver logic, when any of them has something receivable.
    //
    // https://docs.rs/crossbeam-channel/latest/crossbeam_channel/struct.Select.html#method.ready
    //
    // We call ready() rather than select() because select() requires us to
    // complete the operation that is ready, while ready() just tells us that a
    // recv() would not block.
    fn spawn_channel_watcher(&mut self, ctx: Context) {
        let r1 = self.settings_panel.midi_panel().receiver().clone();
        let r2 = self.settings_panel.audio_panel().receiver().clone();
        let r3 = self.orchestrator_panel.receiver().clone();
        let _ = std::thread::spawn(move || {
            let mut sel = Select::new();
            let _ = sel.recv(&r1);
            let _ = sel.recv(&r2);
            let _ = sel.recv(&r3);
            loop {
                let _ = sel.ready();
                ctx.request_repaint();
            }
        });
    }

    fn update_orchestrator_audio_interface_config(&mut self) {
        let sample_rate = self.settings_panel.audio_panel().sample_rate();
        if let Ok(mut o) = self.mini_orchestrator.lock() {
            o.update_sample_rate(SampleRate::from(sample_rate));
        }
    }

    fn handle_control_panel_action(&mut self, action: ControlPanelAction) {
        let input = match action {
            ControlPanelAction::Play => Some(MiniOrchestratorInput::ProjectPlay),
            ControlPanelAction::Stop => Some(MiniOrchestratorInput::ProjectStop),
            ControlPanelAction::New => Some(MiniOrchestratorInput::ProjectNew),
            ControlPanelAction::Open(path) => Some(MiniOrchestratorInput::ProjectOpen(path)),
            ControlPanelAction::Save(path) => Some(MiniOrchestratorInput::ProjectSave(path)),
            ControlPanelAction::ToggleSettings => {
                self.settings_panel.toggle();
                None
            }
        };
        if let Some(input) = input {
            self.orchestrator_panel.send_to_service(input);
        }
    }

    fn handle_menu_bar_action(&mut self, action: MenuBarAction) {
        let mut input = None;
        match action {
            MenuBarAction::Quit => self.exit_requested = true,
            MenuBarAction::TrackNewMidi => input = Some(MiniOrchestratorInput::TrackNewMidi),
            MenuBarAction::TrackNewAudio => input = Some(MiniOrchestratorInput::TrackNewAudio),
            MenuBarAction::TrackNewSend => input = Some(MiniOrchestratorInput::TrackNewSend),
            MenuBarAction::TrackDelete => input = Some(MiniOrchestratorInput::TrackDeleteSelected),
            MenuBarAction::TrackDuplicate => {
                input = Some(MiniOrchestratorInput::TrackDuplicateSelected)
            }
            MenuBarAction::TrackRemoveSelectedPatterns => {
                input = Some(MiniOrchestratorInput::TrackPatternRemoveSelected)
            }
            MenuBarAction::ComingSoon => {
                self.toasts.add(Toast {
                    kind: egui_toast::ToastKind::Info,
                    text: "Coming soon!".into(),
                    options: ToastOptions::default(),
                });
            }
            MenuBarAction::ProjectNew => input = Some(MiniOrchestratorInput::ProjectNew),
            MenuBarAction::ProjectOpen => {
                input = Some(MiniOrchestratorInput::ProjectOpen(PathBuf::from(
                    "minidaw.json",
                )))
            }
            MenuBarAction::ProjectSave => {
                input = Some(MiniOrchestratorInput::ProjectSave(PathBuf::from(
                    "minidaw.json",
                )))
            }
            MenuBarAction::TrackAddThing(key) => {
                input = Some(MiniOrchestratorInput::TrackAddThing(key))
            }
        }
        if let Some(input) = input {
            self.orchestrator_panel.send_to_service(input);
        }
    }

    #[allow(dead_code)]
    #[allow(unused_mut)]
    #[allow(unused_variables)]
    fn handle_palette_action(&mut self, action: PaletteAction) {
        if let Ok(mut o) = self.mini_orchestrator.lock() {
            match action {
                PaletteAction::NewThing(_key) => {
                    // if let Some(track) = o.get_single_selected_track_uid() {
                    //     //                        let _ = o.add_thing_by_key(&key, track);
                    // }
                }
            }
        }
    }

    fn show_top(&mut self, ui: &mut Ui) {
        if let Some(action) = self
            .menu_bar
            .show_with_action(ui, self.orchestrator_panel.is_any_track_selected())
        {
            self.handle_menu_bar_action(action);
        }
        ui.separator();
        if let Some(action) = self.control_panel.show_with_action(ui) {
            self.handle_control_panel_action(action);
        }
    }

    fn show_bottom(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            warn_if_debug_build(ui);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(app_version())
            });
        });
    }

    fn show_left(&mut self, ui: &mut Ui) {
        ScrollArea::horizontal().show(ui, |ui| {
            if let Some(_action) = self.palette_panel.show_with_action(ui) {
                // these are inactive for now because we're skipping the drag/drop stuff.
                //self.handle_palette_action(action);
            }
        });
    }

    fn show_right(&mut self, ui: &mut Ui) {
        ScrollArea::horizontal().show(ui, |ui| ui.label("Under Construction"));
    }

    fn show_center(&mut self, ui: &mut Ui, is_shift_only_down: bool) {
        ScrollArea::vertical().show(ui, |ui| {
            self.orchestrator_panel.show(ui, is_shift_only_down);
        });
    }

    fn update_window_title(&mut self, frame: &mut eframe::Frame) {
        // TODO: it seems like the window remembers its title, so this isn't
        // something we should be doing on every frame.
        let full_title = format!(
            "{} - {}",
            Self::APP_NAME,
            if let Some(title) = {
                if let Ok(o) = self.orchestrator_panel.orchestrator().lock() {
                    o.title().cloned()
                } else {
                    None
                }
            } {
                title
            } else {
                Self::DEFAULT_PROJECT_NAME.to_string()
            }
        );
        frame.set_window_title(&full_title);
    }

    fn show_settings_panel(&mut self, ctx: &Context) {
        let mut is_settings_open = self.settings_panel.is_open();
        egui::Window::new("Settings")
            .open(&mut is_settings_open)
            .show(ctx, |ui| self.settings_panel.show(ui));
        if self.settings_panel.is_open() && !is_settings_open {
            self.settings_panel.toggle();
        }
    }
}
impl eframe::App for MiniDaw {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.handle_message_channels();
        if let Ok(mut dnd) = self.drag_drop_manager.lock() {
            dnd.reset();
        }
        self.update_window_title(frame);

        let mut is_control_only_down = false;
        ctx.input(|i| {
            if i.modifiers.command_only() {
                is_control_only_down = true;
            }
        });

        // TODO: this is unlikely to be the long-term home for Orchestrator
        // updates. Decide how the UI loop should look.
        if let Ok(o) = self.mini_orchestrator.lock() {
            self.control_panel.set_transport(o.transport().clone());
        }

        let top = egui::TopBottomPanel::top("top-panel")
            .resizable(false)
            .exact_height(64.0);
        let bottom = egui::TopBottomPanel::bottom("bottom-panel")
            .resizable(false)
            .exact_height(24.0);
        let left = egui::SidePanel::left("left-panel")
            .resizable(true)
            .default_width(160.0)
            .width_range(160.0..=480.0);
        let right = egui::SidePanel::right("right-panel")
            .resizable(true)
            .default_width(160.0)
            .width_range(160.0..=480.0);
        let center = egui::CentralPanel::default();

        top.show(ctx, |ui| {
            self.show_top(ui);
        });
        bottom.show(ctx, |ui| {
            self.show_bottom(ui);
        });
        left.show(ctx, |ui| {
            self.show_left(ui);
        });
        right.show(ctx, |ui| {
            self.show_right(ui);
        });
        center.show(ctx, |ui| {
            self.show_center(ui, is_control_only_down);
            self.toasts.show(ctx);
        });

        self.show_settings_panel(ctx);

        if self.exit_requested {
            frame.close();
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.settings_panel.exit();
        self.orchestrator_panel.exit();
    }
}

fn main() -> anyhow::Result<(), eframe::Error> {
    env_logger::init();
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1366.0, 768.0)),
        ..Default::default()
    };

    eframe::run_native(
        MiniDaw::APP_NAME,
        options,
        Box::new(|cc| Box::new(MiniDaw::new(cc))),
    )
}
