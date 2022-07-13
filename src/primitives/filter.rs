use std::f64::consts::PI;

#[derive(Clone, Copy)]
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

impl MiniFilter {
    pub fn new(sample_rate: u32, filter_type: MiniFilterType) -> Self {
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

    pub fn filter(&mut self, sample: f32) -> f32 {
        let s64 = sample as f64;
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
        r as f32
    }

    fn first_order_low_pass_coefficients(
        sample_rate: u32,
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
        sample_rate: u32,
        cutoff: f32,
    ) -> (u8, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64) {
        let theta_c = 2.0 * PI * cutoff as f64 / (sample_rate as f64);
        let gamma = theta_c.cos() / (1.0 + theta_c.sin());
        let alpha = (1.0 + gamma) / 2.0;
        (
            1, alpha, -alpha, 0.0, 0.0, 0.0, -gamma, 0.0, 0.0, 0.0, 1.0, 0.0,
        )
    }

    fn common_second_order_coefficients(sample_rate: u32, cutoff: f32, q: f32) -> (f64, f64) {
        let theta_c = 2.0 * PI * cutoff as f64 / (sample_rate as f64);
        let delta = 1.0 / (q as f64).max(1.0 / 2.0f64.sqrt());
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
        sample_rate: u32,
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
        sample_rate: u32,
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
        let sqrt2 = 2.0f64.sqrt();
        let sq_tmp1 = sqrt2 * omega3 * kappa;
        let sq_tmp2 = sqrt2 * omega * kappa3;
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
        sample_rate: u32,
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
        let sqrt2 = 2.0f64.sqrt();
        let sq_tmp1 = sqrt2 * omega3 * kappa;
        let sq_tmp2 = sqrt2 * omega * kappa3;
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
        sample_rate: u32,
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
        sample_rate: u32,
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
        sample_rate: u32,
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

#[derive(Debug, Clone, Copy)]
pub enum MiniFilter2Type {
    None,
    LowPass(u32, f32, f32),
    HighPass(u32, f32, f32),
    BandPass(u32, f32, f32),
    BandStop(u32, f32, f32),
    AllPass(u32, f32, f32),
    PeakingEq(u32, f32, f32),
    LowShelf(u32, f32, f32),
    HighShelf(u32, f32, f32),
}

impl Default for MiniFilter2Type {
    fn default() -> Self {
        MiniFilter2Type::None
    }
}

#[derive(Debug, Default)]
pub struct MiniFilter2 {
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

impl MiniFilter2 {
    // 1 / square root of 2
    pub const MIN_Q: f32 = 0.707106781f32;

    pub fn new(filter_type: MiniFilter2Type) -> Self {
        let (sample_rate, a0, a1, a2, b0, b1, b2) = match filter_type {
            MiniFilter2Type::None => (0u32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            MiniFilter2Type::LowPass(sample_rate, cutoff, q) => {
                Self::rbj_low_pass_coefficients(sample_rate, cutoff, q)
            }
            MiniFilter2Type::HighPass(sample_rate, cutoff, q) => {
                Self::rbj_high_pass_coefficients(sample_rate, cutoff, q)
            }
            MiniFilter2Type::BandPass(sample_rate, cutoff, q) => {
                Self::rbj_band_pass_coefficients(sample_rate, cutoff, q)
            }
            MiniFilter2Type::BandStop(sample_rate, cutoff, q) => {
                Self::rbj_band_stop_coefficients(sample_rate, cutoff, q)
            }
            MiniFilter2Type::AllPass(sample_rate, cutoff, q) => {
                Self::rbj_all_pass_coefficients(sample_rate, cutoff, q)
            }
            MiniFilter2Type::PeakingEq(sample_rate, cutoff, db_gain) => {
                Self::rbj_peaking_eq_coefficients(sample_rate, cutoff, db_gain)
            }
            MiniFilter2Type::LowShelf(sample_rate, cutoff, db_gain) => {
                Self::rbj_low_shelf_coefficients(sample_rate, cutoff, db_gain)
            }
            MiniFilter2Type::HighShelf(sample_rate, cutoff, db_gain) => {
                Self::rbj_high_shelf_coefficients(sample_rate, cutoff, db_gain)
            }
        };
        Self {
            a0,
            a1,
            a2,
            b0,
            b1,
            b2,
            ..Default::default()
        }
    }

    pub fn filter(&mut self, sample: f32) -> f32 {
        let s64 = sample as f64;
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
        r as f32
    }

    fn rbj_intermediates_q(sample_rate: u32, cutoff: f32, q: f32) -> (f64, f64, f64, f64) {
        //        let Q = 1.0 / 2.0f64.sqrt();
        let w0 = 2.0f64 * PI * cutoff as f64 / sample_rate as f64;
        let w0cos = w0.cos();
        let w0sin = w0.sin();
        let alpha = w0sin / (2.0f64 * q as f64);
        (w0, w0cos, w0sin, alpha)
    }

    fn rbj_low_pass_coefficients(
        sample_rate: u32,
        cutoff: f32,
        q: f32,
    ) -> (u32, f64, f64, f64, f64, f64, f64) {
        let (w0, w0cos, w0sin, alpha) = MiniFilter2::rbj_intermediates_q(sample_rate, cutoff, q);

        (
            sample_rate,
            1.0 + alpha,
            -2.0f64 * w0cos,
            1.0 - alpha,
            (1.0 - w0cos) / 2.0f64,
            (1.0 - w0cos),
            (1.0 - w0cos) / 2.0f64,
        )
    }

    fn rbj_high_pass_coefficients(
        sample_rate: u32,
        cutoff: f32,
        q: f32,
    ) -> (u32, f64, f64, f64, f64, f64, f64) {
        let (w0, w0cos, w0sin, alpha) = MiniFilter2::rbj_intermediates_q(sample_rate, cutoff, q);

        (
            sample_rate,
            1.0 + alpha,
            -2.0f64 * w0cos,
            1.0 - alpha,
            (1.0 + w0cos) / 2.0f64,
            -(1.0 + w0cos),
            (1.0 + w0cos) / 2.0f64,
        )
    }

    fn rbj_intermediates_bandwidth(sample_rate: u32, cutoff: f32, bw: f32) -> (f64, f64, f64, f64) {
        let w0 = 2.0f64 * PI * cutoff as f64 / sample_rate as f64;
        let w0cos = w0.cos();
        let w0sin = w0.sin();
        let alpha = w0sin * (2.0f64.ln() / 2.0 * bw as f64 * w0 / w0.sin()).sinh();
        (w0, w0cos, w0sin, alpha)
    }

    fn rbj_band_pass_coefficients(
        sample_rate: u32,
        cutoff: f32,
        bandwidth: f32,
    ) -> (u32, f64, f64, f64, f64, f64, f64) {
        let (w0, w0cos, w0sin, alpha) =
            MiniFilter2::rbj_intermediates_bandwidth(sample_rate, cutoff, bandwidth);
        (
            sample_rate,
            1.0 + alpha,
            -2.0f64 * w0cos,
            1.0 - alpha,
            alpha,
            0.0,
            -alpha,
        )
    }

    fn rbj_band_stop_coefficients(
        sample_rate: u32,
        cutoff: f32,
        bandwidth: f32,
    ) -> (u32, f64, f64, f64, f64, f64, f64) {
        let (w0, w0cos, w0sin, alpha) =
            MiniFilter2::rbj_intermediates_bandwidth(sample_rate, cutoff, bandwidth);

        (
            sample_rate,
            1.0 + alpha,
            -2.0f64 * w0cos,
            1.0 - alpha,
            1.0,
            -2.0f64 * w0cos,
            1.0,
        )
    }

    fn rbj_all_pass_coefficients(
        sample_rate: u32,
        cutoff: f32,
        q: f32,
    ) -> (u32, f64, f64, f64, f64, f64, f64) {
        let (w0, w0cos, w0sin, alpha) = MiniFilter2::rbj_intermediates_q(sample_rate, cutoff, q);
        (
            sample_rate,
            1.0 + alpha,
            -2.0f64 * w0cos,
            1.0 - alpha,
            1.0 - alpha,
            -2.0f64 * w0cos,
            1.0 + alpha,
        )
    }

    fn rbj_peaking_eq_coefficients(
        sample_rate: u32,
        cutoff: f32,
        db_gain: f32,
    ) -> (u32, f64, f64, f64, f64, f64, f64) {
        let (w0, w0cos, w0sin, alpha) =
            MiniFilter2::rbj_intermediates_q(sample_rate, cutoff, 1.0 / 2.0f32.sqrt());
        let a = 10f64.powf(db_gain as f64 / 10.0f64).sqrt();

        (
            sample_rate,
            1.0 + alpha / a,
            -2.0f64 * w0cos,
            1.0 - alpha / a,
            1.0 + alpha * a,
            -2.0f64 * w0cos,
            1.0 - alpha * a,
        )
    }

    fn rbj_intermediates_shelving(
        sample_rate: u32,
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
        sample_rate: u32,
        cutoff: f32,
        db_gain: f32,
    ) -> (u32, f64, f64, f64, f64, f64, f64) {
        let a = 10f64.powf(db_gain as f64 / 10.0f64).sqrt();
        let (_w0, w0cos, _w0sin, alpha) =
            MiniFilter2::rbj_intermediates_shelving(sample_rate, cutoff, a, 1.0);

        (
            sample_rate,
            (a + 1.0) + (a - 1.0) * w0cos + 2.0 * a.sqrt() * alpha,
            -2.0 * ((a - 1.0) + (a + 1.0) * w0cos),
            (a + 1.0) + (a - 1.0) * w0cos - 2.0 * a.sqrt() * alpha,
            a * ((a + 1.0) - (a - 1.0) * w0cos + 2.0 * a.sqrt() * alpha),
            2.0 * a * ((a - 1.0) - (a + 1.0) * w0cos),
            a * ((a + 1.0) - (a - 1.0) * w0cos - 2.0 * a.sqrt() * alpha),
        )
    }

    fn rbj_high_shelf_coefficients(
        sample_rate: u32,
        cutoff: f32,
        db_gain: f32,
    ) -> (u32, f64, f64, f64, f64, f64, f64) {
        let a = 10f64.powf(db_gain as f64 / 10.0f64).sqrt();
        let (_w0, w0cos, _w0sin, alpha) =
            MiniFilter2::rbj_intermediates_shelving(sample_rate, cutoff, a, 1.0);

        (
            sample_rate,
            (a + 1.0) - (a - 1.0) * w0cos + 2.0 * a.sqrt() * alpha,
            2.0 * ((a - 1.0) - (a + 1.0) * w0cos),
            (a + 1.0) - (a - 1.0) * w0cos - 2.0 * a.sqrt() * alpha,
            a * ((a + 1.0) + (a - 1.0) * w0cos + 2.0 * a.sqrt() * alpha),
            -2.0 * a * ((a - 1.0) + (a + 1.0) * w0cos),
            a * ((a + 1.0) + (a - 1.0) * w0cos - 2.0 * a.sqrt() * alpha),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::primitives::{clock::Clock, oscillators::MiniOscillator};

    use super::*;

    fn write_filter_sample(filter: &mut MiniFilter, filename: &str) {
        let mut clock = Clock::new(44100, 4, 4, 128.);
        let mut osc = MiniOscillator::new(crate::primitives::oscillators::Waveform::Noise);

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: clock.sample_rate(),
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        const AMPLITUDE: f32 = i16::MAX as f32;
        let mut filter_writer = hound::WavWriter::create(filename, spec).unwrap();

        while clock.seconds < 2.0 {
            let sample_osc = osc.process(clock.seconds);
            let sample_filter = filter.filter(sample_osc);
            let _ = filter_writer.write_sample((sample_filter * AMPLITUDE) as i16);
            clock.tick();
        }
    }

    #[test]
    fn test_mini_filter() {
        const SAMPLE_RATE: u32 = 44100;
        let min_q: f32 = 1.0 / 2.0f32.sqrt();

        let mut filter = MiniFilter::new(SAMPLE_RATE, MiniFilterType::None);
        write_filter_sample(&mut filter, "noise.wav");
        let mut filter = MiniFilter::new(SAMPLE_RATE, MiniFilterType::FirstOrderLowPass(500.));
        write_filter_sample(&mut filter, "noise_1st_lpf_500Hz.wav");
        let mut filter = MiniFilter::new(SAMPLE_RATE, MiniFilterType::FirstOrderHighPass(500.));
        write_filter_sample(&mut filter, "noise_1st_hpf_500KHz.wav");
        let mut filter = MiniFilter::new(SAMPLE_RATE, MiniFilterType::FirstOrderLowPass(1000.));
        write_filter_sample(&mut filter, "noise_1st_lpf_1KHz.wav");
        let mut filter = MiniFilter::new(SAMPLE_RATE, MiniFilterType::FirstOrderHighPass(1000.));
        write_filter_sample(&mut filter, "noise_1st_hpf_1KHz.wav");
        filter = MiniFilter::new(SAMPLE_RATE, MiniFilterType::SecondOrderLowPass(1000., 0.));
        write_filter_sample(&mut filter, "noise_2nd_lpf_1KHz_q0.wav");
        filter = MiniFilter::new(SAMPLE_RATE, MiniFilterType::SecondOrderLowPass(500., min_q));
        write_filter_sample(&mut filter, "noise_2nd_lpf_500Hz_min_q.wav");
        filter = MiniFilter::new(SAMPLE_RATE, MiniFilterType::FourthOrderLowPass(500.));
        write_filter_sample(&mut filter, "noise_4th_lpf_500Hz.wav");
        filter = MiniFilter::new(SAMPLE_RATE, MiniFilterType::FourthOrderHighPass(500.));
        write_filter_sample(&mut filter, "noise_4th_hpf_500Hz.wav");
        filter = MiniFilter::new(
            SAMPLE_RATE,
            MiniFilterType::SecondOrderLowPass(1000., min_q),
        );
        write_filter_sample(&mut filter, "noise_2nd_lpf_1KHz_min_q.wav");
        filter = MiniFilter::new(SAMPLE_RATE, MiniFilterType::SecondOrderLowPass(1000., 0.9));
        write_filter_sample(&mut filter, "noise_2nd_lpf_1KHz_q0.9.wav");
        filter = MiniFilter::new(SAMPLE_RATE, MiniFilterType::SecondOrderLowPass(1000., 10.));
        write_filter_sample(&mut filter, "noise_2nd_lpf_1KHz_q10.wav");
        filter = MiniFilter::new(SAMPLE_RATE, MiniFilterType::SecondOrderLowPass(1000., 20.));
        write_filter_sample(&mut filter, "noise_2nd_lpf_1KHz_q20.wav");
        filter = MiniFilter::new(
            SAMPLE_RATE,
            MiniFilterType::SecondOrderLowPass(1000., 20000.),
        );
        write_filter_sample(&mut filter, "noise_2nd_lpf_1KHz_q20000.wav");
        filter = MiniFilter::new(SAMPLE_RATE, MiniFilterType::SecondOrderHighPass(1000., 20.));
        write_filter_sample(&mut filter, "noise_2nd_hpf_1KHz.wav");
        filter = MiniFilter::new(SAMPLE_RATE, MiniFilterType::SecondOrderBandPass(1000., 10.));
        write_filter_sample(&mut filter, "noise_2nd_bpf_1KHz.wav");
        filter = MiniFilter::new(SAMPLE_RATE, MiniFilterType::SecondOrderBandStop(1000., 20.));
        write_filter_sample(&mut filter, "noise_2nd_bsf_1KHz.wav");
    }

    fn write_filter2_sample(filter: &mut MiniFilter2, filename: &str) {
        let mut clock = Clock::new(44100, 4, 4, 128.);
        let mut osc = MiniOscillator::new(crate::primitives::oscillators::Waveform::Noise);

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: clock.sample_rate(),
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        const AMPLITUDE: f32 = i16::MAX as f32;
        let mut filter_writer = hound::WavWriter::create(filename, spec).unwrap();

        while clock.seconds < 2.0 {
            let sample_osc = osc.process(clock.seconds);
            let sample_filter = filter.filter(sample_osc);
            let _ = filter_writer.write_sample((sample_filter * AMPLITUDE) as i16);
            clock.tick();
        }
    }

    #[test]
    fn test_mini_filter2() {
        const SAMPLE_RATE: u32 = 44100;
        let min_q = 1.0 / 2.0f32.sqrt();
        const Q_10: f32 = 10.0;
        const ONE_OCTAVE: f32 = 1.0;
        const SIX_DB: f32 = 6.0;
        write_filter2_sample(
            &mut MiniFilter2::new(MiniFilter2Type::LowPass(SAMPLE_RATE, 1000., min_q)),
            "rbj_noise_lpf_1KHz_min_q.wav",
        );
        write_filter2_sample(
            &mut MiniFilter2::new(MiniFilter2Type::LowPass(SAMPLE_RATE, 1000., Q_10)),
            "rbj_noise_lpf_1KHz_q10.wav",
        );
        write_filter2_sample(
            &mut MiniFilter2::new(MiniFilter2Type::HighPass(SAMPLE_RATE, 1000., min_q)),
            "rbj_noise_hpf_1KHz_min_q.wav",
        );
        write_filter2_sample(
            &mut MiniFilter2::new(MiniFilter2Type::HighPass(SAMPLE_RATE, 1000., Q_10)),
            "rbj_noise_hpf_1KHz_q10.wav",
        );
        write_filter2_sample(
            &mut MiniFilter2::new(MiniFilter2Type::BandPass(SAMPLE_RATE, 1000., ONE_OCTAVE)),
            "rbj_noise_bpf_1KHz_bw1.wav",
        );
        write_filter2_sample(
            &mut MiniFilter2::new(MiniFilter2Type::BandStop(SAMPLE_RATE, 1000., ONE_OCTAVE)),
            "rbj_noise_bsf_1KHz_bw1.wav",
        );
        write_filter2_sample(
            &mut MiniFilter2::new(MiniFilter2Type::AllPass(SAMPLE_RATE, 1000., min_q)),
            "rbj_noise_apf_1KHz_min_q.wav",
        );
        write_filter2_sample(
            &mut MiniFilter2::new(MiniFilter2Type::PeakingEq(SAMPLE_RATE, 1000., SIX_DB)),
            "rbj_noise_peaking_eq_1KHz_6db.wav",
        );
        write_filter2_sample(
            &mut MiniFilter2::new(MiniFilter2Type::LowShelf(SAMPLE_RATE, 1000., SIX_DB)),
            "rbj_noise_low_shelf_1KHz_6db.wav",
        );
        write_filter2_sample(
            &mut MiniFilter2::new(MiniFilter2Type::HighShelf(SAMPLE_RATE, 1000., SIX_DB)),
            "rbj_noise_high_shelf_1KHz_6db.wav",
        );
    }
}
