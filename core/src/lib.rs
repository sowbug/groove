// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! Fundamental structs and traits.

use derive_more::Display;
use ensnare::core::{BipolarNormal, Normal, Sample, SampleType, StereoSample};
use groove_proc_macros::{Control, Params};
use std::hash::Hash;

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

/// This struct doesn't do anything. It exists only to let the doc system know
/// what the name of the project is.
pub struct Groove;

/// Handles automation, or real-time automatic control of one entity's
/// parameters by another entity's output.
pub mod control;
/// Contains things that generate signals, like oscillators and envelopes.
pub mod generators;
/// Building blocks for higher-level musical instruments. Useful if your project
/// needs Groove's synth voices but not all its baggage.
pub mod instruments;
/// Knows about [MIDI](https://en.wikipedia.org/wiki/MIDI).
pub mod midi;
/// Handles digital-audio, wall-clock, and musical time.
pub mod time;
/// Describes major public interfaces.
pub mod traits;
/// Contains various helper functions that keep different parts of the system
/// consistent.
pub mod util;
/// Contains things that make up instrument voices.
pub mod voices;

pub const SAMPLE_BUFFER_SIZE: usize = 64;

// TODO: I'm not convinced this is useful.
/// [MonoSample] is a single-channel sample. It exists separately from [Sample]
/// for cases where we specifically want a monophonic audio stream.
#[derive(Debug, Default, PartialEq, PartialOrd)]
pub struct MonoSample(pub SampleType);

pub trait IsUid: Eq + Hash + Clone + Copy {
    fn increment(&mut self) -> &Self;
}

/// A [Uid] is an identifier that's unique within the current project.
#[derive(Copy, Clone, Debug, Default, Display, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Uid(pub usize);
impl IsUid for Uid {
    fn increment(&mut self) -> &Self {
        self.0 += 1;
        self
    }
}

/// The Digitally Controller Amplifier (DCA) handles gain and pan for many kinds
/// of synths.
///
/// See DSSPC++, Section 7.9 for requirements. TODO: implement
#[derive(Debug, Default, Control, Params)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Dca {
    #[control]
    #[params]
    gain: Normal,
    #[control]
    #[params]
    pan: BipolarNormal,
}
impl Dca {
    pub fn new_with(params: &DcaParams) -> Self {
        Self {
            gain: params.gain(),
            pan: params.pan(),
        }
    }

    pub fn transform_audio_to_stereo(&mut self, input_sample: Sample) -> StereoSample {
        // See Pirkle, DSSPC++, p.73
        let input_sample: f64 = input_sample.0 * self.gain.value();
        let left_pan: f64 = 1.0 - 0.25 * (self.pan.value() + 1.0).powi(2);
        let right_pan: f64 = 1.0 - (0.5 * self.pan.value() - 0.5).powi(2);
        StereoSample::new(
            (left_pan * input_sample).into(),
            (right_pan * input_sample).into(),
        )
    }

    pub fn gain(&self) -> Normal {
        self.gain
    }

    pub fn set_gain(&mut self, gain: Normal) {
        self.gain = gain;
    }

    pub fn pan(&self) -> BipolarNormal {
        self.pan
    }

    pub fn set_pan(&mut self, pan: BipolarNormal) {
        self.pan = pan;
    }

    pub fn update_from_params(&mut self, params: &DcaParams) {
        self.set_gain(params.gain());
        self.set_pan(params.pan());
    }
}

#[cfg(feature = "egui-framework")]
mod gui {
    use crate::{traits::gui::Displays, BipolarNormal, Dca, Normal};
    use eframe::egui::Slider;

    impl Displays for Dca {
        fn ui(&mut self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
            let mut gain = self.gain().value();
            let gain_response = ui.add(
                Slider::new(&mut gain, Normal::range())
                    .fixed_decimals(2)
                    .text("Gain"),
            );
            if gain_response.changed() {
                self.set_gain(gain.into());
            };

            let mut pan = self.pan().value();
            let pan_response = ui.add(
                Slider::new(&mut pan, BipolarNormal::range())
                    .fixed_decimals(2)
                    .text("Pan"),
            );
            if pan_response.changed() {
                self.set_pan(pan.into());
            };
            gain_response | pan_response
        }
    }
}

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
