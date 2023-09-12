// Copyright (c) 2023 Mike Tsao. All rights reserved.

use eframe::epaint::FontId;
use eframe::{
    egui::{Frame, Margin, Sense, TextFormat},
    emath::Align,
    epaint::{text::LayoutJob, vec2, Color32, Shape, Stroke, TextShape},
};
use groove_core::traits::gui::Displays;
use std::f32::consts::PI;

/// Wraps a [TitleBar] as a [Widget](eframe::egui::Widget).
pub fn title_bar(title: &mut String) -> impl eframe::egui::Widget + '_ {
    move |ui: &mut eframe::egui::Ui| TitleBar::new(title).ui(ui)
}

/// An egui widget that draws a [Track]'s sideways title bar.
#[derive(Debug)]
pub struct TitleBar<'a> {
    title: &'a mut String,
}
impl<'a> Displays for TitleBar<'a> {
    fn ui(&mut self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        let available_size = vec2(16.0, ui.available_height());
        ui.set_min_size(available_size);
        Frame::default()
            .outer_margin(Margin::same(1.0))
            .inner_margin(Margin::same(0.0))
            .fill(Color32::DARK_GRAY)
            .show(ui, |ui| {
                ui.allocate_ui(available_size, |ui| {
                    let mut job = LayoutJob::default();
                    job.append(
                        self.title.as_str(),
                        1.0,
                        TextFormat {
                            color: Color32::YELLOW,
                            font_id: FontId::proportional(12.0),
                            valign: Align::Center,
                            ..Default::default()
                        },
                    );
                    let galley = ui.ctx().fonts(|f| f.layout_job(job));
                    let (response, painter) = ui.allocate_painter(available_size, Sense::click());
                    let t = Shape::Text(TextShape {
                        pos: response.rect.left_bottom(),
                        galley,
                        underline: Stroke::default(),
                        override_text_color: None,
                        angle: 2.0 * PI * 0.75,
                    });
                    painter.add(t);
                    response
                })
                .inner
            })
            .inner
    }
}
impl<'a> TitleBar<'a> {
    fn new(title: &'a mut String) -> Self {
        Self { title }
    }
}
