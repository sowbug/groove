use std::f64::consts::PI;

use crate::common::MonoSample;

use super::EffectTrait__;

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum MiniFilterType {
    None,
    FirstOrderLowPass(f32),
    FirstOrderHighPass(f32),
    SecondOrderLowPass(f32, f32),
    SecondOrderHighPass(f32, f32),
    SecondOrderBandPass(f32, f32),
    SecondOrderBandStop(f32, f32),
    FourthOrderLowPass(f32),
    FourthOrderHighPass(f32),
    // Not sure Butterworth filters are worth implementing. Pirkle says they're very similar to second-order.
    // SecondOrderButterworthLowPass,
    // SecondOrderButterworthHighPass,
    // SecondOrderButterworthBandPass,
    // SecondOrderButterworthBandStop,
}

impl Default for MiniFilterType {
    fn default() -> Self {
        MiniFilterType::None
    }
}

#[derive(Default)]
pub struct MiniFilter {
    order: u8,
    a0: f64,
    a1: f64,
    a2: f64,
    a3: f64,
    a4: f64,
    b1: f64,
    b2: f64,
    b3: f64,
    b4: f64,
    c0: f64,
    d0: f64,
    sample_m1: f64, // "sample minus two" or x(n-2)
    sample_m2: f64,
    sample_m3: f64,
    sample_m4: f64,
    output_m1: f64,
    output_m2: f64,
    output_m3: f64,
    output_m4: f64,
}

#[allow(dead_code)]
impl MiniFilter {
    pub fn new(sample_rate: usize, filter_type: MiniFilterType) -> Self {
        let (order, a0, a1, a2, a3, a4, b1, b2, b3, b4, c0, d0) = match filter_type {
            MiniFilterType::None => (2, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0),
            MiniFilterType::FirstOrderLowPass(cutoff) => {
                Self::first_order_low_pass_coefficients(sample_rate, cutoff)
            }
            MiniFilterType::FirstOrderHighPass(cutoff) => {
                Self::first_order_high_pass_coefficients(sample_rate, cutoff)
            }
            MiniFilterType::SecondOrderLowPass(cutoff, q) => {
                Self::second_order_low_pass_coefficients(sample_rate, cutoff, q)
            }
            MiniFilterType::SecondOrderHighPass(cutoff, q) => {
                Self::second_order_high_pass_coefficients(sample_rate, cutoff, q)
            }
            MiniFilterType::SecondOrderBandPass(cutoff, q) => {
                Self::second_order_band_pass_coefficients(sample_rate, cutoff, q)
            }
            MiniFilterType::SecondOrderBandStop(cutoff, q) => {
                Self::second_order_band_stop_coefficients(sample_rate, cutoff, q)
            }
            MiniFilterType::FourthOrderLowPass(cutoff) => {
                Self::fourth_order_linkwitz_riley_low_pass_coefficients(sample_rate, cutoff)
            }
            MiniFilterType::FourthOrderHighPass(cutoff) => {
                Self::fourth_order_linkwitz_riley_high_pass_coefficients(sample_rate, cutoff)
            }
        };
        Self {
            order,
            a0,
            a1,
            a2,
            a3,
            a4,
            b1,
            b2,
            b3,
            b4,
            c0,
            d0,
            ..Default::default()
        }
    }

    fn first_order_low_pass_coefficients(
        sample_rate: usize,
        cutoff: f32,
    ) -> (u8, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64) {
        let theta_c = 2.0f64 * PI * cutoff as f64 / (sample_rate as f64);
        let gamma = theta_c.cos() / (1.0 + theta_c.sin());
        let alpha = (1.0 - gamma) / 2.0;

        (
            1, alpha, alpha, 0.0, 0.0, 0.0, -gamma, 0.0, 0.0, 0.0, 1.0, 0.0,
        )
    }

    fn first_order_high_pass_coefficients(
        sample_rate: usize,
        cutoff: f32,
    ) -> (u8, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64) {
        let theta_c = 2.0 * PI * cutoff as f64 / (sample_rate as f64);
        let gamma = theta_c.cos() / (1.0 + theta_c.sin());
        let alpha = (1.0 + gamma) / 2.0;
        (
            1, alpha, -alpha, 0.0, 0.0, 0.0, -gamma, 0.0, 0.0, 0.0, 1.0, 0.0,
        )
    }

    fn common_second_order_coefficients(sample_rate: usize, cutoff: f32, q: f32) -> (f64, f64) {
        let theta_c = 2.0 * PI * cutoff as f64 / (sample_rate as f64);
        let delta = 1.0 / (q as f64).max(std::f64::consts::FRAC_1_SQRT_2);
        let beta_n = 1.0 - ((delta / 2.0) * theta_c.sin());
        let beta_d = 1.0 + ((delta / 2.0) * theta_c.sin());
        let beta = 0.5 * (beta_n / beta_d);
        let gamma = (0.5 + beta) * (theta_c.cos());
        (beta, gamma)
    }

    // See Will C. Pirkle's _Designing Audio Effects In C++_ for coefficient sources.
    //
    // In my testing, this behaves identically when (noise, 500Hz, q=0.707) to Audacity's 12db LPF.
    fn second_order_low_pass_coefficients(
        sample_rate: usize,
        cutoff: f32,
        q: f32,
    ) -> (u8, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64) {
        let (beta, gamma) = Self::common_second_order_coefficients(sample_rate, cutoff, q);
        let alpha_n = 0.5 + beta - gamma;

        (
            2,
            alpha_n / 2.0,
            alpha_n,
            alpha_n / 2.0,
            0.0,
            0.0,
            -2.0 * gamma,
            2.0 * beta,
            0.0,
            0.0,
            1.0,
            0.0,
        )
    }

