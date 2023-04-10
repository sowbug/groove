// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! Fundamental structs and traits.

use control::F32ControlValue;
use groove_proc_macros::Nano;
use std::str::FromStr;
use std::{
    fmt::Display,
    iter::Sum,
    ops::{Add, AddAssign, Div, Mul, Neg, Sub},
};
use strum::EnumCount;
use strum_macros::{Display, EnumCount as EnumCountMacro, EnumString, FromRepr, IntoStaticStr};

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

/// [SampleType] is the underlying primitive that makes up [MonoSample] and
/// [StereoSample]. It exists as a transition aid while we migrate from
/// hardcoded f32 to [MonoSample]/[StereoSample].
pub type SampleType = f64;

/// [SignalType] is the primitive used for general digital signal-related work.
/// It's pretty important that all of these different types be the same (e.g.,
/// for now f64), but I'm hoping it's worth the hassle to use different names
/// depending on usage.
pub type SignalType = f64;

/// Use [ParameterType] in places where a [Normal] or [BipolarNormal] could fit,
/// except you don't have any range restrictions.
pub type ParameterType = f64;

/// [Sample] represents a single audio sample.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Sample(pub SampleType);
impl Sample {
    pub const SILENCE_VALUE: SampleType = 0.0;
    pub const SILENCE: Sample = Sample(Self::SILENCE_VALUE);
    pub const MAX_VALUE: SampleType = 1.0;
    pub const MAX: Sample = Sample(Self::MAX_VALUE);
    pub const MIN_VALUE: SampleType = -1.0;
    pub const MIN: Sample = Sample(Self::MIN_VALUE);
}
impl AddAssign for Sample {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}
impl Add for Sample {
    type Output = Self;

    fn add(self, rhs: Sample) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}
impl Mul for Sample {
    type Output = Self;

    fn mul(self, rhs: Sample) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}
impl Mul<f64> for Sample {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self(self.0 * rhs)
    }
}
// TODO #[deprecated] because it hides evidence that migration to SampleType
// isn't complete
impl Mul<f32> for Sample {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self(self.0 * rhs as f64)
    }
}
impl Div<f64> for Sample {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self(self.0 / rhs)
    }
}
impl Sub for Sample {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}
impl Neg for Sample {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}
impl Mul<i16> for Sample {
    type Output = Self;

    fn mul(self, rhs: i16) -> Self::Output {
        Self(self.0 * rhs as f64)
    }
}
impl From<f64> for Sample {
    fn from(value: f64) -> Self {
        Sample(value)
    }
}
impl From<f32> for Sample {
    fn from(value: f32) -> Self {
        Sample(value as f64)
    }
}
impl From<i32> for Sample {
    // TODO: this is an incomplete conversion, because we don't know what the
    // range of the i32 really is. So we leave it to someone else to divide by
    // the correct value to obtain the proper -1.0..=1.0 range.
    fn from(value: i32) -> Self {
        Sample(value as f64)
    }
}
// I predict this conversion will someday be declared evil. We're naively
// averaging the two channels. I'm not sure this makes sense in all situations.
impl From<StereoSample> for Sample {
    fn from(value: StereoSample) -> Self {
        Sample((value.0 .0 + value.1 .0) * 0.5)
    }
}
impl From<BipolarNormal> for Sample {
    fn from(value: BipolarNormal) -> Self {
        Sample(value.0)
    }
}

// TODO: I'm not convinced this is useful.
/// [MonoSample] is a single-channel sample. It exists separately from [Sample]
/// for cases where we specifically want a monophonic audio stream.
#[derive(Debug, Default, PartialEq, PartialOrd)]
pub struct MonoSample(pub SampleType);

/// [StereoSample] is a two-channel sample.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct StereoSample(pub Sample, pub Sample);
impl StereoSample {
    pub const SILENCE: StereoSample = StereoSample(Sample::SILENCE, Sample::SILENCE);
    pub const MAX: StereoSample = StereoSample(Sample::MAX, Sample::MAX);
    pub const MIN: StereoSample = StereoSample(Sample::MIN, Sample::MIN);

