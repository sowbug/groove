// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::mini::Transport;
use eframe::{
    egui::{Label, Layout, RichText, TextStyle},
    emath::Align,
    epaint::vec2,
};
use ensnare::traits::Displays;

/// Wraps a [Transport] as a [Widget](eframe::egui::Widget).
pub fn transport(transport: &mut Transport) -> impl eframe::egui::Widget + '_ {
    move |ui: &mut eframe::egui::Ui| TransportWidget::new(transport).ui(ui)
}

#[derive(Debug)]
struct TransportWidget<'a> {
    transport: &'a mut Transport,
}
impl<'a> TransportWidget<'a> {
    fn new(transport: &'a mut Transport) -> Self {
        Self { transport }
    }
}
impl<'a> Displays for TransportWidget<'a> {
    fn ui(&mut self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        ui.allocate_ui(vec2(72.0, 20.0), |ui| {
            ui.set_min_width(128.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add(Label::new(
                    RichText::new(format!("{:0.2}", self.transport.tempo))
                        .text_style(TextStyle::Monospace),
                ));
            });
        })
        .response
            | ui.allocate_ui(vec2(72.0, 20.0), |ui| {
                ui.set_min_width(128.0);
                ui.add(Label::new(
                    RichText::new(format!("{}", self.transport.current_time()))
                        .text_style(TextStyle::Monospace),
                ));
            })
            .response
    }
}
