// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! The egui app is an [egui](https://github.com/emilk/egui)-based DAW.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crossbeam_channel::{Receiver, Sender};
use eframe::{
    egui::{self, Context, FontData, FontDefinitions, Layout, RichText, ScrollArea, TextStyle},
    emath::Align2,
    epaint::{Color32, FontFamily, FontId},
    CreationContext,
};
use egui_toast::{Toast, ToastOptions, Toasts};
use groove::{
    app_version,
    egui_widgets::{AudioPanel, ControlBar, MidiPanel, Preferences, ThingBrowser},
    Message,
};
use groove_core::{
    time::{ClockParams, TimeSignatureParams},
    traits::gui::Shows,
};
use groove_entities::controllers::NewPattern;
use groove_orchestration::{messages::GrooveInput, Orchestrator};
use groove_utils::Paths;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Instant,
};

struct GrooveApp {
    preferences: Preferences,
    paths: Paths,

    // Used for sending messages to the app.
    sender: Sender<Message>,
    receiver: Receiver<Message>,

    orchestrator: Arc<Mutex<Orchestrator>>,

    control_bar: ControlBar,
    audio_panel: AudioPanel,
    midi_panel: MidiPanel,
    thing_browser: ThingBrowser,
    toasts: Toasts,

    #[allow(dead_code)]
    regular_font_id: FontId,
    #[allow(dead_code)]
    mono_font_id: FontId,
    bold_font_id: FontId,

    new_pattern: NewPattern,

    frames: usize,
    start_of_time: Instant,
}
impl eframe::App for GrooveApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_message_queue();

        let mut bold_font_height = 0.0;
        ctx.fonts(|f| bold_font_height = f.row_height(&self.bold_font_id));

        let top = egui::TopBottomPanel::top("control-bar")
            .resizable(false)
            .exact_height(64.0);
        let bottom = egui::TopBottomPanel::bottom("orchestrator")
            .resizable(false)
            .exact_height(bold_font_height + 2.0);
        let left = egui::SidePanel::left("left-sidebar")
            .resizable(true)
            .default_width(150.0)
            .width_range(80.0..=200.0);
        let right = egui::SidePanel::right("right-sidebar")
            .resizable(true)
            .default_width(150.0)
            .width_range(80.0..=200.0);
        let center = egui::CentralPanel::default();

        top.show(ctx, |ui| {
            if let Ok(mut o) = self.orchestrator.lock() {
                self.control_bar.show(ui, &mut o);
            }
        });
        bottom.show(ctx, |ui| {
            ui.horizontal(|ui| {
                egui::warn_if_debug_build(ui);
                let seconds = (Instant::now() - self.start_of_time).as_secs_f64();
                if seconds != 0.0 {
                    ui.label(format!("FPS {:0.2}", self.frames as f64 / seconds));
                    if seconds > 5.0 {
                        self.frames = 0;
                        self.start_of_time = Instant::now();
                    }
                }
                ui.with_layout(Layout::right_to_left(eframe::emath::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("Build: {:?}", app_version()))
                            .font(self.bold_font_id.clone())
                            .color(Color32::YELLOW),
                    )
                });
            });
        });
        left.show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.thing_browser.show(
                    ui,
                    &self.paths,
                    self.sender.clone(),
                    Arc::clone(&self.orchestrator),
                );
            });
        });
        right.show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.vertical(|ui| {
                    self.preferences.show(ui);
                    self.midi_panel.show(ui);
                    self.audio_panel.show(ui);
                });
            })
        });
        center.show(ctx, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                self.new_pattern.show(ui);
                if let Ok(mut o) = self.orchestrator.lock() {
                    o.show(ui);
                }
            });
            self.toasts.show(ctx);
        });

        // TODO: this is how to keep redrawing when the system doesn't otherwise
        // know that a repaint is needed. This is fine for now, but it's
        // expensive, and we should be smarter about it.
        ctx.request_repaint();

        self.frames += 1;
    }
}
impl GrooveApp {
    pub const FONT_REGULAR: &str = "font-regular";
    pub const FONT_BOLD: &str = "font-bold";
    pub const FONT_MONO: &str = "font-mono";

    fn new(cc: &CreationContext) -> Self {
        eprintln!("new: {:?}\n{:?}", &cc.egui_ctx, &cc.integration_info);

        Self::initialize_fonts(cc);
        Self::initialize_visuals(cc);
        Self::initialize_style(&cc.egui_ctx);

        let (sender, receiver) = crossbeam_channel::unbounded();

        let mut clock_params = ClockParams::default();
        clock_params.time_signature = TimeSignatureParams { top: 4, bottom: 4 };
        let orchestrator = Arc::new(Mutex::new(Orchestrator::new_with(&clock_params)));

        let paths = Paths::default();
        let extra_paths = Self::set_up_extra_paths();

        let load_prefs = Preferences::load();
        let prefs_result = futures::executor::block_on(load_prefs);

        let preferences = match prefs_result {
            Ok(preferences) => preferences,
            Err(e) => {
                eprintln!("While loading preferences: {:?}", e);
                Preferences::default()
            }
        };
        let mut r = Self {
            paths: paths.clone(),

            orchestrator: Arc::clone(&orchestrator),

            control_bar: ControlBar::default(),
            midi_panel: MidiPanel::new_with(sender.clone()),
            audio_panel: AudioPanel::new_with(Arc::clone(&orchestrator)),
            preferences,
            thing_browser: ThingBrowser::scan_everything(&paths, extra_paths),
            toasts: Toasts::new()
                .anchor(Align2::RIGHT_BOTTOM, (-10.0, -10.0))
                .direction(egui::Direction::BottomUp),

            regular_font_id: FontId::proportional(14.0),
            bold_font_id: FontId::new(12.0, FontFamily::Name(Self::FONT_BOLD.into())),
            mono_font_id: FontId::monospace(14.0),

            new_pattern: Default::default(),

            frames: Default::default(),
            start_of_time: Instant::now(),

            // Keep these last to avoid a bunch of temporary variables
            sender,
            receiver,
        };

        r.load_project_at_startup();

        r
    }