    pub fn new_from_f64(left: SampleType, right: SampleType) -> Self {
        Self(Sample(left), Sample(right))
    }

    // TODO: is this necessary? Wouldn't a fluent Rust coder use .into()?
    pub fn new_from_single_f64(value: SampleType) -> Self {
        Self::new_from_f64(value, value)
    }

    // This method should be used only for testing. TODO: get rid of this. Now
    // that we're in a separate crate, we can't easily limit this to test cfg
    // only. That means it's part of the API.
    //
    // TODO: epsilon comparisons are bad. Recommend float-cmp crate instead of
    // this.
    pub fn almost_equals(&self, rhs: Self) -> bool {
        let epsilon = 0.0000001;
        (self.0 .0 - rhs.0 .0).abs() < epsilon && (self.1 .0 - rhs.1 .0).abs() < epsilon
    }
}
impl Add for StereoSample {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        StereoSample(self.0 + rhs.0, self.1 + rhs.1)
    }
}
impl AddAssign for StereoSample {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}
impl Sum for StereoSample {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self(Sample::SILENCE, Sample::SILENCE), |a, b| {
            Self(a.0 + b.0, a.1 + b.1)
        })
    }
}
impl From<Sample> for StereoSample {
    fn from(value: Sample) -> Self {
        Self(value, value)
    }
}
impl From<f64> for StereoSample {
    fn from(value: f64) -> Self {
        Self(Sample(value), Sample(value))
    }
}

// TODO: I tried implementing this using the sort-of new generic const
// expressions, because I wanted to see whether I could have compile-time
// errors for attempts to set the value outside the range. I did not succeed.

/// [RangedF64] enforces the given range limits while not becoming too expensive
/// to use compared to a plain f64. It enforces the value at creation, when
/// setting it explicitly, when converting from an f64, and when getting it. But
/// math operations (Add, Sub, etc.) are not checked! This allows certain
/// operations to (hopefully temporarily) exceed the range, or for
/// floating-point precision problems to (again hopefully) get compensated for
/// later on.
///
/// Also note that [RangedF64] doesn't tell you when clamping happens. It just
/// does it, silently.
///
/// Altogether, [RangedF64] is good for gatekeeping -- parameters, return
/// values, etc., -- and somewhat OK at pure math. But we might decide to clamp
/// (heh) down on out-of-bounds conditions later on, so if you want to do math,
/// prefer f64 sourced from [RangedF64] rather than [RangedF64] itself.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct RangedF64<const LOWER: i8, const UPPER: i8>(f64);
impl<const LOWER: i8, const UPPER: i8> RangedF64<LOWER, UPPER> {
    pub const MAX: f64 = UPPER as f64;
    pub const MIN: f64 = LOWER as f64;
    pub const ZERO: f64 = 0.0;

    pub fn new(value: f64) -> Self {
        Self(value.clamp(Self::MIN, Self::MAX))
    }
    pub fn new_from_f32(value: f32) -> Self {
        Self::new(value as f64)
    }
    // These methods are annoying because they're inconsistent with the others
    // in this file. For example, StereoSample::MAX is a struct, not a
    // primitive. I think this happened because (1) a generic can't define a
    // constant like that -- which is reasonable -- but (2) I then defined
    // Normal/BipolarNormal etc. as old-style types, which meant I couldn't put
    // any consts inside them. TODO: try a new one of the newtype style, and
    // then take a afternoon converting the world to the new ones.
    pub const fn maximum() -> Self {
        Self(Self::MAX)
    }
    pub const fn minimum() -> Self {
        Self(Self::MIN)
    }
    pub const fn zero() -> Self {
        Self(Self::ZERO)
    }
    pub fn value(&self) -> f64 {
        self.0.clamp(Self::MIN, Self::MAX)
    }
    pub fn value_as_f32(&self) -> f32 {
        self.value() as f32
    }
    pub fn set(&mut self, value: f64) {
        self.0 = value.clamp(Self::MIN, Self::MAX);
    }

