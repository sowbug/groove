use std::f64::consts::PI;

use crate::common::MonoSample;

use super::{SinksAudio, SinksControl, SinksControlParamType, SourcesAudio, TransformsAudio};

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

#[derive(Default)]
pub struct MiniFilter2 {
    sources: Vec<Box<dyn SourcesAudio>>,
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

impl SinksAudio for MiniFilter2 {
    fn sources(&mut self) -> &mut Vec<Box<dyn SourcesAudio>> {
        &mut self.sources
    }
}

impl SourcesAudio for MiniFilter2 {
    fn source_audio(&mut self, time_seconds: f32) -> MonoSample {
        let input = self.gather_source_audio(time_seconds);
        self.transform_audio(input)
    }
}

impl TransformsAudio for MiniFilter2 {
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

impl SinksControl for MiniFilter2 {
    fn handle_control(
        &mut self,
        _time_seconds: f32,
        param_type: super::SinksControlParamType,
        new_value: f32,
    ) {
        match param_type {
            SinksControlParamType::Primary => {
                self.set_cutoff(new_value);
            }
            SinksControlParamType::Secondary => {
                self.set_q(new_value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        common::{MidiMessage, MidiNote, WaveformType},
        preset::OscillatorPreset,
        primitives::{
            oscillators::MiniOscillator,
            tests::write_effect_to_file,
            SinksControl,
            SinksControlParamType::{Primary, Secondary},
            SourcesControl,
        },
    };

    use super::*;
    const SAMPLE_RATE: usize = 44100;

    enum TestFilterControllerParam {
        Cutoff,
        Q,
    }

    struct TestFilterController {
        sinks: Vec<Box<dyn SinksControl>>,
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
                sinks: Vec::new(),
                param,
                param_start,
                param_end,
                duration,
                time_start: -1.0f32,
            }
        }
    }

    impl<'a> SourcesControl for TestFilterController {
        fn control_sinks(&mut self) -> &mut Vec<Box<dyn crate::primitives::SinksControl>> {
            &mut self.sinks
        }

        fn control(&mut self, time_seconds: f32) {
            if self.time_start < 0.0 {
                self.time_start = time_seconds;
            }
            if self.param_end != self.param_start {
                for sink in self.sinks.iter_mut() {
                    sink.handle_control(
                        time_seconds,
                        match self.param {
                            TestFilterControllerParam::Cutoff => Primary,
                            TestFilterControllerParam::Q => Secondary,
                        },
                        self.param_start
                            + ((time_seconds - self.time_start) / self.duration)
                                * (self.param_end - self.param_start),
                    );
                }
            }
        }
    }

    #[derive(Default)]
    struct TestNullController {
        control_sinks: Vec<Box<dyn SinksControl>>,
    }

    impl TestNullController {
        fn new() -> Self {
            Self {
                ..Default::default()
            }
        }
    }

    impl SourcesControl for TestNullController {
        fn control_sinks(&mut self) -> &mut Vec<Box<dyn SinksControl>> {
            &mut self.control_sinks
        }
        fn control(&mut self, _time_seconds: f32) {}
    }

    fn add_noise_and_write_filter_to_file(
        filter: &mut MiniFilter2,
        controller: &mut dyn SourcesControl,
        basename: &str,
    ) {
        let source = Box::new(MiniOscillator::new(WaveformType::Noise));
        filter.add_audio_source(source);
        write_effect_to_file(filter, controller, basename);
    }

    #[test]
    fn test_mini_filter2() {
        const Q_10: f32 = 10.0;
        const ONE_OCTAVE: f32 = 1.0;
        const SIX_DB: f32 = 6.0;

        add_noise_and_write_filter_to_file(
            &mut MiniFilter2::new(&MiniFilter2Type::LowPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                q: std::f32::consts::FRAC_1_SQRT_2,
            }),
            &mut TestNullController::new(),
            "rbj_noise_lpf_1KHz_min_q",
        );

        add_noise_and_write_filter_to_file(
            &mut MiniFilter2::new(&MiniFilter2Type::LowPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                q: Q_10,
            }),
            &mut TestNullController::new(),
            "rbj_noise_lpf_1KHz_q10",
        );

