use std::{cell::RefCell, f64::consts::PI, rc::Rc};

use crate::{
    common::MonoSample,
    primitives::clock::Clock,
    traits::{
        IsEffect, SinksAudio, SinksControl, SinksControlParam, SourcesAudio, TransformsAudio,
    },
};

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
    sources: Vec<Rc<RefCell<dyn SourcesAudio>>>,
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

    // https://docs.google.com/spreadsheets/d/1uQylh2h77-fuJ6OM0vjF7yjRXflLFP0yQEnv5wbaP2c/edit#gid=0
    // =LOGEST(Sheet1!B2:B23, Sheet1!A2:A23,true, false)
    // Column A is 24db filter percentages from all the patches
    // Column B is envelope-filter percentages from all the patches
    pub fn percent_to_frequency(percentage: f32) -> f32 {
        debug_assert!((0.0..=1.0).contains(&percentage));
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
    fn sources(&mut self) -> &mut Vec<Rc<RefCell<dyn SourcesAudio>>> {
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

impl SinksControl for Filter {
    fn handle_control(&mut self, _clock: &Clock, param: &SinksControlParam) {
        match param {
            SinksControlParam::Primary { value } => {
                self.set_cutoff(Self::percent_to_frequency(*value));
            }
            SinksControlParam::Secondary { value } => {
                self.set_q(*value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        cell::RefCell,
        rc::{Rc, Weak},
    };

    use crate::{
        common::{MidiMessage, MidiNote, WaveformType},
        preset::OscillatorPreset,
        primitives::clock::Clock,
        primitives::oscillators::Oscillator,
        traits::{
            tests::write_effect_to_file,
            IsController, SinksAudio, SinksControl,
            SinksControlParam::{self},
            SourcesControl, WatchesClock,
        },
    };

    use super::*;
    const SAMPLE_RATE: usize = 44100;

    #[derive(Copy, Clone, Debug, Default)]
    enum TestFilterControllerParam {
        #[default]
        Cutoff,
        Q,
    }

    #[derive(Debug)]
    struct TestFilterController {
        control_sinks: Vec<Weak<RefCell<dyn SinksControl>>>,
        param: TestFilterControllerParam,
        param_start: f32,
        param_end: f32,
        duration: f32,

        time_start: f32,
    }

    impl TestFilterController {
        pub fn new(
            param: TestFilterControllerParam,
            param_start: f32,
            param_end: f32,
            duration: f32,
        ) -> Self {
            Self {
                control_sinks: Vec::new(),
                param,
                param_start,
                param_end,
                duration,
                time_start: -1.0f32,
            }
        }
    }

    impl SourcesControl for TestFilterController {
        fn control_sinks(&self) -> &[Weak<RefCell<dyn SinksControl>>] {
            &self.control_sinks
        }

        fn control_sinks_mut(&mut self) -> &mut Vec<Weak<RefCell<dyn SinksControl>>> {
            &mut self.control_sinks
        }
    }

    impl WatchesClock for TestFilterController {
        fn tick(&mut self, clock: &Clock) -> bool {
            if self.time_start < 0.0 {
                self.time_start = clock.seconds;
            }
            if self.param_end != self.param_start {
                let param = self.param;
                let value = self.param_start
                    + ((clock.seconds - self.time_start) / self.duration)
                        * (self.param_end - self.param_start);
                let sink_param = match param {
                    TestFilterControllerParam::Cutoff => SinksControlParam::Primary { value },
                    TestFilterControllerParam::Q => SinksControlParam::Secondary { value },
                };
                self.issue_control(clock, &sink_param);
            }
            true
        }
    }

    impl IsController for TestFilterController {}

    #[derive(Debug, Default)]
    struct TestNullController {
        control_sinks: Vec<Weak<RefCell<dyn SinksControl>>>,
    }

    impl TestNullController {
        fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }

    impl SourcesControl for TestNullController {
        fn control_sinks(&self) -> &[Weak<RefCell<dyn SinksControl>>] {
            &self.control_sinks
        }
        fn control_sinks_mut(&mut self) -> &mut Vec<Weak<RefCell<dyn SinksControl>>> {
            &mut self.control_sinks
        }
    }

    impl WatchesClock for TestNullController {
        fn tick(&mut self, _clock: &Clock) -> bool {
            true
        }
    }

    impl IsController for TestNullController {}

    fn add_noise_and_write_filter_to_file(
        filter: &mut Filter,
        controller: &mut dyn IsController,
        basename: &str,
    ) {
        let source = Rc::new(RefCell::new(Oscillator::new_with(WaveformType::Noise)));
        filter.add_audio_source(source);
        write_effect_to_file(filter, controller, basename);
    }

    #[test]
    fn test_mini_filter2() {
        const Q_10: f32 = 10.0;
        const ONE_OCTAVE: f32 = 1.0;
        const SIX_DB: f32 = 6.0;

        add_noise_and_write_filter_to_file(
            &mut Filter::new(&FilterType::LowPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                q: std::f32::consts::FRAC_1_SQRT_2,
            }),
            &mut TestNullController::new(),
            "rbj_noise_lpf_1KHz_min_q",
        );

        add_noise_and_write_filter_to_file(
            &mut Filter::new(&FilterType::LowPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                q: Q_10,
            }),
            &mut TestNullController::new(),
            "rbj_noise_lpf_1KHz_q10",
        );

        add_noise_and_write_filter_to_file(
            &mut Filter::new(&FilterType::HighPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                q: std::f32::consts::FRAC_1_SQRT_2,
            }),
            &mut TestNullController::new(),
            "rbj_noise_hpf_1KHz_min_q",
        );
        add_noise_and_write_filter_to_file(
            &mut Filter::new(&FilterType::HighPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                q: Q_10,
            }),
            &mut TestNullController::new(),
            "rbj_noise_hpf_1KHz_q10",
        );
        add_noise_and_write_filter_to_file(
            &mut Filter::new(&FilterType::BandPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                bandwidth: ONE_OCTAVE,
            }),
            &mut TestNullController::new(),
            "rbj_noise_bpf_1KHz_bw1",
        );
        add_noise_and_write_filter_to_file(
            &mut Filter::new(&FilterType::BandStop {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                bandwidth: ONE_OCTAVE,
            }),
            &mut TestNullController::new(),
            "rbj_noise_bsf_1KHz_bw1",
        );
        add_noise_and_write_filter_to_file(
            &mut Filter::new(&FilterType::AllPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.0,
                q: std::f32::consts::FRAC_1_SQRT_2,
            }),
            &mut TestNullController::new(),
            "rbj_noise_apf_1KHz_min_q",
        );
        add_noise_and_write_filter_to_file(
            &mut Filter::new(&FilterType::PeakingEq {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                db_gain: SIX_DB,
            }),
            &mut TestNullController::new(),
            "rbj_noise_peaking_eq_1KHz_6db",
        );
        add_noise_and_write_filter_to_file(
            &mut Filter::new(&FilterType::LowShelf {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                db_gain: SIX_DB,
            }),
            &mut TestNullController::new(),
            "rbj_noise_low_shelf_1KHz_6db",
        );
        add_noise_and_write_filter_to_file(
            &mut Filter::new(&FilterType::HighShelf {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                db_gain: SIX_DB,
            }),
            &mut TestNullController::new(),
            "rbj_noise_high_shelf_1KHz_6db",
        );
    }

    #[test]
    fn test_dynamic_cutoff() {
        let mut source = Oscillator::new_from_preset(&OscillatorPreset {
            waveform: WaveformType::Sawtooth,
            ..Default::default()
        });
        source.set_frequency(MidiMessage::note_to_frequency(MidiNote::C4 as u8));

        add_noise_and_write_filter_to_file(
            &mut Filter::new(&FilterType::LowPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                q: std::f32::consts::FRAC_1_SQRT_2,
            }),
            &mut TestFilterController::new(TestFilterControllerParam::Cutoff, 40.0, 8000.0, 2.0),
            "rbj_sawtooth_middle_c_lpf_dynamic_40Hz_8KHz_min_q",
        );
        add_noise_and_write_filter_to_file(
            &mut Filter::new(&FilterType::LowPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                q: std::f32::consts::FRAC_1_SQRT_2,
            }),
            &mut TestFilterController::new(
                TestFilterControllerParam::Q,
                std::f32::consts::FRAC_1_SQRT_2,
                std::f32::consts::FRAC_1_SQRT_2 * 20.0,
                2.0,
            ),
            "rbj_sawtooth_middle_c_lpf_1KHz_dynamic_min_q_20",
        );
        add_noise_and_write_filter_to_file(
            &mut Filter::new(&FilterType::HighPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                q: std::f32::consts::FRAC_1_SQRT_2,
            }),
            &mut TestFilterController::new(TestFilterControllerParam::Cutoff, 8000.0, 40.0, 2.0),
            "rbj_sawtooth_middle_c_hpf_dynamic_8KHz_40Hz_min_q",
        );
        add_noise_and_write_filter_to_file(
            &mut Filter::new(&FilterType::BandPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                bandwidth: std::f32::consts::FRAC_1_SQRT_2,
            }),
            &mut TestFilterController::new(TestFilterControllerParam::Cutoff, 40.0, 8000.0, 2.0),
            "rbj_sawtooth_middle_c_bpf_dynamic_40Hz_8KHz_min_q",
        );
        add_noise_and_write_filter_to_file(
            &mut Filter::new(&FilterType::BandStop {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                bandwidth: std::f32::consts::FRAC_1_SQRT_2,
            }),
            &mut TestFilterController::new(TestFilterControllerParam::Cutoff, 40.0, 1500.0, 2.0),
            "rbj_sawtooth_middle_c_bsf_dynamic_40Hz_1.5KHz_min_q",
        );
    }
}