    fn fourth_order_linkwitz_riley_low_pass_coefficients(
        sample_rate: usize,
        cutoff: f32,
    ) -> (u8, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64) {
        let omega = 2.0 * PI * cutoff as f64;
        let omega2 = omega * omega;
        let omega3 = omega2 * omega;
        let omega4 = omega2 * omega2;
        let kappa = omega / (PI * cutoff as f64 / sample_rate as f64).tan();
        let kappa2 = kappa * kappa;
        let kappa3 = kappa2 * kappa;
        let kappa4 = kappa2 * kappa2;
        let sq_tmp1 = std::f64::consts::SQRT_2 * omega3 * kappa;
        let sq_tmp2 = std::f64::consts::SQRT_2 * omega * kappa3;
        let a_tmp = 4.0 * omega2 * kappa2 + 2.0 * sq_tmp1 + kappa4 + 2.0 * sq_tmp2 + omega4;

        let a0 = omega4 / a_tmp;
        let a1 = 4.0 * omega4 / a_tmp;
        let a2 = 6.0 * omega4 / a_tmp;
        let a3 = a1;
        let a4 = a0;
        let b1 = (4.0 * (omega4 + sq_tmp1 - kappa4 - sq_tmp2)) / a_tmp;
        let b2 = (6.0 * omega4 - 8.0 * omega2 * kappa2 + 6.0 * kappa4) / a_tmp;
        let b3 = (4.0 * (omega4 - sq_tmp1 + sq_tmp2 - kappa4)) / a_tmp;
        let b4 = (kappa4 - 2.0 * sq_tmp1 + omega4 - 2.0 * sq_tmp2 + 4.0 * omega2 * kappa2) / a_tmp;

        (4, a0, a1, a2, a3, a4, b1, b2, b3, b4, 1.0, 0.0)
    }

    fn fourth_order_linkwitz_riley_high_pass_coefficients(
        sample_rate: usize,
        cutoff: f32,
    ) -> (u8, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64) {
        let omega = 2.0 * PI * cutoff as f64;
        let omega2 = omega * omega;
        let omega3 = omega2 * omega;
        let omega4 = omega2 * omega2;
        let kappa = omega / (PI * cutoff as f64 / sample_rate as f64).tan();
        let kappa2 = kappa * kappa;
        let kappa3 = kappa2 * kappa;
        let kappa4 = kappa2 * kappa2;
        let sq_tmp1 = std::f64::consts::SQRT_2 * omega3 * kappa;
        let sq_tmp2 = std::f64::consts::SQRT_2 * omega * kappa3;
        let a_tmp = 4.0 * omega2 * kappa2 + 2.0 * sq_tmp1 + kappa4 + 2.0 * sq_tmp2 + omega4;

        let a0 = kappa4 / a_tmp;
        let a1 = -4.0 * kappa4 / a_tmp;
        let a2 = 6.0 * kappa4 / a_tmp;
        let a3 = a1;
        let a4 = a0;
        let b1 = (4.0 * (omega4 + sq_tmp1 - kappa4 - sq_tmp2)) / a_tmp;
        let b2 = (6.0 * omega4 - 8.0 * omega2 * kappa2 + 6.0 * kappa4) / a_tmp;
        let b3 = (4.0 * (omega4 - sq_tmp1 + sq_tmp2 - kappa4)) / a_tmp;
        let b4 = (kappa4 - 2.0 * sq_tmp1 + omega4 - 2.0 * sq_tmp2 + 4.0 * omega2 * kappa2) / a_tmp;

        (4, a0, a1, a2, a3, a4, b1, b2, b3, b4, 1.0, 0.0)
    }

    fn second_order_high_pass_coefficients(
        sample_rate: usize,
        cutoff: f32,
        q: f32,
    ) -> (u8, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64) {
        let (beta, gamma) = Self::common_second_order_coefficients(sample_rate, cutoff, q);
        let alpha_n = 0.5 + beta + gamma;

        (
            2,
            alpha_n / 2.0,
            -alpha_n,
            alpha_n / 2.0,
            0.0,
            0.0,
            -2.0 * gamma,
            2.0 * beta,
            0.0,
            0.0,
            1.0,
            0.0,
        )
    }
    fn second_order_band_pass_coefficients(
        sample_rate: usize,
        cutoff: f32,
        q: f32,
    ) -> (u8, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64) {
        let q64 = q as f64;
        let kappa = (PI * cutoff as f64 / sample_rate as f64).tan();
        let kappa_sq = kappa.powi(2);
        let delta = kappa_sq * q64 + kappa + q64;

        (
            2,
            kappa / delta,
            0.0,
            -kappa / delta,
            0.0,
            0.0,
            (2.0 * q as f64 * (kappa_sq - 1.0)) / delta,
            (kappa_sq * q as f64 - kappa + q as f64) / delta,
            0.0,
            0.0,
            1.0,
            0.0,
        )
    }
    fn second_order_band_stop_coefficients(
        sample_rate: usize,
        cutoff: f32,
        q: f32,
    ) -> (u8, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64) {
        let q64 = q as f64;
        let kappa = (PI * cutoff as f64 / sample_rate as f64).tan();
        let kappa_sq = kappa.powi(2);
        let delta = kappa_sq * q64 + kappa + q64;

        let alpha_a = (q64 * (kappa_sq + 1.0)) / delta;
        let alpha_b = (2.0 * q64 * (kappa_sq - 1.0)) / delta;
        (
            2,
            alpha_a,
            alpha_b,
            alpha_a,
            0.0,
            0.0,
            alpha_b,
            (kappa_sq * q64 - kappa + q64) / delta,
            0.0,
            0.0,
            1.0,
            0.0,
        )
    }
}