    fn set_up_extra_paths() -> Vec<PathBuf> {
        let mut local_projects = Paths::hive(groove_utils::PathType::Cwd);
        local_projects.push(Paths::projects_rel());
        let mut user_projects = Paths::hive(groove_utils::PathType::User);
        user_projects.push(Paths::projects_rel());
        vec![user_projects, local_projects]
    }

    fn initialize_fonts(cc: &CreationContext) {
        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            Self::FONT_REGULAR.to_owned(),
            FontData::from_static(include_bytes!("../../res/fonts/inter/Inter-Regular.ttf")),
        );
        fonts.font_data.insert(
            Self::FONT_BOLD.to_owned(),
            FontData::from_static(include_bytes!("../../res/fonts/inter/Inter-Bold.ttf")),
        );
        fonts.font_data.insert(
            Self::FONT_MONO.to_owned(),
            FontData::from_static(include_bytes!(
                "../../res/fonts/cousine/Cousine-Regular.ttf"
            )),
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

    fn initialize_visuals(_cc: &CreationContext) {
        // TODO - currently happy with defaults
    }

    fn initialize_style(ctx: &Context) {
        let mut style = (*ctx.style()).clone();

        style.visuals.override_text_color = Some(Color32::LIGHT_GRAY);

        style.text_styles = [
            (
                TextStyle::Heading,
                FontId::new(14.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Name("Heading2".into()),
                FontId::new(25.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Name("Context".into()),
                FontId::new(23.0, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(12.0, FontFamily::Proportional)),
            (
                TextStyle::Monospace,
                FontId::new(12.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Button,
                FontId::new(12.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Small,
                FontId::new(10.0, FontFamily::Proportional),
            ),
        ]
        .into();

        ctx.set_style(style);
    }

    fn load_project_at_startup(&mut self) {
        if self.preferences.should_reload_last_project() {
            if let Some(path) = self.preferences.project_filename() {
                if let Err(err) = Preferences::handle_load(
                    &self.paths,
                    Path::new(path),
                    Arc::clone(&self.orchestrator),
                ) {
                    self.preferences.set_should_reload_last_project(false);
                    self.add_error_toast(err.to_string());
                }
            }
        }
    }

    fn handle_message_queue(&mut self) {
        loop {
            if let Ok(message) = self.receiver.try_recv() {
                match message {
                    Message::Error(text) => self.add_error_toast(text),
                    Message::ProjectLoaded(Ok(path)) => {
                        self.preferences.set_project_filename(&path);
                    }
                    Message::ProjectLoaded(Err(err)) => {
                        self.add_error_toast(err.to_string());
                    }
                    Message::Midi(channel, message) => {
                        if let Ok(mut o) = self.orchestrator.lock() {
                            o.update(GrooveInput::MidiFromExternal(channel, message));
                        }
                    }
                    Message::SelectMidiInput(port) => {
                        self.preferences.set_selected_midi_input(&port.to_string())
                    }
                    Message::SelectMidiOutput(port) => {
                        self.preferences.set_selected_midi_output(&port.to_string())
                    }
                    Message::MidiPortsRefreshed => self.restore_midi_port_selections(),
                }
            } else {
                break;
            }
        }
    }

    fn add_error_toast(&mut self, text: String) {
        self.toasts.add(Toast {
            kind: egui_toast::ToastKind::Error,
            text: text.into(),
            options: ToastOptions::default(),
        });
    }

    fn restore_midi_port_selections(&mut self) {
        // Unlike the show() handlers, we don't send the
        // Message::SelectMidiInput/Output messages to the app. This is because
        // we know the app was going to reflect that information to Preferences,
        // and we don't need to do that because restore_settings() is always
        // called with the current state of Preferences.
        if let Some(port_name) = self.preferences.selected_midi_input() {
            self.midi_panel
                .send(groove_midi::MidiInterfaceInput::RestoreMidiInput(
                    port_name.clone(),
                ));
        }
        if let Some(port_name) = self.preferences.selected_midi_output() {
            self.midi_panel
                .send(groove_midi::MidiInterfaceInput::RestoreMidiOutput(
                    port_name.clone(),
                ));
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), eframe::Error> {
    env_logger::init();
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1920.0, 1080.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Groove (egui)",
        options,
        Box::new(|cc| Box::new(GrooveApp::new(cc))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // Make sure panics are logged using `console.error`.
    console_error_panic_hook::set_once();

    // Redirect tracing to console.log and friends:
    tracing_wasm::set_as_global_default();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::start_web(
            "canvas_id", // hardcode it
            web_options,
            Box::new(|cc| Box::new(eframe_template::TemplateApp::new(cc))),
        )
        .await
        .expect("failed to start eframe");
    });
}
