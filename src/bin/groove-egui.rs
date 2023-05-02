// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! The egui app is an [egui](https://github.com/emilk/egui)-based DAW.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crossbeam_channel::{Receiver, Sender};
use eframe::{
    egui::{self, Context, FontData, FontDefinitions, Layout, RichText, TextStyle},
    emath::Align2,
    epaint::{Color32, FontFamily, FontId},
    CreationContext,
};
use egui_extras::StripBuilder;
use egui_toast::{Toast, ToastOptions, Toasts};
use groove::{
    app_version,
    egui_widgets::{AudioPanel, ControlBar, MidiPanel, Preferences, ThingBrowser},
    Message,
};
use groove_core::{time::ClockNano, traits::gui::Shows};
use groove_orchestration::Orchestrator;
use groove_utils::Paths;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

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
            ui.with_layout(Layout::right_to_left(eframe::emath::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("Build: {:?}", app_version()))
                        .font(self.bold_font_id.clone())
                        .color(Color32::YELLOW),
                )
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
                // Just experimenting
                StripBuilder::new(ui)
                    .size(egui_extras::Size::exact(80.0))
                    .size(egui_extras::Size::exact(50.0))
                    .vertical(|mut strip| {
                        strip.cell(|ui| self.midi_panel.show(ui));
                        strip.cell(|ui| self.audio_panel.show(ui))
                    });
            });
        });
        center.show(ctx, |ui| {
            if let Ok(mut o) = self.orchestrator.lock() {
                o.show(ui);
            }
            self.toasts.show(ctx);
        });

        // TODO: this is how to keep redrawing when the system doesn't otherwise
        // know that a repaint is needed. This is fine for now, but it's
        // expensive, and we should be smarter about it.
        ctx.request_repaint();
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

        let clock_settings = ClockNano::default();
        let orchestrator = Arc::new(Mutex::new(Orchestrator::new_with(clock_settings)));

        let paths = Paths::default();
        let extra_paths = Self::set_up_extra_paths();

        let load_prefs = Preferences::load();
        let prefs_result = futures::executor::block_on(load_prefs);

        let mut r = Self {
            preferences: match prefs_result {
                Ok(preferences) => preferences,
                Err(e) => {
                    eprintln!("While loading preferences: {:?}", e);
                    Preferences::default()
                }
            },
            paths: paths.clone(),

            sender,
            receiver,

            orchestrator: Arc::clone(&orchestrator),

            control_bar: ControlBar::default(),
            audio_panel: AudioPanel::new_with(Arc::clone(&orchestrator)),
            midi_panel: MidiPanel::new_with(),
            thing_browser: ThingBrowser::scan_everything(&paths, extra_paths),
            toasts: Toasts::new()
                .anchor(Align2::RIGHT_BOTTOM, (-10.0, -10.0))
                .direction(egui::Direction::BottomUp),

            regular_font_id: FontId::proportional(14.0),
            bold_font_id: FontId::new(12.0, FontFamily::Name(Self::FONT_BOLD.into())),
            mono_font_id: FontId::monospace(14.0),
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
        if let Some(path) = self.preferences.last_project_filename() {
            if let Err(err) = Preferences::handle_load(
                &self.paths,
                Path::new(path.as_str()),
                Arc::clone(&self.orchestrator),
            ) {
                self.add_error_toast(err.to_string());
            }
        }
    }

    fn handle_message_queue(&mut self) {
        loop {
            if let Ok(message) = self.receiver.try_recv() {
                match message {
                    Message::Error(text) => self.add_error_toast(text),
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
}
