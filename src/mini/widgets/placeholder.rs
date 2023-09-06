// Copyright (c) 2023 Mike Tsao. All rights reserved.

use eframe::{
    egui::Sense,
    emath,
    epaint::{pos2, vec2, Color32, Pos2, Rect, Shape, Stroke},
};
use groove_core::traits::gui::Displays;

/// Wraps a [Wiggler] as a [Widget](eframe::egui::Widget).
pub fn wiggler() -> impl eframe::egui::Widget {
    move |ui: &mut eframe::egui::Ui| Wiggler::new().ui(ui)
}

/// A placeholder widget that fills available space with an animation.
#[derive(Debug, Default)]
pub struct Wiggler {}
impl Wiggler {
    #[allow(missing_docs)]
    pub fn new() -> Self {
        Default::default()
    }
}
impl Displays for Wiggler {
    fn ui(&mut self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        ui.ctx().request_repaint();

        let color = if ui.visuals().dark_mode {
            Color32::from_additive_luminance(196)
        } else {
            Color32::from_black_alpha(240)
        };

        let (response, painter) =
            ui.allocate_painter(vec2(ui.available_width(), 64.0), Sense::click());

        let time = ui.input(|i| i.time);
        let to_screen = emath::RectTransform::from_to(
            Rect::from_x_y_ranges(0.0..=1.0, -1.0..=1.0),
            response.rect,
        );

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
            shapes.push(Shape::line(points, Stroke::new(thickness, color)));
        }

        shapes.push(Shape::LineSegment {
            points: [to_screen * pos2(0.0, 1.0), to_screen * pos2(1.0, 1.0)],
            stroke: Stroke { width: 1.0, color },
        });

        painter.extend(shapes);

        response
    }
}
