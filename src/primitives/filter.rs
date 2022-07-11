use std::f32::consts::PI;

pub enum MiniFilterType {
    None,
    FirstOrderLowPass(f32),
    FirstOrderHighPass(f32),
    SecondOrderLowPass(f32, f32),
    SecondOrderHighPass(f32, f32),
    SecondOrderBandPass(f32, f32),
    SecondOrderBandStop(f32, f32),
    // Not sure Butterworth filters are worth implementing. Pirkle says they're very similar to second-order.
    // SecondOrderButterworthLowPass,
    // SecondOrderButterworthHighPass,
    // SecondOrderButterworthBandPass,
    // SecondOrderButterworthBandStop,
}
#[derive(Default)]
pub struct MiniFilter {
    a0: f32,
    a1: f32,
    a2: f32,
    b1: f32,
    b2: f32,
    c0: f32,
    d0: f32,
    sample_m1: f32, // "sample minus two" or x(n-2)
    sample_m2: f32,
    output_m1: f32,
    output_m2: f32,
}

impl MiniFilter {
    pub fn new(sample_rate: u32, filter_type: MiniFilterType) -> Self {
        let (a0, a1, a2, b1, b2, c0, d0) = match filter_type {
            MiniFilterType::None => (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0),
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
        };
        Self {
            a0,
            a1,
            a2,
            b1,
            b2,
            c0,
            d0,
            ..Default::default()
        }
    }

    pub fn filter(&mut self, sample: f32) -> f32 {
        let result = self.d0 * sample
            + self.c0
                * (self.a0 * sample + self.a1 * self.sample_m1 + self.a2 * self.sample_m2
                    - self.b1 * self.output_m1
                    - self.b2 * self.output_m2);

        // Scroll everything forward in time.
        self.sample_m2 = self.sample_m1;
        self.sample_m1 = sample;
        self.output_m2 = self.output_m1;
        self.output_m1 = result;
        result
    }

    fn first_order_low_pass_coefficients(
        sample_rate: u32,
        cutoff: f32,
    ) -> (f32, f32, f32, f32, f32, f32, f32) {
        let theta_c = 2.0 * PI * cutoff / (sample_rate as f32);
        let gamma = theta_c.cos() / (1.0 + theta_c.sin());
        let alpha = (1.0 - gamma) / 2.0;

        (alpha, alpha, 0.0, -gamma, 0.0, 1.0, 0.0)
    }

    fn first_order_high_pass_coefficients(
        sample_rate: u32,
        cutoff: f32,
    ) -> (f32, f32, f32, f32, f32, f32, f32) {
        let theta_c = 2.0 * PI * cutoff / (sample_rate as f32);
        let gamma = theta_c.cos() / (1.0 + theta_c.sin());
        let alpha = (1.0 + gamma) / 2.0;
        (alpha, -alpha, 0.0, -gamma, 0.0, 1.0, 0.0)
    }

    fn common_second_order_coefficients(sample_rate: u32, cutoff: f32, q: f32) -> (f32, f32) {
        let theta_c = 2.0 * PI * cutoff / (sample_rate as f32);
        let delta = 1.0 / q.max(1.0 / 2.0f32.sqrt());
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
    ) -> (f32, f32, f32, f32, f32, f32, f32) {
        let (beta, gamma) = Self::common_second_order_coefficients(sample_rate, cutoff, q);
        let alpha_n = 0.5 + beta - gamma;

        (
            alpha_n / 2.0,
            alpha_n,
            alpha_n / 2.0,
            -2.0 * gamma,
            2.0 * beta,
            1.0,
            0.0,
        )
    }

    fn second_order_high_pass_coefficients(
        sample_rate: u32,
        cutoff: f32,
        q: f32,
    ) -> (f32, f32, f32, f32, f32, f32, f32) {
        let (beta, gamma) = Self::common_second_order_coefficients(sample_rate, cutoff, q);
        let alpha_n = 0.5 + beta + gamma;

        (
            alpha_n / 2.0,
            -alpha_n,
            alpha_n / 2.0,
            -2.0 * gamma,
            2.0 * beta,
            1.0,
            0.0,
        )
    }
    fn second_order_band_pass_coefficients(
        sample_rate: u32,
        cutoff: f32,
        q: f32,
    ) -> (f32, f32, f32, f32, f32, f32, f32) {
        let kappa = (PI * cutoff / sample_rate as f32).tan();
        let kappa_sq = kappa.powi(2);
        let delta = kappa_sq * q + kappa + q;

        (
            kappa / delta,
            0.0,
            -kappa / delta,
            (2.0 * q * (kappa_sq - 1.0)) / delta,
            (kappa_sq * q - kappa + q) / delta,
            1.0,
            0.0,
        )
    }
    fn second_order_band_stop_coefficients(
        sample_rate: u32,
        cutoff: f32,
        q: f32,
    ) -> (f32, f32, f32, f32, f32, f32, f32) {
        let kappa = (PI * cutoff / sample_rate as f32).tan();
        let kappa_sq = kappa.powi(2);
        let delta = kappa_sq * q + kappa + q;

        let alpha_a = (q * (kappa_sq + 1.0)) / delta;
        let alpha_b = (2.0 * q * (kappa_sq - 1.0)) / delta;
        (
            alpha_a,
            alpha_b,
            alpha_a,
            alpha_b,
            (kappa_sq * q - kappa + q) / delta,
            1.0,
            0.0,
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
        filter = MiniFilter::new(SAMPLE_RATE, MiniFilterType::SecondOrderLowPass(500., 0.707));
        write_filter_sample(&mut filter, "noise_2nd_lpf_500Hz_q0.707.wav");
        filter = MiniFilter::new(
            SAMPLE_RATE,
            MiniFilterType::SecondOrderLowPass(1000., 0.707),
        );
        write_filter_sample(&mut filter, "noise_2nd_lpf_1KHz_q0.707.wav");
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
}