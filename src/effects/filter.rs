use crate::{
    common::{MonoSample, Rrc, Ww},
    traits::{IsEffect, IsMutable, SinksAudio, SourcesAudio, TransformsAudio},
};
use std::{cell::RefCell, f64::consts::PI, rc::Rc};

#[derive(Debug, Clone, Copy, Default)]
pub enum FilterType {
    #[default]
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

#[derive(Debug, Default)]
pub struct Filter {
    pub(crate) me: Ww<Self>,
    sources: Vec<Ww<dyn SourcesAudio>>,
    is_muted: bool,
    filter_type: FilterType,
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
impl IsEffect for Filter {}

#[allow(dead_code)]
#[allow(unused_variables)]
impl Filter {
    pub const FREQUENCY_TO_LINEAR_BASE: f32 = 800.0;
    pub const FREQUENCY_TO_LINEAR_COEFFICIENT: f32 = 25.0;

    pub(crate) const CONTROL_PARAM_CUTOFF: &str = "cutoff";
    pub(crate) const CONTROL_PARAM_Q: &str = "q";
    pub(crate) const CONTROL_PARAM_BANDWIDTH: &str = "bandwidth";
    pub(crate) const CONTROL_PARAM_DB_GAIN: &str = "db-gain";

    // https://docs.google.com/spreadsheets/d/1uQylh2h77-fuJ6OM0vjF7yjRXflLFP0yQEnv5wbaP2c/edit#gid=0
    // =LOGEST(Sheet1!B2:B23, Sheet1!A2:A23,true, false)
    // Column A is 24db filter percentages from all the patches
    // Column B is envelope-filter percentages from all the patches
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

        // I was stressed out about slightly negative values, but then
        // I decided that adjusting the log numbers to handle more edge
        // cases wasn't going to make a practical difference. So I'm
        // clamping to [0, 1].
        (frequency / Self::FREQUENCY_TO_LINEAR_COEFFICIENT)
            .log(Self::FREQUENCY_TO_LINEAR_BASE)
            .clamp(0.0, 1.0)
    }

    pub fn new(filter_type: &FilterType) -> Self {
        let mut r = Self {
            ..Default::default()
        };
        r.recalculate_coefficients(filter_type);
        r
    }

    pub fn new_wrapped_with(filter_type: &FilterType) -> Rrc<Self> {
        // TODO: Rc::new_cyclic() should make this easier, but I couldn't get the syntax right.
        // https://doc.rust-lang.org/std/rc/struct.Rc.html#method.new_cyclic

        let wrapped = Rc::new(RefCell::new(Self::new(filter_type)));
        wrapped.borrow_mut().me = Rc::downgrade(&wrapped);
        wrapped
    }