        add_noise_and_write_filter_to_file(
            &mut MiniFilter2::new(&MiniFilter2Type::HighPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                q: std::f32::consts::FRAC_1_SQRT_2,
            }),
            &mut TestNullController::new(),
            "rbj_noise_hpf_1KHz_min_q",
        );
        add_noise_and_write_filter_to_file(
            &mut MiniFilter2::new(&MiniFilter2Type::HighPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                q: Q_10,
            }),
            &mut TestNullController::new(),
            "rbj_noise_hpf_1KHz_q10",
        );
        add_noise_and_write_filter_to_file(
            &mut MiniFilter2::new(&MiniFilter2Type::BandPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                bandwidth: ONE_OCTAVE,
            }),
            &mut TestNullController::new(),
            "rbj_noise_bpf_1KHz_bw1",
        );
        add_noise_and_write_filter_to_file(
            &mut MiniFilter2::new(&MiniFilter2Type::BandStop {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                bandwidth: ONE_OCTAVE,
            }),
            &mut TestNullController::new(),
            "rbj_noise_bsf_1KHz_bw1",
        );
        add_noise_and_write_filter_to_file(
            &mut MiniFilter2::new(&MiniFilter2Type::AllPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.0,
                q: std::f32::consts::FRAC_1_SQRT_2,
            }),
            &mut TestNullController::new(),
            "rbj_noise_apf_1KHz_min_q",
        );
        add_noise_and_write_filter_to_file(
            &mut MiniFilter2::new(&MiniFilter2Type::PeakingEq {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                db_gain: SIX_DB,
            }),
            &mut TestNullController::new(),
            "rbj_noise_peaking_eq_1KHz_6db",
        );
        add_noise_and_write_filter_to_file(
            &mut MiniFilter2::new(&MiniFilter2Type::LowShelf {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                db_gain: SIX_DB,
            }),
            &mut TestNullController::new(),
            "rbj_noise_low_shelf_1KHz_6db",
        );
        add_noise_and_write_filter_to_file(
            &mut MiniFilter2::new(&MiniFilter2Type::HighShelf {
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
        let mut source = MiniOscillator::new_from_preset(&OscillatorPreset {
            waveform: WaveformType::Sawtooth,
            ..Default::default()
        });
        source.set_frequency(MidiMessage::note_to_frequency(MidiNote::C4 as u8));

        add_noise_and_write_filter_to_file(
            &mut MiniFilter2::new(&MiniFilter2Type::LowPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                q: std::f32::consts::FRAC_1_SQRT_2,
            }),
            &mut TestFilterController::new(TestFilterControllerParam::Cutoff, 40.0, 8000.0, 2.0),
            "rbj_sawtooth_middle_c_lpf_dynamic_40Hz_8KHz_min_q",
        );
        add_noise_and_write_filter_to_file(
            &mut MiniFilter2::new(&MiniFilter2Type::LowPass {
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
            &mut MiniFilter2::new(&MiniFilter2Type::HighPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                q: std::f32::consts::FRAC_1_SQRT_2,
            }),
            &mut TestFilterController::new(TestFilterControllerParam::Cutoff, 8000.0, 40.0, 2.0),
            "rbj_sawtooth_middle_c_hpf_dynamic_8KHz_40Hz_min_q",
        );
        add_noise_and_write_filter_to_file(
            &mut MiniFilter2::new(&MiniFilter2Type::BandPass {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                bandwidth: std::f32::consts::FRAC_1_SQRT_2,
            }),
            &mut TestFilterController::new(TestFilterControllerParam::Cutoff, 40.0, 8000.0, 2.0),
            "rbj_sawtooth_middle_c_bpf_dynamic_40Hz_8KHz_min_q",
        );
        add_noise_and_write_filter_to_file(
            &mut MiniFilter2::new(&MiniFilter2Type::BandStop {
                sample_rate: SAMPLE_RATE,
                cutoff: 1000.,
                bandwidth: std::f32::consts::FRAC_1_SQRT_2,
            }),
            &mut TestFilterController::new(TestFilterControllerParam::Cutoff, 40.0, 1500.0, 2.0),
            "rbj_sawtooth_middle_c_bsf_dynamic_40Hz_1.5KHz_min_q",
        );
    }
}
