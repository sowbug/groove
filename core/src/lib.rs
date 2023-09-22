// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! Fundamental structs and traits.

use eframe::egui::Slider;
use ensnare::{prelude::*, traits::prelude::*};
use ensnare_proc_macros::{Control, Params};
use serde::{Deserialize, Serialize};

/// This struct doesn't do anything. It exists only to let the doc system know
/// what the name of the project is.
pub struct Groove;

/// Handles automation, or real-time automatic control of one entity's
/// parameters by another entity's output.
pub mod control;
/// Contains things that generate signals, like oscillators and envelopes.
pub mod generators;
/// Knows about [MIDI](https://en.wikipedia.org/wiki/MIDI).
pub mod midi;
/// Handles digital-audio, wall-clock, and musical time.
pub mod time;
/// Contains various helper functions that keep different parts of the system
/// consistent.
pub mod util;

pub const SAMPLE_BUFFER_SIZE: usize = 64;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mono_to_stereo() {
        assert_eq!(StereoSample::from(Sample::MIN), StereoSample::MIN);
        assert_eq!(StereoSample::from(Sample::SILENCE), StereoSample::SILENCE);
        assert_eq!(StereoSample::from(Sample::MAX), StereoSample::MAX);
    }

    #[test]
    fn stereo_to_mono() {
        assert_eq!(Sample::from(ensnare::core::StereoSample::MIN), Sample::MIN);
        assert_eq!(
            Sample::from(ensnare::core::StereoSample::SILENCE),
            Sample::SILENCE
        );
        assert_eq!(Sample::from(ensnare::core::StereoSample::MAX), Sample::MAX);

        assert_eq!(
            Sample::from(ensnare::core::StereoSample::new(1.0.into(), 0.0.into())),
            Sample::from(0.5)
        );
    }

    #[test]
    fn normal_mainline() {
        let a = Normal::new(0.2);
        let b = Normal::new(0.1);

        // Add(Normal)
        assert_eq!(a + b, Normal::new(0.2 + 0.1), "Addition should work.");

        // Sub(Normal)
        assert_eq!(a - b, Normal::new(0.1), "Subtraction should work.");

        // Add(f64)
        assert_eq!(a + 0.2f64, Normal::new(0.4), "Addition of f64 should work.");

        // Sub(f64)
        assert_eq!(a - 0.1, Normal::new(0.1), "Subtraction of f64 should work.");
    }

    #[test]
    fn normal_out_of_bounds() {
        assert_eq!(
            Normal::new(-1.0),
            Normal::new(0.0),
            "Normal below 0.0 should be clamped to 0.0"
        );
        assert_eq!(
            Normal::new(1.1),
            Normal::new(1.0),
            "Normal above 1.0 should be clamped to 1.0"
        );
    }

    #[test]
    fn convert_sample_to_normal() {
        assert_eq!(
            Normal::from(Sample(-0.5)),
            Normal::new(0.25),
            "Converting Sample -0.5 to Normal should yield 0.25"
        );
        assert_eq!(
            Normal::from(Sample(0.0)),
            Normal::new(0.5),
            "Converting Sample 0.0 to Normal should yield 0.5"
        );
    }

    #[test]
    fn convert_bipolar_normal_to_normal() {
        assert_eq!(
            Normal::from(BipolarNormal::from(-1.0)),
            Normal::new(0.0),
            "Bipolar -> Normal wrong"
        );
        assert_eq!(
            Normal::from(BipolarNormal::from(0.0)),
            Normal::new(0.5),
            "Bipolar -> Normal wrong"
        );
        assert_eq!(
            Normal::from(BipolarNormal::from(1.0)),
            Normal::new(1.0),
            "Bipolar -> Normal wrong"
        );
    }

    #[test]
    fn convert_normal_to_bipolar_normal() {
        assert_eq!(
            BipolarNormal::from(Normal::from(0.0)),
            BipolarNormal::new(-1.0),
            "Normal -> Bipolar wrong"
        );
        assert_eq!(
            BipolarNormal::from(Normal::from(0.5)),
            BipolarNormal::new(0.0),
            "Normal -> Bipolar wrong"
        );
        assert_eq!(
            BipolarNormal::from(Normal::from(1.0)),
            BipolarNormal::new(1.0),
            "Normal -> Bipolar wrong"
        );
    }

    #[test]
    fn dca_mainline() {
        let mut dca = Dca::new_with(&DcaParams {
            gain: 1.0.into(),
            pan: BipolarNormal::zero(),
        });
        const VALUE_IN: Sample = Sample(0.5);
        const VALUE: Sample = Sample(0.5);
        assert_eq!(
            dca.transform_audio_to_stereo(VALUE_IN),
            StereoSample::new(VALUE * 0.75, VALUE * 0.75),
            "Pan center should give 75% equally to each channel"
        );

        dca.set_pan(BipolarNormal::new(-1.0));
        assert_eq!(
            dca.transform_audio_to_stereo(VALUE_IN),
            StereoSample::new(VALUE, 0.0.into()),
            "Pan left should give 100% to left channel"
        );

        dca.set_pan(BipolarNormal::new(1.0));
        assert_eq!(
            dca.transform_audio_to_stereo(VALUE_IN),
            StereoSample::new(0.0.into(), VALUE),
            "Pan right should give 100% to right channel"
        );
    }

    #[test]
    fn convert_sample_to_i16() {
        assert_eq!(Sample::MAX.into_i16(), i16::MAX);
        assert_eq!(Sample::MIN.into_i16(), i16::MIN);
        assert_eq!(Sample::SILENCE.into_i16(), 0);
    }

    #[test]
    fn convert_stereo_sample_to_i16() {
        let s = StereoSample(Sample::MIN, Sample::MAX);
        let (l, r) = s.into_i16();
        assert_eq!(l, i16::MIN);
        assert_eq!(r, i16::MAX);
    }
}
