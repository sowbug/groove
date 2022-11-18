use crate::{
    common::{rrc, rrc_downgrade, MonoSample, Rrc, Ww},
    traits::{HasOverhead, IsEffect, Overhead, SinksAudio, SourcesAudio, TransformsAudio}, clock::Clock,
};
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, Default)]
pub enum FilterType {
    #[default]
    None,
    LowPass,
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
    LowPass {
        cutoff: f32,
        q: f32,
    },
    HighPass {
        cutoff: f32,
        q: f32,
    },
    BandPass {
        cutoff: f32,
        bandwidth: f32,
    },
    BandStop {
        cutoff: f32,
        bandwidth: f32,
    },
    AllPass {
        cutoff: f32,
        q: f32,
    },
    PeakingEq {
        cutoff: f32,
        db_gain: f32,
    },
    LowShelf {
        cutoff: f32,
        db_gain: f32,
    },
    HighShelf {
        cutoff: f32,
        db_gain: f32,
    },
}

impl FilterParams {
    fn type_for(params: Self) -> FilterType {
        #[allow(unused_variables)]
        match params {
            FilterParams::None => FilterType::None,
            FilterParams::LowPass { cutoff, q } => FilterType::LowPass,
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

#[derive(Debug, Default)]
struct CoefficientSet {
    a0: f64,
    a1: f64,
    a2: f64,
    b0: f64,
    b1: f64,
    b2: f64,
}

/// https://en.wikipedia.org/wiki/Digital_biquad_filter
#[derive(Debug)]
pub struct BiQuadFilter {
    pub(crate) me: Ww<Self>,
    overhead: Overhead,

    sources: Vec<Ww<dyn SourcesAudio>>,

    sample_rate: usize,
    filter_type: FilterType,
    cutoff: f32,
    param2: f32, // can represent q, bandwidth, or db_gain
    coefficients: CoefficientSet,

    // Working variables
    sample_m1: f64, // "sample minus two" or x(n-2)
    sample_m2: f64,
    output_m1: f64,
    output_m2: f64,
}
impl IsEffect for BiQuadFilter {}

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
    pub const FREQUENCY_TO_LINEAR_BASE: f32 = 800.0;
    pub const FREQUENCY_TO_LINEAR_COEFFICIENT: f32 = 25.0;

    // https://docs.google.com/spreadsheets/d/1uQylh2h77-fuJ6OM0vjF7yjRXflLFP0yQEnv5wbaP2c/edit#gid=0
    // =LOGEST(Sheet1!B2:B23, Sheet1!A2:A23,true, false)
    //
    // Column A is 24db filter percentages from all the patches. Column B is
    // envelope-filter percentages from all the patches.
    pub fn percent_to_frequency(percentage: f32) -> f32 {
        debug_assert!(
            (0.0..=1.0).contains(&percentage),
            "Expected range (0.0..=1.0) but got {}",
            percentage
        );
        Self::FREQUENCY_TO_LINEAR_BASE * Self::FREQUENCY_TO_LINEAR_COEFFICIENT.powf(percentage)
    }

    pub fn frequency_to_percent(frequency: f32) -> f32 {
        debug_assert!(frequency >= 0.0);

        // I was stressed out about slightly negative values, but then I decided
        // that adjusting the log numbers to handle more edge cases wasn't going
        // to make a practical difference. So I'm clamping to [0, 1].
        (frequency / Self::FREQUENCY_TO_LINEAR_COEFFICIENT)
            .log(Self::FREQUENCY_TO_LINEAR_BASE)
            .clamp(0.0, 1.0)
    }

    /// Returns a new/default struct without calling update_coefficients().
    fn default_fields() -> Self {
        Self {
            me: Default::default(),
            overhead: Default::default(),
            sources: Default::default(),
            filter_type: Default::default(),
            sample_rate: Default::default(),
            cutoff: Default::default(),
            param2: Default::default(),
            coefficients: CoefficientSet::default(),
            sample_m1: Default::default(),
            sample_m2: Default::default(),
            output_m1: Default::default(),
            output_m2: Default::default(),
        }
    }

    pub fn new_with(params: &FilterParams, sample_rate: usize) -> Self {
        let mut r = Self::default_fields();
        r.filter_type = FilterParams::type_for(*params);
        r.sample_rate = sample_rate;
        match *params {
            FilterParams::None => {}
            FilterParams::LowPass { cutoff, q } => {
                r.cutoff = cutoff;
                r.param2 = q;
            }
            FilterParams::HighPass { cutoff, q } => {
                r.cutoff = cutoff;
                r.param2 = q;
            }
            FilterParams::BandPass { cutoff, bandwidth } => {
                r.cutoff = cutoff;
                r.param2 = bandwidth;
            }
            FilterParams::BandStop { cutoff, bandwidth } => {
                r.cutoff = cutoff;
                r.param2 = bandwidth;
            }
            FilterParams::AllPass { cutoff, q } => {
                r.cutoff = cutoff;
                r.param2 = q;
            }
            FilterParams::PeakingEq { cutoff, db_gain } => {
                r.cutoff = cutoff;
                r.param2 = db_gain;
            }
            FilterParams::LowShelf { cutoff, db_gain } => {
                r.cutoff = cutoff;
                r.param2 = db_gain;
            }
            FilterParams::HighShelf { cutoff, db_gain } => {
                r.cutoff = cutoff;
                r.param2 = db_gain;
            }
        }
        r.update_coefficients();
        r
    }

    pub fn new_wrapped_with(params: &FilterParams, sample_rate: usize) -> Rrc<Self> {
        let wrapped = rrc(Self::new_with(params, sample_rate));
        wrapped.borrow_mut().me = rrc_downgrade(&wrapped);
        wrapped
    }

    fn update_coefficients(&mut self) {
        self.coefficients = match self.filter_type {
            FilterType::None => self.rbj_none_coefficients(),
            FilterType::LowPass => self.rbj_low_pass_coefficients(),
            FilterType::HighPass => self.rbj_high_pass_coefficients(),
            FilterType::BandPass => self.rbj_band_pass_coefficients(),
            FilterType::BandStop => self.rbj_band_stop_coefficients(),
            FilterType::AllPass => self.rbj_all_pass_coefficients(),
            FilterType::PeakingEq => self.rbj_peaking_eq_coefficients(),
            FilterType::LowShelf => self.rbj_low_shelf_coefficients(),
            FilterType::HighShelf => self.rbj_high_shelf_coefficients(),
        };
    }

    pub(crate) fn cutoff_hz(&self) -> f32 {
        self.cutoff
    }

    pub(crate) fn set_cutoff_hz(&mut self, hz: f32) {
        self.cutoff = hz;
        self.update_coefficients();
    }

    pub(crate) fn cutoff_pct(&self) -> f32 {
        Self::frequency_to_percent(self.cutoff)
    }

    pub(crate) fn set_cutoff_pct(&mut self, percent: f32) {
        self.set_cutoff_hz(Self::percent_to_frequency(percent));
    }

    // Note that these three are all alises for the same field: param2. This was
    // easier, for now, than some kind of fancy impl per filter type.
    pub fn q(&self) -> f32 {
        self.param2
    }
    pub fn set_q(&mut self, param2: f32) {
        self.param2 = param2;
        self.update_coefficients();
    }

    pub fn db_gain(&self) -> f32 {
        self.param2
    }
    pub fn set_db_gain(&mut self, param2: f32) {
        self.param2 = param2;
        self.update_coefficients();
    }

    pub fn bandwidth(&self) -> f32 {
        self.param2
    }
    pub fn set_bandwidth(&mut self, param2: f32) {
        self.param2 = param2;
        self.update_coefficients();
    }
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

    fn rbj_intermediates_q(sample_rate: usize, cutoff: f32, q: f32) -> (f64, f64, f64, f64) {
        let w0 = 2.0f64 * PI * cutoff as f64 / sample_rate as f64;
        let w0cos = w0.cos();
        let w0sin = w0.sin();
        let alpha = w0sin / (2.0f64 * q as f64);
        (w0, w0cos, w0sin, alpha)
    }

    fn rbj_low_pass_coefficients(&self) -> CoefficientSet {
        let (w0, w0cos, w0sin, alpha) =
            Self::rbj_intermediates_q(self.sample_rate, self.cutoff, self.param2);

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
            Self::rbj_intermediates_q(self.sample_rate, self.cutoff, self.param2);

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
        cutoff: f32,
        bw: f32,
    ) -> (f64, f64, f64, f64) {
        let w0 = 2.0f64 * PI * cutoff as f64 / sample_rate as f64;
        let w0cos = w0.cos();
        let w0sin = w0.sin();
        let alpha = w0sin * (2.0f64.ln() / 2.0 * bw as f64 * w0 / w0.sin()).sinh();
        (w0, w0cos, w0sin, alpha)
    }

    fn rbj_band_pass_coefficients(&self) -> CoefficientSet {
        let (w0, w0cos, w0sin, alpha) =
            Self::rbj_intermediates_bandwidth(self.sample_rate, self.cutoff, self.param2);
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
            Self::rbj_intermediates_bandwidth(self.sample_rate, self.cutoff, self.param2);

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
            Self::rbj_intermediates_q(self.sample_rate, self.cutoff, self.param2);
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
            self.cutoff,
            std::f32::consts::FRAC_1_SQRT_2,
        );
        let a = 10f64.powf(self.param2 as f64 / 10.0f64).sqrt();

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

    fn rbj_low_shelf_coefficients(&self) -> CoefficientSet {
        let a = 10f64.powf(self.param2 as f64 / 10.0f64).sqrt();
        let (_w0, w0cos, _w0sin, alpha) =
            BiQuadFilter::rbj_intermediates_shelving(self.sample_rate, self.cutoff, a, 1.0);

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
        let a = 10f64.powf(self.param2 as f64 / 10.0f64).sqrt();
        let (_w0, w0cos, _w0sin, alpha) =
            BiQuadFilter::rbj_intermediates_shelving(self.sample_rate, self.cutoff, a, 1.0);

        CoefficientSet {
            a0: (a + 1.0) - (a - 1.0) * w0cos + 2.0 * a.sqrt() * alpha,
            a1: 2.0 * ((a - 1.0) - (a + 1.0) * w0cos),
            a2: (a + 1.0) - (a - 1.0) * w0cos - 2.0 * a.sqrt() * alpha,
            b0: a * ((a + 1.0) + (a - 1.0) * w0cos + 2.0 * a.sqrt() * alpha),
            b1: -2.0 * a * ((a - 1.0) + (a + 1.0) * w0cos),
            b2: a * ((a + 1.0) + (a - 1.0) * w0cos - 2.0 * a.sqrt() * alpha),
        }
    }
}

impl SinksAudio for BiQuadFilter {
    fn sources(&self) -> &[Ww<dyn SourcesAudio>] {
        &self.sources
    }
    fn sources_mut(&mut self) -> &mut Vec<Ww<dyn SourcesAudio>> {
        &mut self.sources
    }
}
impl TransformsAudio for BiQuadFilter {
    fn transform_audio(&mut self, _clock: &Clock, input_sample: MonoSample) -> MonoSample {
        let s64 = input_sample as f64;
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
        r as MonoSample
    }
}
impl HasOverhead for BiQuadFilter {
    fn overhead(&self) -> &Overhead {
        &self.overhead
    }

    fn overhead_mut(&mut self) -> &mut Overhead {
        &mut self.overhead
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        control::BiQuadFilterControlParams,
        envelopes::{EnvelopeStep, SteppedEnvelope},
        settings::patches::WaveformType,
        utils::tests::write_source_and_controlled_effect,
    };

    // TODO: these aren't really unit tests. They just spit out files that I
    // listen to in Audacity.
    #[test]
    fn test_filters_with_output_wav() {
        const Q_10: f32 = 10.0;
        const ONE_OCTAVE: f32 = 1.0;
        const SIX_DB: f32 = 6.0;
        const SAMPLE_RATE: usize = 44100;

        let tests = vec![
            (
                "rbj_noise_lpf_1KHz_min_q",
                &FilterParams::LowPass {
                    cutoff: 1000.,
                    q: std::f32::consts::FRAC_1_SQRT_2,
                },
            ),
            (
                "rbj_noise_lpf_1KHz_q10",
                &FilterParams::LowPass {
                    cutoff: 1000.,
                    q: Q_10,
                },
            ),
            (
                "rbj_noise_hpf_1KHz_min_q",
                &FilterParams::HighPass {
                    cutoff: 1000.,
                    q: std::f32::consts::FRAC_1_SQRT_2,
                },
            ),
            (
                "rbj_noise_hpf_1KHz_q10",
                &FilterParams::HighPass {
                    cutoff: 1000.,
                    q: Q_10,
                },
            ),
            (
                "rbj_noise_bpf_1KHz_bw1",
                &FilterParams::BandPass {
                    cutoff: 1000.,
                    bandwidth: ONE_OCTAVE,
                },
            ),
            (
                "rbj_noise_bsf_1KHz_bw1",
                &FilterParams::BandStop {
                    cutoff: 1000.,
                    bandwidth: ONE_OCTAVE,
                },
            ),
            (
                "rbj_noise_apf_1KHz_min_q",
                &FilterParams::AllPass {
                    cutoff: 1000.0,
                    q: std::f32::consts::FRAC_1_SQRT_2,
                },
            ),
            (
                "rbj_noise_peaking_eq_1KHz_6db",
                &FilterParams::PeakingEq {
                    cutoff: 1000.,
                    db_gain: SIX_DB,
                },
            ),
            (
                "rbj_noise_low_shelf_1KHz_6db",
                &FilterParams::LowShelf {
                    cutoff: 1000.,
                    db_gain: SIX_DB,
                },
            ),
            (
                "rbj_noise_high_shelf_1KHz_6db",
                &FilterParams::HighShelf {
                    cutoff: 1000.,
                    db_gain: SIX_DB,
                },
            ),
        ];
        for t in tests {
            write_source_and_controlled_effect(
                t.0,
                WaveformType::Noise,
                Some(BiQuadFilter::new_wrapped_with(t.1, SAMPLE_RATE)),
                None,
            );
        }
    }

    #[test]
    fn test_dynamic_cutoff() {
        const SAMPLE_RATE: usize = 44100;
        let tests = vec![
            (
                "rbj_sawtooth_middle_c_lpf_dynamic_40Hz_8KHz_min_q",
                &FilterParams::LowPass {
                    cutoff: 1000.,
                    q: std::f32::consts::FRAC_1_SQRT_2,
                },
                BiQuadFilterControlParams::CutoffPct,
                40.0,
                8000.0,
            ),
            (
                "rbj_sawtooth_middle_c_lpf_dynamic_40Hz_8KHz_min_q",
                &FilterParams::LowPass {
                    cutoff: 1000.,
                    q: std::f32::consts::FRAC_1_SQRT_2,
                },
                BiQuadFilterControlParams::CutoffPct,
                40.0,
                8000.0,
            ),
            (
                "rbj_sawtooth_middle_c_lpf_1KHz_dynamic_min_q_20",
                &FilterParams::LowPass {
                    cutoff: 1000.,
                    q: std::f32::consts::FRAC_1_SQRT_2,
                },
                BiQuadFilterControlParams::Q, ////// NOTE! This is Q! Not cutoff!
                std::f32::consts::FRAC_1_SQRT_2,
                std::f32::consts::FRAC_1_SQRT_2 * 20.0,
            ),
            (
                "rbj_sawtooth_middle_c_hpf_dynamic_8KHz_40Hz_min_q",
                &FilterParams::HighPass {
                    cutoff: 1000.,
                    q: std::f32::consts::FRAC_1_SQRT_2,
                },
                BiQuadFilterControlParams::CutoffPct,
                8000.0,
                40.0,
            ),
            (
                "rbj_sawtooth_middle_c_bpf_dynamic_40Hz_8KHz_min_q",
                &FilterParams::BandPass {
                    cutoff: 1000.,
                    bandwidth: std::f32::consts::FRAC_1_SQRT_2,
                },
                BiQuadFilterControlParams::CutoffPct,
                40.0,
                8000.0,
            ),
            (
                "rbj_sawtooth_middle_c_bsf_dynamic_40Hz_1.5KHz_min_q",
                &FilterParams::BandStop {
                    cutoff: 1000.,
                    bandwidth: std::f32::consts::FRAC_1_SQRT_2,
                },
                BiQuadFilterControlParams::CutoffPct,
                40.0,
                1500.0,
            ),
        ];
        for t in tests {
            let _effect = BiQuadFilter::new_wrapped_with(t.1, SAMPLE_RATE);
            let mut envelope = Box::new(SteppedEnvelope::new_with_time_unit(
                crate::clock::ClockTimeUnit::Seconds,
            ));
            let (start_value, end_value) = match t.2 {
                BiQuadFilterControlParams::CutoffPct => (
                    BiQuadFilter::frequency_to_percent(t.3),
                    BiQuadFilter::frequency_to_percent(t.4),
                ),
                BiQuadFilterControlParams::Q => (t.3, t.4),
                _ => todo!(),
            };
            envelope.push_step(EnvelopeStep::new_with_duration(
                0.0,
                2.0,
                start_value,
                end_value,
                crate::envelopes::EnvelopeFunction::Linear,
            ));
            // TODO: re-enable this. I'm too tired to do it right now.
            //
            // let control_sink_opt = effect.borrow_mut().message_for(&t.2.to_string());
            // let controller = rrc(TestControlSourceContinuous::new_with(envelope));
            // controller.borrow_mut().add_control_sink(control_sink);
            // write_source_and_controlled_effect(
            //     t.0,
            //     WaveformType::Sawtooth,
            //     Some(effect),
            //     Some(controller),
            // );
        }
    }
}