impl EffectTrait__ for MiniFilter {
    fn process(&mut self, input: MonoSample, _time_seconds: f32) -> MonoSample {
        let s64 = input as f64;
        let r = match self.order {
            0 => 0.,
            1 => {
                let result = self.d0 * s64
                    + self.c0
                        * (self.a0 * s64 + self.a1 * self.sample_m1 + self.a2 * self.sample_m2
                            - self.b1 * self.output_m1
                            - self.b2 * self.output_m2);

                // Scroll everything forward in time.
                self.sample_m2 = self.sample_m1;
                self.sample_m1 = s64;
                self.output_m2 = self.output_m1;
                self.output_m1 = result;
                result
            }
            2 => {
                let result = self.d0 * s64
                    + self.c0
                        * (self.a0 * s64 + self.a1 * self.sample_m1 + self.a2 * self.sample_m2
                            - self.b1 * self.output_m1
                            - self.b2 * self.output_m2);

                // Scroll everything forward in time.
                self.sample_m2 = self.sample_m1;
                self.sample_m1 = s64;
                self.output_m2 = self.output_m1;
                self.output_m1 = result;
                result
            }
            3 => {
                panic!("no such order");
            }
            4 => {
                let result = self.d0 * s64
                    + self.c0
                        * (self.a0 * s64
                            + self.a1 * self.sample_m1
                            + self.a2 * self.sample_m2
                            + self.a3 * self.sample_m3
                            + self.a4 * self.sample_m4
                            - self.b1 * self.output_m1
                            - self.b2 * self.output_m2
                            - self.b3 * self.output_m3
                            - self.b4 * self.output_m4);

                // Scroll everything forward in time.
                self.sample_m4 = self.sample_m3;
                self.sample_m3 = self.sample_m2;
                self.sample_m2 = self.sample_m1;
                self.sample_m1 = s64;

                self.output_m4 = self.output_m3;
                self.output_m3 = self.output_m2;
                self.output_m2 = self.output_m1;
                self.output_m1 = result;
                result
            }
            _ => {
                panic!("impossible");
            }
        };
        r as MonoSample
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MiniFilter2Type {
    None,
    LowPass {
        sample_rate: usize,
        cutoff: f32,
        q: f32,
    },
    HighPass {
        sample_rate: usize,
        cutoff: f32,
        q: f32,
    },
    BandPass {
        sample_rate: usize,
        cutoff: f32,
        bandwidth: f32,
    },
    BandStop {
        sample_rate: usize,
        cutoff: f32,
        bandwidth: f32,
    },
    AllPass {
        sample_rate: usize,
        cutoff: f32,
        q: f32,
    },
    PeakingEq {
        sample_rate: usize,
        cutoff: f32,
        db_gain: f32,
    },
    LowShelf {
        sample_rate: usize,
        cutoff: f32,
        db_gain: f32,
    },
    HighShelf {
        sample_rate: usize,
        cutoff: f32,
        db_gain: f32,
    },
}

impl Default for MiniFilter2Type {
    fn default() -> Self {
        MiniFilter2Type::None
    }
}

#[derive(Debug, Default)]
pub struct MiniFilter2 {
    filter_type: MiniFilter2Type,
    sample_rate: usize,
    cutoff: f32,
    a0: f64,
    a1: f64,
    a2: f64,
    b0: f64,
    b1: f64,
    b2: f64,
    sample_m1: f64, // "sample minus two" or x(n-2)
    sample_m2: f64,
    output_m1: f64,
    output_m2: f64,
}

#[allow(dead_code)]
#[allow(unused_variables)]
impl MiniFilter2 {
    pub const FREQUENCY_TO_LINEAR_BASE: f32 = 800.0;
    pub const FREQUENCY_TO_LINEAR_COEFFICIENT: f32 = 25.0;

    // https://docs.google.com/spreadsheets/d/1uQylh2h77-fuJ6OM0vjF7yjRXflLFP0yQEnv5wbaP2c/edit#gid=0
    // =LOGEST(Sheet1!B2:B23, Sheet1!A2:A23,true, false)
    // Column A is 24db filter percentages from all the patches
    // Column B is envelope-filter percentages from all the patches
    pub fn percent_to_frequency(percentage: f32) -> f32 {
        Self::FREQUENCY_TO_LINEAR_BASE * Self::FREQUENCY_TO_LINEAR_COEFFICIENT.powf(percentage)
    }

    pub fn frequency_to_percent(frequency: f32) -> f32 {
        (frequency / Self::FREQUENCY_TO_LINEAR_COEFFICIENT).log(Self::FREQUENCY_TO_LINEAR_BASE)
    }

    pub fn new(filter_type: &MiniFilter2Type) -> Self {
        let mut r = Self {
            ..Default::default()
        };
        r.recalculate_coefficients(filter_type);
        r
    }

    fn recalculate_coefficients(&mut self, new_filter_type: &MiniFilter2Type) {
        (self.a0, self.a1, self.a2, self.b0, self.b1, self.b2) = match *new_filter_type {
            MiniFilter2Type::None => (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            MiniFilter2Type::LowPass {
                sample_rate,
                cutoff,
                q,
            } => {
                self.sample_rate = sample_rate;
                self.cutoff = cutoff;
                Self::rbj_low_pass_coefficients(sample_rate, cutoff, q)
            }
            MiniFilter2Type::HighPass {
                sample_rate,
                cutoff,
                q,
            } => {
                self.sample_rate = sample_rate;
                self.cutoff = cutoff;
                Self::rbj_high_pass_coefficients(sample_rate, cutoff, q)
            }
            MiniFilter2Type::BandPass {
                sample_rate,
                cutoff,
                bandwidth,
            } => {
                self.sample_rate = sample_rate;
                self.cutoff = cutoff;
                Self::rbj_band_pass_coefficients(sample_rate, cutoff, bandwidth)
            }
            MiniFilter2Type::BandStop {
                sample_rate,
                cutoff,
                bandwidth,
            } => {
                self.sample_rate = sample_rate;
                self.cutoff = cutoff;
                Self::rbj_band_stop_coefficients(sample_rate, cutoff, bandwidth)
            }
            MiniFilter2Type::AllPass {
                sample_rate,
                cutoff,
                q,
            } => {
                self.sample_rate = sample_rate;
                self.cutoff = cutoff;
                Self::rbj_all_pass_coefficients(sample_rate, cutoff, q)
            }
            MiniFilter2Type::PeakingEq {
                sample_rate,
                cutoff,
                db_gain,
            } => {
                self.sample_rate = sample_rate;
                self.cutoff = cutoff;
                Self::rbj_peaking_eq_coefficients(sample_rate, cutoff, db_gain)
            }
            MiniFilter2Type::LowShelf {
                sample_rate,
                cutoff,
                db_gain,
            } => {
                self.sample_rate = sample_rate;
                self.cutoff = cutoff;
                Self::rbj_low_shelf_coefficients(sample_rate, cutoff, db_gain)
            }
            MiniFilter2Type::HighShelf {
                sample_rate,
                cutoff,
                db_gain,
            } => {
                self.sample_rate = sample_rate;
                self.cutoff = cutoff;
                Self::rbj_high_shelf_coefficients(sample_rate, cutoff, db_gain)
            }
        };
        self.filter_type = *new_filter_type;
    }

    pub fn set_cutoff(&mut self, new_cutoff: f32) {
        let new_filter_type = match self.filter_type {
            MiniFilter2Type::None => MiniFilter2Type::None,
            MiniFilter2Type::LowPass {
                sample_rate,
                cutoff,
                q,
            } => MiniFilter2Type::LowPass {
                sample_rate,
                cutoff: new_cutoff,
                q,
            },
            MiniFilter2Type::HighPass {
                sample_rate,
                cutoff,
                q,
            } => MiniFilter2Type::LowPass {
                sample_rate,
                cutoff: new_cutoff,
                q,
            },
            MiniFilter2Type::BandPass {
                sample_rate,
                cutoff,
                bandwidth,
            } => MiniFilter2Type::BandPass {
                sample_rate,
                cutoff: new_cutoff,
                bandwidth,
            },
            MiniFilter2Type::BandStop {
                sample_rate,
                cutoff,
                bandwidth,
            } => MiniFilter2Type::BandStop {
                sample_rate,
                cutoff: new_cutoff,
                bandwidth,
            },
            MiniFilter2Type::AllPass {
                sample_rate,
                cutoff,
                q,
            } => MiniFilter2Type::AllPass {
                sample_rate,
                cutoff: new_cutoff,
                q,
            },
            MiniFilter2Type::PeakingEq {
                sample_rate,
                cutoff,
                db_gain,
            } => MiniFilter2Type::PeakingEq {
                sample_rate,
                cutoff: new_cutoff,
                db_gain,
            },
            MiniFilter2Type::LowShelf {
                sample_rate,
                cutoff,
                db_gain,
            } => MiniFilter2Type::LowShelf {
                sample_rate,
                cutoff: new_cutoff,
                db_gain,
            },
            MiniFilter2Type::HighShelf {
                sample_rate,
                cutoff,
                db_gain,
            } => MiniFilter2Type::HighShelf {
                sample_rate,
                cutoff: new_cutoff,
                db_gain,
            },
        };
        self.recalculate_coefficients(&new_filter_type)
    }

    pub fn set_q(&mut self, new_val: f32) {
        let new_filter_type = match self.filter_type {
            MiniFilter2Type::None => MiniFilter2Type::None,
            MiniFilter2Type::LowPass {
                sample_rate,
                cutoff,
                q,
            } => MiniFilter2Type::LowPass {
                sample_rate,
                cutoff,
                q: new_val,
            },
            MiniFilter2Type::HighPass {
                sample_rate,
                cutoff,
                q,
            } => MiniFilter2Type::LowPass {
                sample_rate,
                cutoff,
                q: new_val,
            },
            MiniFilter2Type::BandPass {
                sample_rate,
                cutoff,
                bandwidth,
            } => MiniFilter2Type::BandPass {
                sample_rate,
                cutoff,
                bandwidth: new_val,
            },
            MiniFilter2Type::BandStop {
                sample_rate,
                cutoff,
                bandwidth,
            } => MiniFilter2Type::BandStop {
                sample_rate,
                cutoff,
                bandwidth: new_val,
            },
            MiniFilter2Type::AllPass {
                sample_rate,
                cutoff,
                q,
            } => MiniFilter2Type::AllPass {
                sample_rate,
                cutoff,
                q: new_val,
            },
            MiniFilter2Type::PeakingEq {
                sample_rate,
                cutoff,
                db_gain,
            } => MiniFilter2Type::PeakingEq {
                sample_rate,
                cutoff,
                db_gain: new_val,
            },
            MiniFilter2Type::LowShelf {
                sample_rate,
                cutoff,
                db_gain,
            } => MiniFilter2Type::LowShelf {
                sample_rate,
                cutoff,
                db_gain: new_val,
            },
            MiniFilter2Type::HighShelf {
                sample_rate,
                cutoff,
                db_gain,
            } => MiniFilter2Type::HighShelf {
                sample_rate,
                cutoff,
                db_gain: new_val,
            },
        };
        self.recalculate_coefficients(&new_filter_type)
    }

    fn rbj_intermediates_q(sample_rate: usize, cutoff: f32, q: f32) -> (f64, f64, f64, f64) {
        let w0 = 2.0f64 * PI * cutoff as f64 / sample_rate as f64;
        let w0cos = w0.cos();
        let w0sin = w0.sin();
        let alpha = w0sin / (2.0f64 * q as f64);
        (w0, w0cos, w0sin, alpha)
    }

    fn rbj_low_pass_coefficients(
        sample_rate: usize,
        cutoff: f32,
        q: f32,
    ) -> (f64, f64, f64, f64, f64, f64) {
        let (w0, w0cos, w0sin, alpha) = MiniFilter2::rbj_intermediates_q(sample_rate, cutoff, q);

        (
            1.0 + alpha,
            -2.0f64 * w0cos,
            1.0 - alpha,
            (1.0 - w0cos) / 2.0f64,
            (1.0 - w0cos),
            (1.0 - w0cos) / 2.0f64,
        )
    }

    fn rbj_high_pass_coefficients(
        sample_rate: usize,
        cutoff: f32,
        q: f32,
    ) -> (f64, f64, f64, f64, f64, f64) {
        let (w0, w0cos, w0sin, alpha) = MiniFilter2::rbj_intermediates_q(sample_rate, cutoff, q);

        (
            1.0 + alpha,
            -2.0f64 * w0cos,
            1.0 - alpha,
            (1.0 + w0cos) / 2.0f64,
            -(1.0 + w0cos),
            (1.0 + w0cos) / 2.0f64,
        )
    }

    fn rbj_intermediates_bandwidth(sample_rate: usize, cutoff: f32, bw: f32) -> (f64, f64, f64, f64) {
        let w0 = 2.0f64 * PI * cutoff as f64 / sample_rate as f64;
        let w0cos = w0.cos();
        let w0sin = w0.sin();
        let alpha = w0sin * (2.0f64.ln() / 2.0 * bw as f64 * w0 / w0.sin()).sinh();
        (w0, w0cos, w0sin, alpha)
    }

    fn rbj_band_pass_coefficients(
        sample_rate: usize,
        cutoff: f32,
        bandwidth: f32,
    ) -> (f64, f64, f64, f64, f64, f64) {
        let (w0, w0cos, w0sin, alpha) =
            MiniFilter2::rbj_intermediates_bandwidth(sample_rate, cutoff, bandwidth);
        (
            1.0 + alpha,
            -2.0f64 * w0cos,
            1.0 - alpha,
            alpha,
            0.0,
            -alpha,
        )
    }

    fn rbj_band_stop_coefficients(
        sample_rate: usize,
        cutoff: f32,
        bandwidth: f32,
    ) -> (f64, f64, f64, f64, f64, f64) {
        let (w0, w0cos, w0sin, alpha) =
            MiniFilter2::rbj_intermediates_bandwidth(sample_rate, cutoff, bandwidth);

        (
            1.0 + alpha,
            -2.0f64 * w0cos,
            1.0 - alpha,
            1.0,
            -2.0f64 * w0cos,
            1.0,
        )
    }

    fn rbj_all_pass_coefficients(
        sample_rate: usize,
        cutoff: f32,
        q: f32,
    ) -> (f64, f64, f64, f64, f64, f64) {
        let (w0, w0cos, w0sin, alpha) = MiniFilter2::rbj_intermediates_q(sample_rate, cutoff, q);
        (
            1.0 + alpha,
            -2.0f64 * w0cos,
            1.0 - alpha,
            1.0 - alpha,
            -2.0f64 * w0cos,
            1.0 + alpha,
        )
    }

    fn rbj_peaking_eq_coefficients(
        sample_rate: usize,
        cutoff: f32,
        db_gain: f32,
    ) -> (f64, f64, f64, f64, f64, f64) {
        let (w0, w0cos, w0sin, alpha) =
            MiniFilter2::rbj_intermediates_q(sample_rate, cutoff, std::f32::consts::FRAC_1_SQRT_2);
        let a = 10f64.powf(db_gain as f64 / 10.0f64).sqrt();

        (
            1.0 + alpha / a,
            -2.0f64 * w0cos,
            1.0 - alpha / a,
            1.0 + alpha * a,
            -2.0f64 * w0cos,
            1.0 - alpha * a,
        )
    }

    fn rbj_intermediates_shelving(
        sample_rate: usize,
        cutoff: f32,
        a: f64,
        s: f32,
    ) -> (f64, f64, f64, f64) {
        let w0 = 2.0f64 * PI * cutoff as f64 / sample_rate as f64;
        let w0cos = w0.cos();
        let w0sin = w0.sin();
        let alpha = w0sin / 2.0 * ((a + 1.0 / a) * (1.0 / s as f64 - 1.0) + 2.0).sqrt();
        (w0, w0cos, w0sin, alpha)
    }

    fn rbj_low_shelf_coefficients(
        sample_rate: usize,
        cutoff: f32,
        db_gain: f32,
    ) -> (f64, f64, f64, f64, f64, f64) {
        let a = 10f64.powf(db_gain as f64 / 10.0f64).sqrt();
        let (_w0, w0cos, _w0sin, alpha) =
            MiniFilter2::rbj_intermediates_shelving(sample_rate, cutoff, a, 1.0);

        (
            (a + 1.0) + (a - 1.0) * w0cos + 2.0 * a.sqrt() * alpha,
            -2.0 * ((a - 1.0) + (a + 1.0) * w0cos),
            (a + 1.0) + (a - 1.0) * w0cos - 2.0 * a.sqrt() * alpha,
            a * ((a + 1.0) - (a - 1.0) * w0cos + 2.0 * a.sqrt() * alpha),
            2.0 * a * ((a - 1.0) - (a + 1.0) * w0cos),
            a * ((a + 1.0) - (a - 1.0) * w0cos - 2.0 * a.sqrt() * alpha),
        )
    }

    fn rbj_high_shelf_coefficients(
        sample_rate: usize,
        cutoff: f32,
        db_gain: f32,
    ) -> (f64, f64, f64, f64, f64, f64) {
        let a = 10f64.powf(db_gain as f64 / 10.0f64).sqrt();
        let (_w0, w0cos, _w0sin, alpha) =
            MiniFilter2::rbj_intermediates_shelving(sample_rate, cutoff, a, 1.0);

        (
            (a + 1.0) - (a - 1.0) * w0cos + 2.0 * a.sqrt() * alpha,
            2.0 * ((a - 1.0) - (a + 1.0) * w0cos),
            (a + 1.0) - (a - 1.0) * w0cos - 2.0 * a.sqrt() * alpha,
            a * ((a + 1.0) + (a - 1.0) * w0cos + 2.0 * a.sqrt() * alpha),
            -2.0 * a * ((a - 1.0) + (a + 1.0) * w0cos),
            a * ((a + 1.0) + (a - 1.0) * w0cos - 2.0 * a.sqrt() * alpha),
        )
    }
}

impl EffectTrait__ for MiniFilter2 {
    fn process(&mut self, input: MonoSample, _time_seconds: f32) -> MonoSample {
        let s64 = input as f64;
        let r = (self.b0 / self.a0) * s64
            + (self.b1 / self.a0) * self.sample_m1
            + (self.b2 / self.a0) * self.sample_m2
            - (self.a1 / self.a0) * self.output_m1
            - (self.a2 / self.a0) * self.output_m2;

        // Scroll everything forward in time.
        self.sample_m2 = self.sample_m1;
        self.sample_m1 = s64;

        self.output_m2 = self.output_m1;
        self.output_m1 = r;
        r as MonoSample
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use crate::{
        common::{MidiMessage, MidiNote, WaveformType},
        preset::OscillatorPreset,
        primitives::{oscillators::MiniOscillator, tests::write_effect_to_file, ControllerTrait__},
    };

    use super::*;
    const SAMPLE_RATE: usize = 44100;

    #[test]
    fn test_mini_filter() {
        let mut osc = MiniOscillator::new(WaveformType::Noise);

        write_effect_to_file(
            &mut osc,
            Rc::new(RefCell::new(MiniFilter::new(
                SAMPLE_RATE,
                MiniFilterType::None,
            ))),
            &mut None,
            "noise",
        );
        write_effect_to_file(
            &mut osc,
            Rc::new(RefCell::new(MiniFilter::new(
                SAMPLE_RATE,
                MiniFilterType::FirstOrderLowPass(500.),
            ))),
            &mut None,
            "noise_1st_lpf_500Hz",
        );
        write_effect_to_file(
            &mut osc,
            Rc::new(RefCell::new(MiniFilter::new(
                SAMPLE_RATE,
                MiniFilterType::FirstOrderHighPass(500.),
            ))),
            &mut None,
            "noise_1st_hpf_500KHz",
        );
        write_effect_to_file(
            &mut osc,
            Rc::new(RefCell::new(MiniFilter::new(
                SAMPLE_RATE,
                MiniFilterType::FirstOrderLowPass(1000.),
            ))),
            &mut None,
            "noise_1st_lpf_1KHz",
        );
        write_effect_to_file(
            &mut osc,
            Rc::new(RefCell::new(MiniFilter::new(
                SAMPLE_RATE,
                MiniFilterType::FirstOrderHighPass(1000.),
            ))),
            &mut None,
            "noise_1st_hpf_1KHz",
        );
        write_effect_to_file(
            &mut osc,
            Rc::new(RefCell::new(MiniFilter::new(
                SAMPLE_RATE,
                MiniFilterType::SecondOrderLowPass(1000., 0.),
            ))),
            &mut None,
            "noise_2nd_lpf_1KHz_q0",
        );
        write_effect_to_file(
            &mut osc,
            Rc::new(RefCell::new(MiniFilter::new(
                SAMPLE_RATE,
                MiniFilterType::SecondOrderLowPass(500., std::f32::consts::FRAC_1_SQRT_2),
            ))),
            &mut None,
            "noise_2nd_lpf_500Hz_min_q",
        );
        write_effect_to_file(
            &mut osc,
            Rc::new(RefCell::new(MiniFilter::new(
                SAMPLE_RATE,
                MiniFilterType::FourthOrderLowPass(500.),
            ))),
            &mut None,
            "noise_4th_lpf_500Hz",
        );
        write_effect_to_file(
            &mut osc,
            Rc::new(RefCell::new(MiniFilter::new(
                SAMPLE_RATE,
                MiniFilterType::FourthOrderHighPass(500.),
            ))),
            &mut None,
            "noise_4th_hpf_500Hz",
        );
        write_effect_to_file(
            &mut osc,
            Rc::new(RefCell::new(MiniFilter::new(
                SAMPLE_RATE,
                MiniFilterType::SecondOrderLowPass(1000., std::f32::consts::FRAC_1_SQRT_2),
            ))),
            &mut None,
            "noise_2nd_lpf_1KHz_min_q",
        );

        write_effect_to_file(
            &mut osc,
            Rc::new(RefCell::new(MiniFilter::new(
                SAMPLE_RATE,
                MiniFilterType::SecondOrderLowPass(1000., 0.9),
            ))),
            &mut None,
            "noise_2nd_lpf_1KHz_q0.9",
        );
        write_effect_to_file(
            &mut osc,
            Rc::new(RefCell::new(MiniFilter::new(
                SAMPLE_RATE,
                MiniFilterType::SecondOrderLowPass(1000., 10.),
            ))),
            &mut None,
            "noise_2nd_lpf_1KHz_q10",
        );
        write_effect_to_file(
            &mut osc,
            Rc::new(RefCell::new(MiniFilter::new(
                SAMPLE_RATE,
                MiniFilterType::SecondOrderLowPass(1000., 20.),
            ))),
            &mut None,
            "noise_2nd_lpf_1KHz_q20",
        );
        write_effect_to_file(
            &mut osc,
            Rc::new(RefCell::new(MiniFilter::new(
                SAMPLE_RATE,
                MiniFilterType::SecondOrderLowPass(1000., 20000.),
            ))),
            &mut None,
            "noise_2nd_lpf_1KHz_q20000",
        );
        write_effect_to_file(
            &mut osc,
            Rc::new(RefCell::new(MiniFilter::new(
                SAMPLE_RATE,
                MiniFilterType::SecondOrderHighPass(1000., 20.),
            ))),
            &mut None,
            "noise_2nd_hpf_1KHz",
        );
        write_effect_to_file(
            &mut osc,
            Rc::new(RefCell::new(MiniFilter::new(
                SAMPLE_RATE,
                MiniFilterType::SecondOrderBandPass(1000., 10.),
            ))),
            &mut None,
            "noise_2nd_bpf_1KHz",
        );
        write_effect_to_file(
            &mut osc,
            Rc::new(RefCell::new(MiniFilter::new(
                SAMPLE_RATE,
                MiniFilterType::SecondOrderBandStop(1000., 20.),
            ))),
            &mut None,
            "noise_2nd_bsf_1KHz",
        );
    }

    struct TestFilterCutoffController {
        target: Rc<RefCell<MiniFilter2>>,
        param_start: f32,
        param_end: f32,
        duration: f32,

        time_start: f32,
    }

    impl TestFilterCutoffController {
        pub fn new(
            target: Rc<RefCell<MiniFilter2>>,
            param_start: f32,
            param_end: f32,
            duration: f32,
        ) -> Self {
            Self {
                target,
                param_start,
                param_end,
                duration,
                time_start: -1.0f32,
            }
        }
    }

    impl<'a> ControllerTrait__ for TestFilterCutoffController {
        fn process(&mut self, time_seconds: f32) {
            if self.time_start < 0.0 {
                self.time_start = time_seconds;
            }
            if self.param_end != self.param_start {
                self.target.borrow_mut().set_cutoff(
                    self.param_start
                        + ((time_seconds - self.time_start) / self.duration)
                            * (self.param_end - self.param_start),
                );
            }
        }
    }

    struct TestFilterQController {
        target: Rc<RefCell<MiniFilter2>>,
        param_start: f32,
        param_end: f32,
        duration: f32,

        time_start: f32,
    }

    impl TestFilterQController {
        pub fn new(
            target: Rc<RefCell<MiniFilter2>>,
            param_start: f32,
            param_end: f32,
            duration: f32,
        ) -> Self {
            Self {
                target,
                param_start,
                param_end,
                duration,
                time_start: -1.0f32,
            }
        }
    }

    impl<'a> ControllerTrait__ for TestFilterQController {
        fn process(&mut self, time_seconds: f32) {
            if self.time_start < 0.0 {
                self.time_start = time_seconds;
            }
            if self.param_end != self.param_start {
                self.target.borrow_mut().set_q(
                    self.param_start
                        + ((time_seconds - self.time_start) / self.duration)
                            * (self.param_end - self.param_start),
                );
            }
        }
    }

    #[test]
    fn test_mini_filter2() {
        const Q_10: f32 = 10.0;
        const ONE_OCTAVE: f32 = 1.0;
        const SIX_DB: f32 = 6.0;

        let mut source = MiniOscillator::new(WaveformType::Noise);

        write_effect_to_file(
            &mut source,
            Rc::new(RefCell::new(MiniFilter2::new(&MiniFilter2Type::LowPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                q: std::f32::consts::FRAC_1_SQRT_2,
            }))),
            &mut None,
            "rbj_noise_lpf_1KHz_min_q",
        );
        write_effect_to_file(
            &mut source,
            Rc::new(RefCell::new(MiniFilter2::new(&MiniFilter2Type::LowPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                q: Q_10,
            }))),
            &mut None,
            "rbj_noise_lpf_1KHz_q10",
        );
        write_effect_to_file(
            &mut source,
            Rc::new(RefCell::new(MiniFilter2::new(&MiniFilter2Type::HighPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                q: std::f32::consts::FRAC_1_SQRT_2,
            }))),
            &mut None,
            "rbj_noise_hpf_1KHz_min_q",
        );
        write_effect_to_file(
            &mut source,
            Rc::new(RefCell::new(MiniFilter2::new(&MiniFilter2Type::HighPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                q: Q_10,
            }))),
            &mut None,
            "rbj_noise_hpf_1KHz_q10",
        );
        write_effect_to_file(
            &mut source,
            Rc::new(RefCell::new(MiniFilter2::new(&MiniFilter2Type::BandPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                bandwidth: ONE_OCTAVE,
            }))),
            &mut None,
            "rbj_noise_bpf_1KHz_bw1",
        );
        write_effect_to_file(
            &mut source,
            Rc::new(RefCell::new(MiniFilter2::new(&MiniFilter2Type::BandStop {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                bandwidth: ONE_OCTAVE,
            }))),
            &mut None,
            "rbj_noise_bsf_1KHz_bw1",
        );
        write_effect_to_file(
            &mut source,
            Rc::new(RefCell::new(MiniFilter2::new(&MiniFilter2Type::AllPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.0,
                q: std::f32::consts::FRAC_1_SQRT_2,
            }))),
            &mut None,
            "rbj_noise_apf_1KHz_min_q",
        );
        write_effect_to_file(
            &mut source,
            Rc::new(RefCell::new(MiniFilter2::new(
                &MiniFilter2Type::PeakingEq {
                    sample_rate: SAMPLE_RATE,
                    cutoff: 1000.,
                    db_gain: SIX_DB,
                },
            ))),
            &mut None,
            "rbj_noise_peaking_eq_1KHz_6db",
        );
        write_effect_to_file(
            &mut source,
            Rc::new(RefCell::new(MiniFilter2::new(&MiniFilter2Type::LowShelf {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                db_gain: SIX_DB,
            }))),
            &mut None,
            "rbj_noise_low_shelf_1KHz_6db",
        );
        write_effect_to_file(
            &mut source,
            Rc::new(RefCell::new(MiniFilter2::new(
                &MiniFilter2Type::HighShelf {
                    sample_rate: SAMPLE_RATE,
                    cutoff: 1000.,
                    db_gain: SIX_DB,
                },
            ))),
            &mut None,
            "rbj_noise_high_shelf_1KHz_6db",
        );
    }

    #[test]
    fn test_dynamic_cutoff() {
        let mut source = MiniOscillator::new_from_preset(&OscillatorPreset {
            waveform: WaveformType::Sawtooth,
            ..Default::default()
        });
        source.set_frequency(MidiMessage::note_to_frequency(MidiNote::C4 as u8));

        {
            let effect = Rc::new(RefCell::new(MiniFilter2::new(&MiniFilter2Type::LowPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                q: std::f32::consts::FRAC_1_SQRT_2,
            })));
            let mut controller = TestFilterCutoffController::new(effect.clone(), 40.0, 8000.0, 2.0);
            write_effect_to_file(
                &mut source,
                effect.clone(),
                &mut Some(&mut controller),
                "rbj_sawtooth_middle_c_lpf_dynamic_40Hz_8KHz_min_q",
            );
        }
        {
            let effect = Rc::new(RefCell::new(MiniFilter2::new(&MiniFilter2Type::LowPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                q: std::f32::consts::FRAC_1_SQRT_2,
            })));
            let mut controller = TestFilterQController::new(
                effect.clone(),
                std::f32::consts::FRAC_1_SQRT_2,
                std::f32::consts::FRAC_1_SQRT_2 * 20.0,
                2.0,
            );
            write_effect_to_file(
                &mut source,
                effect.clone(),
                &mut Some(&mut controller),
                "rbj_sawtooth_middle_c_lpf_1KHz_dynamic_min_q_20",
            );
        }
        {
            let effect = Rc::new(RefCell::new(MiniFilter2::new(&MiniFilter2Type::HighPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                q: std::f32::consts::FRAC_1_SQRT_2,
            })));
            let mut controller = TestFilterCutoffController::new(effect.clone(), 8000.0, 40.0, 2.0);
            write_effect_to_file(
                &mut source,
                effect.clone(),
                &mut Some(&mut controller),
                "rbj_sawtooth_middle_c_hpf_dynamic_8KHz_40Hz_min_q",
            );
        }
        {
            let effect = Rc::new(RefCell::new(MiniFilter2::new(&MiniFilter2Type::BandPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                bandwidth: std::f32::consts::FRAC_1_SQRT_2,
            })));
            let mut controller = TestFilterCutoffController::new(effect.clone(), 40.0, 8000.0, 2.0);
            write_effect_to_file(
                &mut source,
                effect.clone(),
                &mut Some(&mut controller),
                "rbj_sawtooth_middle_c_bpf_dynamic_40Hz_8KHz_min_q",
            );
        }
        {
            let effect = Rc::new(RefCell::new(MiniFilter2::new(&MiniFilter2Type::BandStop {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                bandwidth: std::f32::consts::FRAC_1_SQRT_2,
            })));
            let mut controller = TestFilterCutoffController::new(effect.clone(), 40.0, 1500.0, 2.0);
            write_effect_to_file(
                &mut source,
                effect.clone(),
                &mut Some(&mut controller),
                "rbj_sawtooth_middle_c_bsf_dynamic_40Hz_1.5KHz_min_q",
            );
        }
    }
}
