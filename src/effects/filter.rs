use crate::{
    clock::Clock,
    common::MonoSample,
    messages::{EntityMessage, MessageBounds},
    traits::{HasUid, IsEffect, Response, TransformsAudio, Updateable},
};
use std::{f64::consts::PI, marker::PhantomData, str::FromStr};
use strum_macros::{Display, EnumString, FromRepr};

#[derive(Display, Debug, EnumString, FromRepr)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum BiQuadFilterControlParams {
    Bandwidth,
    #[strum(serialize = "cutoff", serialize = "cutoff-pct")]
    CutoffPct,
    DbGain,
    Q,
}

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

#[derive(Clone, Debug, Default)]
struct CoefficientSet {
    a0: f64,
    a1: f64,
    a2: f64,
    b0: f64,
    b1: f64,
    b2: f64,
}

/// https://en.wikipedia.org/wiki/Digital_biquad_filter
#[derive(Clone, Debug)]
pub struct BiQuadFilter<M: MessageBounds> {
    uid: usize,

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

    _phantom: PhantomData<M>,
}
impl<M: MessageBounds> IsEffect for BiQuadFilter<M> {}
impl<M: MessageBounds> TransformsAudio for BiQuadFilter<M> {
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
impl<M: MessageBounds> Updateable for BiQuadFilter<M> {
    default type Message = M;

    #[allow(unused_variables)]
    default fn update(&mut self, clock: &Clock, message: Self::Message) -> Response<Self::Message> {
        Response::none()
    }

    fn param_id_for_name(&self, name: &str) -> usize {
        if let Ok(param) = BiQuadFilterControlParams::from_str(name) {
            param as usize
        } else {
            usize::MAX
        }
    }

    fn set_indexed_param_f32(&mut self, index: usize, value: f32) {
        if let Some(param) = BiQuadFilterControlParams::from_repr(index) {
            match param {
                BiQuadFilterControlParams::Bandwidth => self.set_bandwidth(value),
                BiQuadFilterControlParams::CutoffPct => self.set_cutoff_pct(value),
                BiQuadFilterControlParams::DbGain => self.set_db_gain(value),
                BiQuadFilterControlParams::Q => self.set_q(Self::denormalize_q(value)),
            }
        } else {
            todo!()
        }
    }
}
impl Updateable for BiQuadFilter<EntityMessage> {
    type Message = EntityMessage;

    #[allow(unused_variables)]
    fn update(&mut self, clock: &Clock, message: Self::Message) -> Response<Self::Message> {
        match message {
            Self::Message::UpdateF32(param_id, value) => {
                self.set_indexed_param_f32(param_id, value);
            }

            Self::Message::UpdateParam1U8(value) => {
                self.set_indexed_param_f32(
                    BiQuadFilterControlParams::CutoffPct as usize,
                    value as f32 / 100.0,
                );
            }
            _ => todo!(),
        }
        Response::none()
    }
}
impl<M: MessageBounds> HasUid for BiQuadFilter<M> {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}

// We can't derive this because we need to call recalculate_coefficients(). Is
// there an elegant way to get that done for free without a bunch of repetition?
impl<M: MessageBounds> Default for BiQuadFilter<M> {
    fn default() -> Self {
        let mut r = Self::default_fields();
        r.update_coefficients();
        r
    }
}

#[allow(dead_code)]
#[allow(unused_variables)]
impl<M: MessageBounds> BiQuadFilter<M> {
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
            "Expected range (0.0..=1.0) but got {percentage}",
        );
        Self::FREQUENCY_TO_LINEAR_COEFFICIENT * Self::FREQUENCY_TO_LINEAR_BASE.powf(percentage)
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

    // A placeholder for an intelligent mapping of 0.0..=1.0 to a reasonable Q
    // range
    pub fn denormalize_q(value: f32) -> f32 {
        value * value * 50.0 + 0.707
    }

    /// Returns a new/default struct without calling update_coefficients().
    fn default_fields() -> Self {
        Self {
            uid: usize::default(),
            filter_type: Default::default(),
            sample_rate: Default::default(),
            cutoff: Default::default(),
            param2: Default::default(),
            coefficients: CoefficientSet::default(),
            sample_m1: Default::default(),
            sample_m2: Default::default(),
            output_m1: Default::default(),
            output_m2: Default::default(),
            _phantom: Default::default(),
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

    pub fn cutoff_hz(&self) -> f32 {
        self.cutoff
    }

    pub(crate) fn set_cutoff_hz(&mut self, hz: f32) {
        if self.cutoff != hz {
            self.cutoff = hz;
            self.update_coefficients();
        }
    }

    pub fn cutoff_pct(&self) -> f32 {
        Self::frequency_to_percent(self.cutoff)
    }

    pub(crate) fn set_cutoff_pct(&mut self, percent: f32) {
        self.set_cutoff_hz(Self::percent_to_frequency(percent));
    }

    // Note that these three are all aliases for the same field: param2. This
    // was easier, for now, than some kind of fancy impl per filter type.
    pub fn q(&self) -> f32 {
        self.param2
    }
    pub fn set_q(&mut self, q: f32) {
        if self.param2 != q {
            self.param2 = q;
            self.update_coefficients();
        }
    }

    pub fn db_gain(&self) -> f32 {
        self.param2
    }
    pub fn set_db_gain(&mut self, db_gain: f32) {
        if self.param2 != db_gain {
            self.param2 = db_gain;
            self.update_coefficients();
        }
    }

    pub fn bandwidth(&self) -> f32 {
        self.param2
    }
    pub fn set_bandwidth(&mut self, bandwidth: f32) {
        if self.param2 != bandwidth {
            self.param2 = bandwidth;
            self.update_coefficients();
        }
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
            BiQuadFilter::<M>::rbj_intermediates_shelving(self.sample_rate, self.cutoff, a, 1.0);

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
            BiQuadFilter::<M>::rbj_intermediates_shelving(self.sample_rate, self.cutoff, a, 1.0);

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

#[cfg(test)]
mod tests {
    // TODO: get FFT working, and then write tests.
}
