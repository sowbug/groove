// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! The egui app is an [egui](https://github.com/emilk/egui)-based DAW.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::{
    egui::{self, Context, FontData, FontDefinitions, Layout, RichText, TextStyle},
    epaint::{Color32, FontFamily, FontId},
    CreationContext,
};
use egui_extras::StripBuilder;
use groove::egui_widgets::{AudioPanel, ControlBar, MidiPanel, ThingBrowser};
use groove_core::{time::ClockNano, traits::gui::Shows};
use groove_orchestration::Orchestrator;
use std::sync::{Arc, Mutex};

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
    orchestrator: Arc<Mutex<Orchestrator>>,

    control_bar: ControlBar,
    audio_panel: AudioPanel,
    midi_panel: MidiPanel,

    thing_browser: ThingBrowser,

    #[allow(dead_code)]
    regular_font_id: FontId,
    #[allow(dead_code)]
    mono_font_id: FontId,
    bold_font_id: FontId,
}
impl eframe::App for GrooveApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
            if let Ok(o) = self.orchestrator.lock() {
                ui.with_layout(Layout::left_to_right(eframe::emath::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("clock: {:?}", o.clock()))
                            .font(self.bold_font_id.clone())
                            .color(Color32::YELLOW),
                    )
                });
            }
        });
        left.show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.thing_browser.show(ui, Arc::clone(&self.orchestrator));
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

        let clock_settings = ClockNano::default();
        let orchestrator = Arc::new(Mutex::new(Orchestrator::new_with(clock_settings)));

        Self {
            orchestrator: Arc::clone(&orchestrator),

            control_bar: ControlBar::default(),
            audio_panel: AudioPanel::new_with(Arc::clone(&orchestrator)),
            midi_panel: MidiPanel::new_with(),
            thing_browser: ThingBrowser::scan_everything(),

            regular_font_id: FontId::proportional(14.0),
            bold_font_id: FontId::new(12.0, FontFamily::Name(Self::FONT_BOLD.into())),
            mono_font_id: FontId::monospace(14.0),
        }
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
}
