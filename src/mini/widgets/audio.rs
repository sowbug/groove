// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::mini::rng::Rng;
use eframe::{
    egui::{self, Sense},
    emath::RectTransform,
    epaint::{pos2, Color32, Rect, RectShape, Rounding, Stroke},
};
use ensnare::prelude::*;
use ensnare::traits::Displays;
use spectrum_analyzer::{scaling::divide_by_N_sqrt, FrequencyLimit};

/// A fixed-size circular buffer for use by audio widgets.
#[derive(Debug, Default)]
pub struct CircularSampleBuffer {
    buffer: Vec<Sample>,
    cursor: usize,
    rng: Rng,
}
impl CircularSampleBuffer {
    /// Creates a new [CircularSampleBuffer] of the given size.
    pub fn new(size: usize) -> Self {
        let mut r = Self {
            buffer: Vec::with_capacity(size),
            cursor: Default::default(),
            rng: Rng::default(),
        };
        r.buffer.resize(size, Sample::SILENCE);
        r
    }

    /// Returns the start of the buffer in memory and the cursor position. It is
    /// the caller's responsibility to figure out the boundaries of the buffer
    /// using the cursor value.
    pub fn get(&self) -> (&[Sample], usize) {
        (&self.buffer, self.cursor)
    }

    /// Adds a slice of [Sample]s to the buffer, overwriting what's already there.
    pub fn push(&mut self, new_samples: &[Sample]) {
        let src_len = new_samples.len();
        let dst_len = self.buffer.len();
        if src_len > dst_len {
            panic!("Error: tried to push too much data into circular buffer");
        }
        let d = &mut self.buffer;

        // Copy as much of the src as we can with the first memcpy.
        let available_dst_len = dst_len - self.cursor;
        let part_1_len = src_len.min(available_dst_len);
        d[self.cursor..(self.cursor + part_1_len)].copy_from_slice(&new_samples[0..part_1_len]);
        self.cursor += part_1_len;
        if self.cursor >= dst_len {
            self.cursor = 0;
        }

        // If needed, copy the rest with a second memcpy.
        if part_1_len < src_len {
            let part_2_len = src_len - part_1_len;
            d[0..part_2_len].copy_from_slice(&new_samples[part_1_len..]);
            self.cursor += part_2_len;

            // This could happen if self.cursor was at the max position and
            // src_len == dst_len
            if self.cursor >= dst_len {
                self.cursor = 0;
            }
        }
    }

    /// TODO remove - temp for development
    pub fn add_some_noise(&mut self) {
        let new_samples: Vec<Sample> = (0..8)
            .map(|_| Sample::from(Normal::from(self.rng.0.rand_u64() as f64 / u64::MAX as f64)))
            .collect();
        self.push(&new_samples);
    }

    /// Does a quick-and-dirty FFT of the sample buffer, producing a Vec<f32>
    /// that is suitable for an unlabeled visualization. If you want labels,
    /// then do this transformation yourself so you can display the Hz bucket
    /// labels.
    pub fn analyze_spectrum(
        &self,
    ) -> anyhow::Result<Vec<f32>, spectrum_analyzer::error::SpectrumAnalyzerError> {
        let samples: Vec<f32> = self.buffer.iter().map(|x| x.0 as f32).collect();
        let hann_window = spectrum_analyzer::windows::hann_window(&samples);
        let spectrum = spectrum_analyzer::samples_fft_to_spectrum(
            &hann_window,
            44100,
            FrequencyLimit::All,
            Some(&divide_by_N_sqrt),
        )?;
        Ok(spectrum.data().iter().map(|(_hz, val)| val.val()).collect())
    }
}

/// Wraps a [TimeDomain] as a [Widget](eframe::egui::Widget).
pub fn time_domain(samples: &[Sample], start: usize) -> impl eframe::egui::Widget + '_ {
    move |ui: &mut eframe::egui::Ui| TimeDomain::new(samples, start).ui(ui)
}

/// Wraps a [FrequencyDomain] as a [Widget](eframe::egui::Widget).
pub fn frequency_domain(values: &[f32]) -> impl eframe::egui::Widget + '_ {
    move |ui: &mut eframe::egui::Ui| FrequencyDomain::new(values).ui(ui)
}

/// Creates 256 samples of noise.
pub fn init_random_samples() -> [Sample; 256] {
    let mut r = [Sample::default(); 256];
    let mut rng = Rng::default();
    for s in &mut r {
        let value = rng.0.rand_float().fract() * 2.0 - 1.0;
        *s = Sample::from(value);
    }
    r
}

/// Displays a series of [Sample]s in the time domain. That's a fancy way of
/// saying it shows the amplitudes.
#[derive(Debug)]
pub struct TimeDomain<'a> {
    samples: &'a [Sample],
    start: usize,
}
impl<'a> TimeDomain<'a> {
    fn new(samples: &'a [Sample], start: usize) -> Self {
        Self { samples, start }
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

/// Displays a series of [Sample]s in the frequency domain. Or, to put it
/// another way, shows a spectrum analysis of a clip.
#[derive(Debug)]
pub struct FrequencyDomain<'a> {
    values: &'a [f32],
}
impl<'a> FrequencyDomain<'a> {
    fn new(values: &'a [f32]) -> Self {
        Self { values }
    }
}
impl<'a> Displays for FrequencyDomain<'a> {
    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let (response, painter) =
            ui.allocate_painter(ui.available_size_before_wrap(), Sense::hover());

        let buf_min = 0.0;
        let buf_max = 1.0;

        #[allow(unused_variables)]
        let to_screen = RectTransform::from_to(
            Rect::from_x_y_ranges(0.0..=self.values.len() as f32, buf_max..=buf_min),
            response.rect,
        );
        let mut shapes = Vec::default();

        shapes.push(eframe::epaint::Shape::Rect(RectShape {
            rect: response.rect,
            rounding: Rounding::same(3.0),
            fill: Color32::DARK_GREEN,
            stroke: Stroke {
                width: 2.0,
                color: Color32::YELLOW,
            },
        }));

        for (i, value) in self.values.iter().enumerate() {
            shapes.push(eframe::epaint::Shape::LineSegment {
                points: [
                    to_screen * pos2(i as f32, buf_min),
                    to_screen * pos2(i as f32, *value),
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