    pub fn scale(&self, factor: f64) -> f64 {
        self.0 * factor
    }
}
impl<const LOWER: i8, const UPPER: i8> Add for RangedF64<LOWER, UPPER> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}
impl<const LOWER: i8, const UPPER: i8> Sub for RangedF64<LOWER, UPPER> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}
impl<const LOWER: i8, const UPPER: i8> Add<f64> for RangedF64<LOWER, UPPER> {
    type Output = Self;

    fn add(self, rhs: f64) -> Self::Output {
        Self(self.0 + rhs)
    }
}
impl<const LOWER: i8, const UPPER: i8> Sub<f64> for RangedF64<LOWER, UPPER> {
    type Output = Self;

    fn sub(self, rhs: f64) -> Self::Output {
        Self(self.0 - rhs)
    }
}
impl<const LOWER: i8, const UPPER: i8> From<RangedF64<LOWER, UPPER>> for f64 {
    fn from(value: RangedF64<LOWER, UPPER>) -> Self {
        value.0.clamp(Self::MIN, Self::MAX)
    }
}
impl<const LOWER: i8, const UPPER: i8> From<f64> for RangedF64<LOWER, UPPER> {
    fn from(value: f64) -> Self {
        Self(value.clamp(Self::MIN, Self::MAX))
    }
}
impl<const LOWER: i8, const UPPER: i8> From<f32> for RangedF64<LOWER, UPPER> {
    fn from(value: f32) -> Self {
        Self(value.clamp(Self::MIN as f32, Self::MAX as f32) as f64)
    }
}

/// A Normal is a [RangedF64] whose range is [0.0, 1.0].
pub type Normal = RangedF64<0, 1>;

/// A BipolarNormal is a [RangedF64] whose range is [-1.0, 1.0].
pub type BipolarNormal = RangedF64<-1, 1>;

impl From<Sample> for Normal {
    // Sample -1.0..=1.0
    // Normal 0.0..=1.0
    fn from(value: Sample) -> Self {
        Self(value.0 * 0.5 + 0.5)
    }
}
impl From<BipolarNormal> for Normal {
    fn from(value: BipolarNormal) -> Self {
        Self(value.value() * 0.5 + 0.5)
    }
}
impl From<Sample> for BipolarNormal {
    // A [Sample] has the same range as a [BipolarNormal], so no conversion is
    // necessary.
    fn from(value: Sample) -> Self {
        Self(value.0)
    }
}
impl Mul<Normal> for BipolarNormal {
    type Output = BipolarNormal;

    fn mul(self, rhs: Normal) -> Self::Output {
        Self(self.0 * rhs.value())
    }
}
impl From<BipolarNormal> for StereoSample {
    fn from(value: BipolarNormal) -> Self {
        StereoSample::from(value.value())
    }
}
impl From<Normal> for BipolarNormal {
    fn from(value: Normal) -> Self {
        Self(value.value() * 2.0 - 1.0)
    }
}
impl From<FrequencyHz> for Normal {
    fn from(value: FrequencyHz) -> Self {
        FrequencyHz::frequency_to_percent(value.value())
    }
}
impl Into<FrequencyHz> for Normal {
    fn into(self) -> FrequencyHz {
        FrequencyHz::percent_to_frequency(self).into()
    }
}
impl Sub<Normal> for f64 {
    type Output = Self;

    fn sub(self, rhs: Normal) -> Self::Output {
        self - rhs.0
    }
}
impl Mul<Normal> for f64 {
    type Output = Self;

    fn mul(self, rhs: Normal) -> Self::Output {
        self * rhs.0
    }
}
impl Mul<f64> for Normal {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self(self.0 * rhs)
    }
}
impl Into<f32> for BipolarNormal {
    fn into(self) -> f32 {
        self.value_as_f32()
    }
}
impl Into<f32> for Normal {
    fn into(self) -> f32 {
        self.value_as_f32()
    }
}

