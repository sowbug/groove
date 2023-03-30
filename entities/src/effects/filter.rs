// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    traits::{IsEffect, TransformsAudio},
    FrequencyHz, Normal, ParameterType, Sample,
};
use groove_proc_macros::{Nano, Uid};
use std::{f64::consts::PI, str::FromStr};
use strum::EnumCount;
use strum_macros::{Display, EnumCount as EnumCountMacro, EnumString, FromRepr, IntoStaticStr};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default)]
pub enum FilterType {
    #[default]
    None,
    LowPass12db,
    LowPass24db,
    HighPass,
    BandPass,
    BandStop,
    AllPass,
    PeakingEq,
    LowShelf,
    HighShelf,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum FilterParams {
    #[default]
    None,
    LowPass12db {
        cutoff: ParameterType,
        q: f32,
    },
    LowPass24db {
        cutoff: ParameterType,
        passband_ripple: f32,
    },
    HighPass {
        cutoff: ParameterType,
        q: f32,
    },
    BandPass {
        cutoff: ParameterType,
        bandwidth: f32,
    },
    BandStop {
        cutoff: ParameterType,
        bandwidth: f32,
    },
    AllPass {
        cutoff: ParameterType,
        q: f32,
    },
    PeakingEq {
        cutoff: ParameterType,
        db_gain: f32,
    },
    LowShelf {
        cutoff: ParameterType,
        db_gain: f32,
    },
    HighShelf {
        cutoff: ParameterType,
        db_gain: f32,
    },
}

impl FilterParams {
    fn type_for(params: Self) -> FilterType {
        #[allow(unused_variables)]
        match params {
            FilterParams::None => FilterType::None,
            FilterParams::LowPass12db { cutoff, q } => FilterType::LowPass12db,
            FilterParams::LowPass24db {
                cutoff,
                passband_ripple: q,
            } => FilterType::LowPass24db,
            FilterParams::HighPass { cutoff, q } => FilterType::HighPass,
            FilterParams::BandPass { cutoff, bandwidth } => FilterType::BandPass,
            FilterParams::BandStop { cutoff, bandwidth } => FilterType::BandStop,
            FilterParams::AllPass { cutoff, q } => FilterType::AllPass,
            FilterParams::PeakingEq { cutoff, db_gain } => FilterType::PeakingEq,
            FilterParams::LowShelf { cutoff, db_gain } => FilterType::LowShelf,
            FilterParams::HighShelf { cutoff, db_gain } => FilterType::HighShelf,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct CoefficientSet {
    a0: f64,
    a1: f64,
    a2: f64,
    b0: f64,
    b1: f64,
    b2: f64,
}

#[derive(Clone, Debug, Default)]
struct CoefficientSet2 {
    // a3 isn't needed right now
    a4: f64,
    a5: f64,
    b3: f64,
    b4: f64,
    b5: f64,
}

/// <https://en.wikipedia.org/wiki/Digital_biquad_filter>
#[derive(Clone, Debug, Nano, Uid)]
pub struct BiQuadFilter {
    uid: usize,

    #[nano]
    cutoff: FrequencyHz,

    sample_rate: usize,
    filter_type: FilterType,

    // #[controllable(
    //     name = "q",
    //     name = "bandwidth",
    //     name = "db-gain",
    //     name = "passband-ripple"
    // )]
    // param2: f32,
    #[nano]
    q: ParameterType,

    coefficients: CoefficientSet,
    coefficients_2: CoefficientSet2,

    // Working variables
    sample_m1: f64, // "sample minus two" or x(n-2)
    sample_m2: f64,
    output_m1: f64,
    output_m2: f64,

    state_0: f64,
    state_1: f64,
    state_2: f64,
    state_3: f64,
}
impl IsEffect for BiQuadFilter {}
impl TransformsAudio for BiQuadFilter {
    fn transform_channel(&mut self, _channel: usize, input_sample: Sample) -> Sample {
        match self.filter_type {
            FilterType::LowPass24db => {
                // Thanks
                // https://www.musicdsp.org/en/latest/Filters/229-lpf-24db-oct.html
                let input = input_sample.0;
                let stage_1 = self.coefficients.b0 * input + self.state_0;
                self.state_0 =
                    self.coefficients.b1 * input + self.coefficients.a1 * stage_1 + self.state_1;
                self.state_1 = self.coefficients.b2 * input + self.coefficients.a2 * stage_1;
                let output = self.coefficients_2.b3 * stage_1 + self.state_2;
                self.state_2 = self.coefficients_2.b4 * stage_1
                    + self.coefficients_2.a4 * output
                    + self.state_3;
                self.state_3 = self.coefficients_2.b5 * stage_1 + self.coefficients_2.a5 * output;
                Sample::from(output)
            }
            _ => {
                let s64 = input_sample.0;
                let r = (self.coefficients.b0 / self.coefficients.a0) * s64
                    + (self.coefficients.b1 / self.coefficients.a0) * self.sample_m1
                    + (self.coefficients.b2 / self.coefficients.a0) * self.sample_m2
                    - (self.coefficients.a1 / self.coefficients.a0) * self.output_m1
                    - (self.coefficients.a2 / self.coefficients.a0) * self.output_m2;

                // Scroll everything forward in time.
                self.sample_m2 = self.sample_m1;
                self.sample_m1 = s64;

                self.output_m2 = self.output_m1;
                self.output_m1 = r;
                Sample::from(r)
            }
        }
    }
}
// We can't derive this because we need to call recalculate_coefficients(). Is
// there an elegant way to get that done for free without a bunch of repetition?
impl Default for BiQuadFilter {
    fn default() -> Self {
        let mut r = Self::default_fields();
        r.update_coefficients();
        r
    }
}

#[allow(dead_code)]
#[allow(unused_variables)]
impl BiQuadFilter {
    // A placeholder for an intelligent mapping of 0.0..=1.0 to a reasonable Q
    // range
    pub fn denormalize_q(value: Normal) -> ParameterType {
        value.value() * value.value() * 10.0 + 0.707
    }

    // A placeholder for an intelligent mapping of 0.0..=1.0 to a reasonable
    // 24db passband parameter range
    pub fn convert_passband(value: f32) -> f32 {
        value * 100.0 + 0.1
    }

    /// Returns a new/default struct without calling update_coefficients().
    fn default_fields() -> Self {
        Self {
            uid: usize::default(),
            filter_type: Default::default(),
            cutoff: Default::default(),
            q: Default::default(),
            sample_rate: Default::default(),
            coefficients: CoefficientSet::default(),
            coefficients_2: CoefficientSet2::default(),
            sample_m1: Default::default(),
            sample_m2: Default::default(),
            output_m1: Default::default(),
            output_m2: Default::default(),
            state_0: Default::default(),
            state_1: Default::default(),
            state_2: Default::default(),
            state_3: Default::default(),
        }
    }

    pub fn new_with(sample_rate: usize, params: BiQuadFilterNano) -> Self {
        let mut r = Self::default_fields();

        //        r.filter_type = FilterParams::type_for(*params);
        // TODO TODO TODO
        r.filter_type = FilterType::LowPass24db;
        r.sample_rate = sample_rate;
        r.cutoff = params.cutoff();
        r.q = params.q();
        // match *params {
        //     FilterParams::None => {}
        //     FilterParams::LowPass12db { cutoff, q } => {
        //         r.params.cutoff = cutoff;
        //         r.param2 = q;
        //     }
        //     FilterParams::LowPass24db {
        //         cutoff,
        //         passband_ripple,
        //     } => {
        //         r.params.cutoff = cutoff;
        //         r.param2 = passband_ripple;
        //     }
        //     FilterParams::HighPass { cutoff, q } => {
        //         r.params.cutoff = cutoff;
        //         r.param2 = q;
        //     }
        //     FilterParams::BandPass { cutoff, bandwidth } => {
        //         r.params.cutoff = cutoff;
        //         r.param2 = bandwidth;
        //     }
        //     FilterParams::BandStop { cutoff, bandwidth } => {
        //         r.params.cutoff = cutoff;
        //         r.param2 = bandwidth;
        //     }
        //     FilterParams::AllPass { cutoff, q } => {
        //         r.params.cutoff = cutoff;
        //         r.param2 = q;
        //     }
        //     FilterParams::PeakingEq { cutoff, db_gain } => {
        //         r.params.cutoff = cutoff;
        //         r.param2 = db_gain;
        //     }
        //     FilterParams::LowShelf { cutoff, db_gain } => {
        //         r.params.cutoff = cutoff;
        //         r.param2 = db_gain;
        //     }
        //     FilterParams::HighShelf { cutoff, db_gain } => {
        //         r.params.cutoff = cutoff;
        //         r.param2 = db_gain;
        //     }
        //}
        r.update_coefficients();
        r
    }

    fn update_coefficients(&mut self) {
        self.coefficients = match self.filter_type {
            FilterType::None => self.rbj_none_coefficients(),
            FilterType::LowPass12db => self.rbj_low_pass_coefficients(),
            FilterType::HighPass => self.rbj_high_pass_coefficients(),
            FilterType::BandPass => self.rbj_band_pass_coefficients(),
            FilterType::BandStop => self.rbj_band_stop_coefficients(),
            FilterType::AllPass => self.rbj_all_pass_coefficients(),
            FilterType::PeakingEq => self.rbj_peaking_eq_coefficients(),
            FilterType::LowShelf => self.rbj_low_shelf_coefficients(),
            FilterType::HighShelf => self.rbj_high_shelf_coefficients(),
            _ => self.rbj_none_coefficients(),
        };
        if matches!(self.filter_type, FilterType::LowPass24db) {
            let k = (PI * self.cutoff.value() / self.sample_rate as f64).tan();
            let p2 = self.q;
            let sg = p2.sinh();
            let cg = p2.cosh() * p2.cosh();

            let c0 = 1.0 / (cg - 0.853_553_390_593_273_7);
            let c1 = k * c0 * sg * 1.847_759_065_022_573_5;
            let c2 = 1.0 / (cg - 0.146_446_609_406_726_24);
            let c3 = k * c2 * sg * 0.765_366_864_730_179_6;
            let k = k * k;

            let a0 = 1.0 / (c1 + k + c0);
            let a1 = 2.0 * (c0 - k) * a0;
            let a2 = (c1 - k - c0) * a0;
            let b0 = a0 * k;
            let b1 = 2.0 * b0;
            let b2 = b0;
            self.coefficients = CoefficientSet {
                a0,
                a1,
                a2,
                b0,
                b1,
                b2,
            };

            let a3 = 1.0 / (c3 + k + c2);
            let a4 = 2.0 * (c2 - k) * a3;
            let a5 = (c3 - k - c2) * a3;
            let b3 = a3 * k;
            let b4 = 2.0 * b3;
            let b5 = b3;
            self.coefficients_2 = CoefficientSet2 { a4, a5, b3, b4, b5 };
        }
    }

    pub fn cutoff(&self) -> FrequencyHz {
        self.cutoff
    }

    pub(crate) fn set_cutoff(&mut self, hz: FrequencyHz) {
        if self.cutoff != hz {
            self.cutoff = hz;
            self.update_coefficients();
        }
    }

    pub fn cutoff_pct(&self) -> Normal {
        self.cutoff.into()
    }

    pub fn set_cutoff_pct(&mut self, percent: Normal) {
        self.set_cutoff(percent.into());
    }

    // pub fn set_param2(&mut self, value: f32) {
    //     if self.param2 != value {
    //         self.param2 = value;
    //         self.update_coefficients();
    //     }
    // }

    pub fn q(&self) -> ParameterType {
        self.q
    }
    pub fn set_q(&mut self, q: ParameterType) {
        if self.q != q {
            self.q = q;
            self.update_coefficients();
        }
    }

    // pub fn db_gain(&self) -> f32 {
    //     self.param2
    // }
    // pub fn set_db_gain(&mut self, db_gain: f32) {
    //     if self.param2 != db_gain {
    //         self.param2 = db_gain;
    //         self.update_coefficients();
    //     }
    // }

    // pub fn bandwidth(&self) -> f32 {
    //     self.param2
    // }
    // pub fn set_bandwidth(&mut self, bandwidth: f32) {
    //     if self.param2 != bandwidth {
    //         self.param2 = bandwidth;
    //         self.update_coefficients();
    //     }
    // }

    // /// Range is -1..1
    // pub fn passband_ripple(&self) -> f32 {
    //     self.param2
    // }
    // pub fn set_passband_ripple(&mut self, passband_ripple: f32) {
    //     if self.param2 != passband_ripple {
    //         self.param2 = passband_ripple;
    //         self.update_coefficients();
    //     }
    // }

    fn rbj_none_coefficients(&self) -> CoefficientSet {
        CoefficientSet {
            a0: 1.0,
            a1: 0.0,
            a2: 0.0,
            b0: 0.0,
            b1: 0.0,
            b2: 0.0,
        }
    }

    // Excerpted from Robert Bristow-Johnson's audio cookbook to explain various
    // parameters
    //
    // Fs (the sampling frequency)
    //
    // f0 ("wherever it's happenin', man."  Center Frequency or Corner
    //     Frequency, or shelf midpoint frequency, depending on which filter
    //     type.  The "significant frequency".)
    //
    // dBgain (used only for peaking and shelving filters)
    //
    // Q (the EE kind of definition, except for peakingEQ in which A*Q is the
    // classic EE Q.  That adjustment in definition was made so that a boost of
    // N dB followed by a cut of N dB for identical Q and f0/Fs results in a
    // precisely flat unity gain filter or "wire".)
    //
    // - _or_ BW, the bandwidth in octaves (between -3 dB frequencies for BPF
    //     and notch or between midpoint (dBgain/2) gain frequencies for peaking
    //     EQ)
    //
    // - _or_ S, a "shelf slope" parameter (for shelving EQ only).  When S = 1,
    //     the shelf slope is as steep as it can be and remain monotonically
    //     increasing or decreasing gain with frequency.  The shelf slope, in
    //     dB/octave, remains proportional to S for all other values for a fixed
    //     f0/Fs and dBgain.

    fn rbj_intermediates_q(
        sample_rate: usize,
        cutoff: ParameterType,
        q: ParameterType,
    ) -> (f64, f64, f64, f64) {
        let w0 = 2.0f64 * PI * cutoff / sample_rate as f64;
        let w0cos = w0.cos();
        let w0sin = w0.sin();
        let alpha = w0sin / (2.0f64 * q);
        (w0, w0cos, w0sin, alpha)
    }

    fn rbj_low_pass_coefficients(&self) -> CoefficientSet {
        let (w0, w0cos, w0sin, alpha) =
            Self::rbj_intermediates_q(self.sample_rate, self.cutoff.value(), self.q);

        CoefficientSet {
            a0: 1.0 + alpha,
            a1: -2.0f64 * w0cos,
            a2: 1.0 - alpha,
            b0: (1.0 - w0cos) / 2.0f64,
            b1: (1.0 - w0cos),
            b2: (1.0 - w0cos) / 2.0f64,
        }
    }

    fn rbj_high_pass_coefficients(&self) -> CoefficientSet {
        let (w0, w0cos, w0sin, alpha) =
            Self::rbj_intermediates_q(self.sample_rate, self.cutoff.value(), self.q);

        CoefficientSet {
            a0: 1.0 + alpha,
            a1: -2.0f64 * w0cos,
            a2: 1.0 - alpha,
            b0: (1.0 + w0cos) / 2.0f64,
            b1: -(1.0 + w0cos),
            b2: (1.0 + w0cos) / 2.0f64,
        }
    }

    fn rbj_intermediates_bandwidth(
        sample_rate: usize,
        cutoff: ParameterType,
        bw: ParameterType,
    ) -> (f64, f64, f64, f64) {
        let w0 = 2.0f64 * PI * cutoff / sample_rate as f64;
        let w0cos = w0.cos();
        let w0sin = w0.sin();
        let alpha = w0sin * (2.0f64.ln() / 2.0 * bw as f64 * w0 / w0.sin()).sinh();
        (w0, w0cos, w0sin, alpha)
    }

    fn rbj_band_pass_coefficients(&self) -> CoefficientSet {
        let (w0, w0cos, w0sin, alpha) =
            Self::rbj_intermediates_bandwidth(self.sample_rate, self.cutoff.value(), self.q);
        CoefficientSet {
            a0: 1.0 + alpha,
            a1: -2.0f64 * w0cos,
            a2: 1.0 - alpha,
            b0: alpha,
            b1: 0.0,
            b2: -alpha,
        }
    }

    fn rbj_band_stop_coefficients(&self) -> CoefficientSet {
        let (w0, w0cos, w0sin, alpha) =
            Self::rbj_intermediates_bandwidth(self.sample_rate, self.cutoff.value(), self.q);

        CoefficientSet {
            a0: 1.0 + alpha,
            a1: -2.0f64 * w0cos,
            a2: 1.0 - alpha,
            b0: 1.0,
            b1: -2.0f64 * w0cos,
            b2: 1.0,
        }
    }

    fn rbj_all_pass_coefficients(&self) -> CoefficientSet {
        let (w0, w0cos, w0sin, alpha) =
            Self::rbj_intermediates_q(self.sample_rate, self.cutoff.value(), self.q);
        CoefficientSet {
            a0: 1.0 + alpha,
            a1: -2.0f64 * w0cos,
            a2: 1.0 - alpha,
            b0: 1.0 - alpha,
            b1: -2.0f64 * w0cos,
            b2: 1.0 + alpha,
        }
    }

    fn rbj_peaking_eq_coefficients(&self) -> CoefficientSet {
        let (w0, w0cos, w0sin, alpha) = Self::rbj_intermediates_q(
            self.sample_rate,
            self.cutoff.value(),
            std::f64::consts::FRAC_1_SQRT_2,
        );
        let a = 10f64.powf(self.q / 10.0f64).sqrt();

        CoefficientSet {
            a0: 1.0 + alpha / a,
            a1: -2.0f64 * w0cos,
            a2: 1.0 - alpha / a,
            b0: 1.0 + alpha * a,
            b1: -2.0f64 * w0cos,
            b2: 1.0 - alpha * a,
        }
    }

    fn rbj_intermediates_shelving(
        sample_rate: usize,
        cutoff: ParameterType,
        a: f64,
        s: f32,
    ) -> (f64, f64, f64, f64) {
        let w0 = 2.0f64 * PI * cutoff as f64 / sample_rate as f64;
        let w0cos = w0.cos();
        let w0sin = w0.sin();
        let alpha = w0sin / 2.0 * ((a + 1.0 / a) * (1.0 / s as f64 - 1.0) + 2.0).sqrt();
        (w0, w0cos, w0sin, alpha)
    }

    fn rbj_low_shelf_coefficients(&self) -> CoefficientSet {
        let a = 10f64.powf(self.q / 10.0f64).sqrt();
        let (_w0, w0cos, _w0sin, alpha) =
            BiQuadFilter::rbj_intermediates_shelving(self.sample_rate, self.cutoff.value(), a, 1.0);

        CoefficientSet {
            a0: (a + 1.0) + (a - 1.0) * w0cos + 2.0 * a.sqrt() * alpha,
            a1: -2.0 * ((a - 1.0) + (a + 1.0) * w0cos),
            a2: (a + 1.0) + (a - 1.0) * w0cos - 2.0 * a.sqrt() * alpha,
            b0: a * ((a + 1.0) - (a - 1.0) * w0cos + 2.0 * a.sqrt() * alpha),
            b1: 2.0 * a * ((a - 1.0) - (a + 1.0) * w0cos),
            b2: a * ((a + 1.0) - (a - 1.0) * w0cos - 2.0 * a.sqrt() * alpha),
        }
    }

    fn rbj_high_shelf_coefficients(&self) -> CoefficientSet {
        let a = 10f64.powf(self.q / 10.0f64).sqrt();
        let (_w0, w0cos, _w0sin, alpha) =
            BiQuadFilter::rbj_intermediates_shelving(self.sample_rate, self.cutoff.value(), a, 1.0);

        CoefficientSet {
            a0: (a + 1.0) - (a - 1.0) * w0cos + 2.0 * a.sqrt() * alpha,
            a1: 2.0 * ((a - 1.0) - (a + 1.0) * w0cos),
            a2: (a + 1.0) - (a - 1.0) * w0cos - 2.0 * a.sqrt() * alpha,
            b0: a * ((a + 1.0) + (a - 1.0) * w0cos + 2.0 * a.sqrt() * alpha),
            b1: -2.0 * a * ((a - 1.0) + (a + 1.0) * w0cos),
            b2: a * ((a + 1.0) + (a - 1.0) * w0cos - 2.0 * a.sqrt() * alpha),
        }
    }

    pub fn update(&mut self, message: BiQuadFilterMessage) {
        match message {
            BiQuadFilterMessage::BiQuadFilter(e) => *self = Self::new_with(self.sample_rate, e),
            BiQuadFilterMessage::Cutoff(cutoff) => self.set_cutoff(cutoff),
            BiQuadFilterMessage::Q(q) => self.set_q(q.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    // TODO: get FFT working, and then write tests.
}