    fn recalculate_coefficients(&mut self, new_filter_type: &FilterType) {
        (self.a0, self.a1, self.a2, self.b0, self.b1, self.b2) = match *new_filter_type {
            FilterType::None => (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            FilterType::LowPass {
                sample_rate,
                cutoff,
                q,
            } => {
                self.sample_rate = sample_rate;
                self.cutoff = cutoff;
                Self::rbj_low_pass_coefficients(sample_rate, cutoff, q)
            }
            FilterType::HighPass {
                sample_rate,
                cutoff,
                q,
            } => {
                self.sample_rate = sample_rate;
                self.cutoff = cutoff;
                Self::rbj_high_pass_coefficients(sample_rate, cutoff, q)
            }
            FilterType::BandPass {
                sample_rate,
                cutoff,
                bandwidth,
            } => {
                self.sample_rate = sample_rate;
                self.cutoff = cutoff;
                Self::rbj_band_pass_coefficients(sample_rate, cutoff, bandwidth)
            }
            FilterType::BandStop {
                sample_rate,
                cutoff,
                bandwidth,
            } => {
                self.sample_rate = sample_rate;
                self.cutoff = cutoff;
                Self::rbj_band_stop_coefficients(sample_rate, cutoff, bandwidth)
            }
            FilterType::AllPass {
                sample_rate,
                cutoff,
                q,
            } => {
                self.sample_rate = sample_rate;
                self.cutoff = cutoff;
                Self::rbj_all_pass_coefficients(sample_rate, cutoff, q)
            }
            FilterType::PeakingEq {
                sample_rate,
                cutoff,
                db_gain,
            } => {
                self.sample_rate = sample_rate;
                self.cutoff = cutoff;
                Self::rbj_peaking_eq_coefficients(sample_rate, cutoff, db_gain)
            }
            FilterType::LowShelf {
                sample_rate,
                cutoff,
                db_gain,
            } => {
                self.sample_rate = sample_rate;
                self.cutoff = cutoff;
                Self::rbj_low_shelf_coefficients(sample_rate, cutoff, db_gain)
            }
            FilterType::HighShelf {
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
            FilterType::None => FilterType::None,
            FilterType::LowPass {
                sample_rate,
                cutoff,
                q,
            } => FilterType::LowPass {
                sample_rate,
                cutoff: new_cutoff,
                q,
            },
            FilterType::HighPass {
                sample_rate,
                cutoff,
                q,
            } => FilterType::LowPass {
                sample_rate,
                cutoff: new_cutoff,
                q,
            },
            FilterType::BandPass {
                sample_rate,
                cutoff,
                bandwidth,
            } => FilterType::BandPass {
                sample_rate,
                cutoff: new_cutoff,
                bandwidth,
            },
            FilterType::BandStop {
                sample_rate,
                cutoff,
                bandwidth,
            } => FilterType::BandStop {
                sample_rate,
                cutoff: new_cutoff,
                bandwidth,
            },
            FilterType::AllPass {
                sample_rate,
                cutoff,
                q,
            } => FilterType::AllPass {
                sample_rate,
                cutoff: new_cutoff,
                q,
            },
            FilterType::PeakingEq {
                sample_rate,
                cutoff,
                db_gain,
            } => FilterType::PeakingEq {
                sample_rate,
                cutoff: new_cutoff,
                db_gain,
            },
            FilterType::LowShelf {
                sample_rate,
                cutoff,
                db_gain,
            } => FilterType::LowShelf {
                sample_rate,
                cutoff: new_cutoff,
                db_gain,
            },
            FilterType::HighShelf {
                sample_rate,
                cutoff,
                db_gain,
            } => FilterType::HighShelf {
                sample_rate,
                cutoff: new_cutoff,
                db_gain,
            },
        };
        self.recalculate_coefficients(&new_filter_type)
    }

    pub fn set_q(&mut self, new_val: f32) {
        let new_filter_type = match self.filter_type {
            FilterType::None => FilterType::None,
            FilterType::LowPass {
                sample_rate,
                cutoff,
                q,
            } => FilterType::LowPass {
                sample_rate,
                cutoff,
                q: new_val,
            },
            FilterType::HighPass {
                sample_rate,
                cutoff,
                q,
            } => FilterType::LowPass {
                sample_rate,
                cutoff,
                q: new_val,
            },
            FilterType::BandPass {
                sample_rate,
                cutoff,
                bandwidth,
            } => FilterType::BandPass {
                sample_rate,
                cutoff,
                bandwidth: new_val,
            },
            FilterType::BandStop {
                sample_rate,
                cutoff,
                bandwidth,
            } => FilterType::BandStop {
                sample_rate,
                cutoff,
                bandwidth: new_val,
            },
            FilterType::AllPass {
                sample_rate,
                cutoff,
                q,
            } => FilterType::AllPass {
                sample_rate,
                cutoff,
                q: new_val,
            },
            FilterType::PeakingEq {
                sample_rate,
                cutoff,
                db_gain,
            } => FilterType::PeakingEq {
                sample_rate,
                cutoff,
                db_gain: new_val,
            },
            FilterType::LowShelf {
                sample_rate,
                cutoff,
                db_gain,
            } => FilterType::LowShelf {
                sample_rate,
                cutoff,
                db_gain: new_val,
            },
            FilterType::HighShelf {
                sample_rate,
                cutoff,
                db_gain,
            } => FilterType::HighShelf {
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
        let (w0, w0cos, w0sin, alpha) = Filter::rbj_intermediates_q(sample_rate, cutoff, q);

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
        let (w0, w0cos, w0sin, alpha) = Filter::rbj_intermediates_q(sample_rate, cutoff, q);

        (
            1.0 + alpha,
            -2.0f64 * w0cos,
            1.0 - alpha,
            (1.0 + w0cos) / 2.0f64,
            -(1.0 + w0cos),
            (1.0 + w0cos) / 2.0f64,
        )
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

    fn rbj_band_pass_coefficients(
        sample_rate: usize,
        cutoff: f32,
        bandwidth: f32,
    ) -> (f64, f64, f64, f64, f64, f64) {
        let (w0, w0cos, w0sin, alpha) =
            Filter::rbj_intermediates_bandwidth(sample_rate, cutoff, bandwidth);
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
            Filter::rbj_intermediates_bandwidth(sample_rate, cutoff, bandwidth);

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
        let (w0, w0cos, w0sin, alpha) = Filter::rbj_intermediates_q(sample_rate, cutoff, q);
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
            Filter::rbj_intermediates_q(sample_rate, cutoff, std::f32::consts::FRAC_1_SQRT_2);
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
            Filter::rbj_intermediates_shelving(sample_rate, cutoff, a, 1.0);

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
            Filter::rbj_intermediates_shelving(sample_rate, cutoff, a, 1.0);

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

impl SinksAudio for Filter {
    fn sources(&self) -> &[Ww<dyn SourcesAudio>] {
        &self.sources
    }
    fn sources_mut(&mut self) -> &mut Vec<Ww<dyn SourcesAudio>> {
        &mut self.sources
    }
}

impl TransformsAudio for Filter {
    fn transform_audio(&mut self, input_sample: MonoSample) -> MonoSample {
        let s64 = input_sample as f64;
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
impl IsMutable for Filter {
    fn is_muted(&self) -> bool {
        self.is_muted
    }

    fn set_muted(&mut self, is_muted: bool) {
        self.is_muted = is_muted;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        common::{rrc, WaveformType},
        envelopes::{EnvelopeStep, SteppedEnvelope},
        traits::{MakesControlSink, SourcesControl},
        utils::tests::{write_source_and_controlled_effect, TestControlSourceContinuous},
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
                &FilterType::LowPass {
                    sample_rate: SAMPLE_RATE,
                    cutoff: 1000.,
                    q: std::f32::consts::FRAC_1_SQRT_2,
                },
            ),
            (
                "rbj_noise_lpf_1KHz_q10",
                &FilterType::LowPass {
                    sample_rate: SAMPLE_RATE,
                    cutoff: 1000.,
                    q: Q_10,
                },
            ),
            (
                "rbj_noise_hpf_1KHz_min_q",
                &FilterType::HighPass {
                    sample_rate: SAMPLE_RATE,
                    cutoff: 1000.,
                    q: std::f32::consts::FRAC_1_SQRT_2,
                },
            ),
            (
                "rbj_noise_hpf_1KHz_q10",
                &FilterType::HighPass {
                    sample_rate: SAMPLE_RATE,
                    cutoff: 1000.,
                    q: Q_10,
                },
            ),
            (
                "rbj_noise_bpf_1KHz_bw1",
                &FilterType::BandPass {
                    sample_rate: SAMPLE_RATE,
                    cutoff: 1000.,
                    bandwidth: ONE_OCTAVE,
                },
            ),
            (
                "rbj_noise_bsf_1KHz_bw1",
                &FilterType::BandStop {
                    sample_rate: SAMPLE_RATE,
                    cutoff: 1000.,
                    bandwidth: ONE_OCTAVE,
                },
            ),
            (
                "rbj_noise_apf_1KHz_min_q",
                &FilterType::AllPass {
                    sample_rate: SAMPLE_RATE,
                    cutoff: 1000.0,
                    q: std::f32::consts::FRAC_1_SQRT_2,
                },
            ),
            (
                "rbj_noise_peaking_eq_1KHz_6db",
                &FilterType::PeakingEq {
                    sample_rate: SAMPLE_RATE,
                    cutoff: 1000.,
                    db_gain: SIX_DB,
                },
            ),
            (
                "rbj_noise_low_shelf_1KHz_6db",
                &FilterType::LowShelf {
                    sample_rate: SAMPLE_RATE,
                    cutoff: 1000.,
                    db_gain: SIX_DB,
                },
            ),
            (
                "rbj_noise_high_shelf_1KHz_6db",
                &FilterType::HighShelf {
                    sample_rate: SAMPLE_RATE,
                    cutoff: 1000.,
                    db_gain: SIX_DB,
                },
            ),
        ];
        for t in tests {
            write_source_and_controlled_effect(
                t.0,
                WaveformType::Noise,
                Some(Filter::new_wrapped_with(t.1)),
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
                &FilterType::LowPass {
                    sample_rate: SAMPLE_RATE,
                    cutoff: 1000.,
                    q: std::f32::consts::FRAC_1_SQRT_2,
                },
                Filter::CONTROL_PARAM_CUTOFF,
                40.0,
                8000.0,
            ),
            (
                "rbj_sawtooth_middle_c_lpf_dynamic_40Hz_8KHz_min_q",
                &FilterType::LowPass {
                    sample_rate: SAMPLE_RATE,
                    cutoff: 1000.,
                    q: std::f32::consts::FRAC_1_SQRT_2,
                },
                Filter::CONTROL_PARAM_CUTOFF,
                40.0,
                8000.0,
            ),
            (
                "rbj_sawtooth_middle_c_lpf_1KHz_dynamic_min_q_20",
                &FilterType::LowPass {
                    sample_rate: SAMPLE_RATE,
                    cutoff: 1000.,
                    q: std::f32::consts::FRAC_1_SQRT_2,
                },
                Filter::CONTROL_PARAM_Q, ////// NOTE! This is Q! Not cutoff!
                std::f32::consts::FRAC_1_SQRT_2,
                std::f32::consts::FRAC_1_SQRT_2 * 20.0,
            ),
            (
                "rbj_sawtooth_middle_c_hpf_dynamic_8KHz_40Hz_min_q",
                &FilterType::HighPass {
                    sample_rate: SAMPLE_RATE,
                    cutoff: 1000.,
                    q: std::f32::consts::FRAC_1_SQRT_2,
                },
                Filter::CONTROL_PARAM_CUTOFF,
                8000.0,
                40.0,
            ),
            (
                "rbj_sawtooth_middle_c_bpf_dynamic_40Hz_8KHz_min_q",
                &FilterType::BandPass {
                    sample_rate: SAMPLE_RATE,
                    cutoff: 1000.,
                    bandwidth: std::f32::consts::FRAC_1_SQRT_2,
                },
                Filter::CONTROL_PARAM_CUTOFF,
                40.0,
                8000.0,
            ),
            (
                "rbj_sawtooth_middle_c_bsf_dynamic_40Hz_1.5KHz_min_q",
                &FilterType::BandStop {
                    sample_rate: SAMPLE_RATE,
                    cutoff: 1000.,
                    bandwidth: std::f32::consts::FRAC_1_SQRT_2,
                },
                Filter::CONTROL_PARAM_CUTOFF,
                40.0,
                1500.0,
            ),
        ];
        for t in tests {
            let effect = Filter::new_wrapped_with(t.1);
            let mut envelope = Box::new(SteppedEnvelope::new_with_time_unit(
                crate::clock::ClockTimeUnit::Seconds,
            ));
            let (start_value, end_value) = match t.2 {
                Filter::CONTROL_PARAM_CUTOFF => (
                    Filter::frequency_to_percent(t.3),
                    Filter::frequency_to_percent(t.4),
                ),
                Filter::CONTROL_PARAM_Q => (t.3, t.4),
                _ => todo!(),
            };
            envelope.push_step(EnvelopeStep::new_with_duration(
                0.0,
                2.0,
                start_value,
                end_value,
                crate::envelopes::EnvelopeFunction::Linear,
            ));
            let control_sink_opt = effect.borrow_mut().make_control_sink(t.2);
            if let Some(control_sink) = control_sink_opt {
                let controller = rrc(TestControlSourceContinuous::new_with(envelope));
                controller.borrow_mut().add_control_sink(control_sink);
                write_source_and_controlled_effect(
                    t.0,
                    WaveformType::Sawtooth,
                    Some(effect),
                    Some(controller),
                );
            }
        }
    }
}