/// The Digitally Controller Amplifier (DCA) handles gain and pan for many kinds
/// of synths.
///
/// See DSSPC++, Section 7.9 for requirements. TODO: implement
#[derive(Debug, Nano)]
pub struct Dca {
    #[nano]
    gain: Normal,
    #[nano]
    pan: BipolarNormal,
}
impl Dca {
    pub fn new_with(params: DcaNano) -> Self {
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
        StereoSample::new_from_f64(left_pan * input_sample, right_pan * input_sample)
    }

    pub fn gain(&self) -> RangedF64<0, 1> {
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
}

/// [FrequencyHz] is a frequency measured in
/// [Hertz](https://en.wikipedia.org/wiki/Hertz), or cycles per second. Because
/// we're usually discussing human hearing or LFOs, we can expect [FrequencyHz]
/// to range from about 0.0 to about 22,000.0. But because of
/// [aliasing](https://en.wikipedia.org/wiki/Nyquist_frequency), it's not
/// surprising to see 2x the upper range, which is where the 44.1KHz CD-quality
/// sampling rate comes from, and when we pick rendering rates, we might go up
/// to 192KHz (2x for sampling a 96KHz signal).
///
/// Eventually we might impose a non-negative restriction on this type.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct FrequencyHz(ParameterType);
impl FrequencyHz {
    pub const FREQUENCY_TO_LINEAR_BASE: ParameterType = 800.0;
    pub const FREQUENCY_TO_LINEAR_COEFFICIENT: ParameterType = 25.0;

    // https://docs.google.com/spreadsheets/d/1uQylh2h77-fuJ6OM0vjF7yjRXflLFP0yQEnv5wbaP2c/edit#gid=0
    // =LOGEST(Sheet1!B2:B23, Sheet1!A2:A23,true, false)
    //
    // Column A is 24db filter percentages from all the patches. Column B is
    // envelope-filter percentages from all the patches.
    pub fn percent_to_frequency(percentage: Normal) -> ParameterType {
        Self::FREQUENCY_TO_LINEAR_COEFFICIENT
            * Self::FREQUENCY_TO_LINEAR_BASE.powf(percentage.value())
    }

    pub fn frequency_to_percent(frequency: ParameterType) -> Normal {
        debug_assert!(frequency >= 0.0);

        // I was stressed out about slightly negative values, but then I decided
        // that adjusting the log numbers to handle more edge cases wasn't going
        // to make a practical difference. So I'm clamping to [0, 1].
        Normal::from(
            (frequency / Self::FREQUENCY_TO_LINEAR_COEFFICIENT).log(Self::FREQUENCY_TO_LINEAR_BASE),
        )
    }

    pub fn zero() -> Self {
        FrequencyHz(0.0)
    }

    pub fn value(&self) -> f64 {
        self.0
    }
}
impl From<f64> for FrequencyHz {
    fn from(value: f64) -> Self {
        Self(value)
    }
}
impl From<FrequencyHz> for f64 {
    fn from(value: FrequencyHz) -> Self {
        value.0
    }
}
impl From<f32> for FrequencyHz {
    fn from(value: f32) -> Self {
        Self(value as ParameterType)
    }
}
impl From<FrequencyHz> for f32 {
    fn from(value: FrequencyHz) -> Self {
        value.0 as f32
    }
}
impl Mul for FrequencyHz {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}
impl Mul<f64> for FrequencyHz {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self(self.0 * rhs)
    }
}
impl Mul<Ratio> for FrequencyHz {
    type Output = Self;

    fn mul(self, rhs: Ratio) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}
