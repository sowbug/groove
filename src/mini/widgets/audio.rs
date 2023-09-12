// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::mini::rng::Rng;
use eframe::{
    egui::{self, Sense},
    emath::RectTransform,
    epaint::{pos2, Color32, Rect, RectShape, Rounding, Stroke},
};
use groove_core::{traits::gui::Displays, Sample};

/// Wraps a [TimeDomain] as a [Widget](eframe::egui::Widget).
pub fn time_domain(samples: &[Sample], start: usize) -> impl eframe::egui::Widget + '_ {
    move |ui: &mut eframe::egui::Ui| TimeDomain::new(samples, start).ui(ui)
}

#[derive(Debug)]
pub struct TimeDomain<'a> {
    samples: &'a [Sample],
    start: usize,
}
impl<'a> TimeDomain<'a> {
    fn new(samples: &'a [Sample], start: usize) -> Self {
        Self { samples, start }
    }

    pub fn init_random_samples() -> [Sample; 256] {
        let mut r = [Sample::default(); 256];
        let mut rng = Rng::default();
        for s in &mut r {
            let value = rng.0.rand_float().fract() * 2.0 - 1.0;
            *s = Sample::from(value);
        }
        r
    }
}
impl<'a> Displays for TimeDomain<'a> {
    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let (response, painter) =
            ui.allocate_painter(ui.available_size_before_wrap(), Sense::hover());

        let to_screen = RectTransform::from_to(
            Rect::from_x_y_ranges(
                0.0..=self.samples.len() as f32,
                Sample::MAX.0 as f32..=Sample::MIN.0 as f32,
            ),
            response.rect,
        );
        let mut shapes = Vec::default();

        shapes.push(eframe::epaint::Shape::Rect(RectShape {
            rect: response.rect,
            rounding: Rounding::same(3.0),
            fill: Color32::DARK_BLUE,
            stroke: Stroke {
                width: 2.0,
                color: Color32::YELLOW,
            },
        }));

        for i in 0..self.samples.len() {
            let cursor = (self.start + i) % self.samples.len();
            let sample = self.samples[cursor];
            shapes.push(eframe::epaint::Shape::LineSegment {
                points: [
                    to_screen * pos2(i as f32, Sample::MIN.0 as f32),
                    to_screen * pos2(i as f32, sample.0 as f32),
                ],
                stroke: Stroke {
                    width: 1.0,
                    color: Color32::YELLOW,
                },
            })
        }

        painter.extend(shapes);
        response
    }
}
