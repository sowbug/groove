// Copyright (c) 2023 Mike Tsao. All rights reserved.

use eframe::{
    egui::{self, Frame, Label},
    emath,
    epaint::{self, pos2, vec2, Color32, Pos2, Rect, Stroke},
};

use ensnare::traits::Displays;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Default)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Waveform {}
impl Displays for Waveform {
    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let color = if ui.visuals().dark_mode {
            Color32::from_additive_luminance(196)
        } else {
            Color32::from_black_alpha(240)
        };

        Frame::canvas(ui.style()).show(ui, |ui| {
            ui.ctx().request_repaint();
            let time = ui.input(|i| i.time);

            let desired_size = ui.available_width() * vec2(1.0, 0.35);
            let (_id, rect) = ui.allocate_space(desired_size);

            let to_screen =
                emath::RectTransform::from_to(Rect::from_x_y_ranges(0.0..=1.0, -1.0..=1.0), rect);

            let mut shapes = vec![];

            for &mode in &[2, 3, 5] {
                let mode = mode as f64;
                let n = 120;
                let speed = 1.5;

                let points: Vec<Pos2> = (0..=n)
                    .map(|i| {
                        let t = i as f64 / (n as f64);
                        let amp = (time * speed * mode).sin() / mode;
                        let y = amp * (t * std::f64::consts::TAU / 2.0 * mode).sin();
                        to_screen * pos2(t as f32, y as f32)
                    })
                    .collect();

                let thickness = 10.0 / mode as f32;
                shapes.push(epaint::Shape::line(points, Stroke::new(thickness, color)));
            }

            ui.painter().extend(shapes);
        });
        ui.vertical_centered(|ui| ui.add(Label::new("hello!")))
            .inner
    }
}