impl Div for FrequencyHz {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self(self.0 / rhs.0)
    }
}
impl From<usize> for FrequencyHz {
    fn from(value: usize) -> Self {
        Self(value as f64)
    }
}
impl From<FrequencyHz> for usize {
    fn from(value: FrequencyHz) -> Self {
        value.0 as usize
    }
}
impl Display for FrequencyHz {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

/// The [Ratio] type is a multiplier. A value of 2.0 would multiply another
/// value by two (a x 2.0:1.0), and a value of 0.5 would divide it by two (a x
/// 1.0:2.0 = a x 0.5).
///
/// Negative ratios are meaningless for current use cases.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Ratio(ParameterType);
impl Ratio {
    pub fn value(&self) -> ParameterType {
        self.0
    }
}
impl Default for Ratio {
    fn default() -> Self {
        Self(1.0)
    }
}
impl From<f64> for Ratio {
    fn from(value: f64) -> Self {
        Self(value)
    }
}
impl From<BipolarNormal> for Ratio {
    fn from(value: BipolarNormal) -> Self {
        Self(2.0f64.powf(value.value() * 3.0))
    }
}
impl From<Ratio> for BipolarNormal {
    fn from(value: Ratio) -> Self {
        BipolarNormal::from(value.value().log2() / 3.0)
    }
}
impl From<Normal> for Ratio {
    fn from(value: Normal) -> Self {
        Self::from(BipolarNormal::from(value))
    }
}
impl From<Ratio> for Normal {
    fn from(value: Ratio) -> Self {
        Self::from(BipolarNormal::from(value))
    }
}
impl From<f32> for Ratio {
    fn from(value: f32) -> Self {
        Self(value as ParameterType)
    }
}
impl From<F32ControlValue> for Ratio {
    fn from(value: F32ControlValue) -> Self {
        Self::from(Normal::from(value))
    }
}
impl From<Ratio> for F32ControlValue {
    fn from(value: Ratio) -> Self {
        F32ControlValue(Normal::from(value).value_as_f32())
    }
}
impl Mul<ParameterType> for Ratio {
    type Output = Self;

    fn mul(self, rhs: ParameterType) -> Self::Output {
        Ratio(self.0 * rhs)
    }
}
impl Div<ParameterType> for Ratio {
    type Output = Self;

    fn div(self, rhs: ParameterType) -> Self::Output {
        Ratio(self.0 / rhs)
    }
}
impl Mul<Ratio> for ParameterType {
    type Output = Self;

    fn mul(self, rhs: Ratio) -> Self::Output {
        self * rhs.0
    }
}
impl Div<Ratio> for ParameterType {
    type Output = Self;

    fn div(self, rhs: Ratio) -> Self::Output {
        self / rhs.0
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
        assert_eq!(Sample::from(StereoSample::MIN), Sample::MIN);
        assert_eq!(Sample::from(StereoSample::SILENCE), Sample::SILENCE);
        assert_eq!(Sample::from(StereoSample::MAX), Sample::MAX);

        assert_eq!(
            Sample::from(StereoSample::new_from_f64(1.0, 0.0)),
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
        let mut dca = Dca::new_with(DcaNano {
            gain: 1.0.into(),
            pan: BipolarNormal::zero(),
        });
        const VALUE_IN: Sample = Sample(0.5);
        const VALUE: f64 = 0.5;
        assert_eq!(
            dca.transform_audio_to_stereo(VALUE_IN),
            StereoSample::new_from_f64(VALUE * 0.75, VALUE * 0.75),
            "Pan center should give 75% equally to each channel"
        );

        dca.set_pan(BipolarNormal::new(-1.0));
        assert_eq!(
            dca.transform_audio_to_stereo(VALUE_IN),
            StereoSample::new_from_f64(VALUE, 0.0),
            "Pan left should give 100% to left channel"
        );

        dca.set_pan(BipolarNormal::new(1.0));
        assert_eq!(
            dca.transform_audio_to_stereo(VALUE_IN),
            StereoSample::new_from_f64(0.0, VALUE),
            "Pan right should give 100% to right channel"
        );
    }

    #[test]
    fn ratio_ok() {
        assert_eq!(Ratio::from(BipolarNormal::from(-1.0)).value(), 0.125);
        assert_eq!(Ratio::from(BipolarNormal::from(0.0)).value(), 1.0);
        assert_eq!(Ratio::from(BipolarNormal::from(1.0)).value(), 8.0);

        assert_eq!(BipolarNormal::from(Ratio::from(0.125)).value(), -1.0);
        assert_eq!(BipolarNormal::from(Ratio::from(1.0)).value(), 0.0);
        assert_eq!(BipolarNormal::from(Ratio::from(8.0)).value(), 1.0);
    }
}
