// Copyright (c) 2023 Mike Tsao. All rights reserved.

use eframe::egui::{Slider, Ui};
use ensnare::{prelude::*, traits::prelude::*};
use ensnare_proc_macros::{Control, IsEffect, Params, Uid};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Control, IsEffect, Params, Uid, Serialize, Deserialize)]
pub struct BiQuadFilterLowPass24db {
    #[control]
    #[params]
    cutoff: FrequencyHz,
    #[control]
    #[params]
    passband_ripple: ParameterType,

    uid: Uid,
    #[serde(skip)]
    sample_rate: SampleRate,
    #[serde(skip)]
    channels: [BiQuadFilterLowPass24dbChannel; 2],
}
impl Default for BiQuadFilterLowPass24db {
    fn default() -> Self {
        Self {
            cutoff: FrequencyHz::from(1000.0),
            passband_ripple: 1.0,
            uid: Default::default(),
            sample_rate: Default::default(),
            channels: Default::default(),
        }
    }
}
impl Serializable for BiQuadFilterLowPass24db {}
impl Configurable for BiQuadFilterLowPass24db {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.update_coefficients();
    }

    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }
}
impl TransformsAudio for BiQuadFilterLowPass24db {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        match channel {
            0 | 1 => self.channels[channel].transform_channel(channel, input_sample),
            _ => panic!(),
        }
    }
}
impl BiQuadFilterLowPass24db {
    pub fn new_with(params: &BiQuadFilterLowPass24dbParams) -> Self {
        let mut r = Self {
            cutoff: params.cutoff(),
            passband_ripple: params.passband_ripple(),
            uid: Default::default(),
            sample_rate: Default::default(),
            channels: [
                BiQuadFilterLowPass24dbChannel::default(),
                BiQuadFilterLowPass24dbChannel::default(),
            ],
        };
        r.update_coefficients();
        r
    }

    fn update_coefficients(&mut self) {
        self.channels[0].update_coefficients(self.sample_rate, self.cutoff, self.passband_ripple);
        self.channels[1].update_coefficients(self.sample_rate, self.cutoff, self.passband_ripple);
    }

    pub fn cutoff(&self) -> FrequencyHz {
        self.cutoff
    }
    pub fn set_cutoff(&mut self, cutoff: FrequencyHz) {
        if self.cutoff != cutoff {
            self.cutoff = cutoff;
            self.update_coefficients();
        }
    }
    pub fn passband_ripple(&self) -> ParameterType {
        self.passband_ripple
    }
    pub fn set_passband_ripple(&mut self, passband_ripple: ParameterType) {
        if self.passband_ripple != passband_ripple {
            self.passband_ripple = passband_ripple;
            self.update_coefficients();
        }
    }

    // TODO: (see Envelope's method and comments) -- this looks wasteful because
    // it could compute coefficients twice in a single transaction, but the use
    // case (egui change notifications) calls for only one thing changing at a
    // time.
    pub fn update_from_params(&mut self, params: &BiQuadFilterLowPass24dbParams) {
        self.set_cutoff(params.cutoff());
        self.set_passband_ripple(params.passband_ripple());
    }
}

#[derive(Debug, Default)]
struct BiQuadFilterLowPass24dbChannel {
    inner: BiQuadFilter,
    coefficients2: CoefficientSet2,
}
impl TransformsAudio for BiQuadFilterLowPass24dbChannel {
    fn transform_channel(&mut self, _: usize, input_sample: Sample) -> Sample {
        // Thanks
        // https://www.musicdsp.org/en/latest/Filters/229-lpf-24db-oct.html
        let input = input_sample.0;
        let stage_1 = self.inner.coefficients.b0 * input + self.inner.state_0;
        self.inner.state_0 = self.inner.coefficients.b1 * input
            + self.inner.coefficients.a1 * stage_1
            + self.inner.state_1;
        self.inner.state_1 =
            self.inner.coefficients.b2 * input + self.inner.coefficients.a2 * stage_1;
        let output = self.coefficients2.b3 * stage_1 + self.inner.state_2;
        self.inner.state_2 =
            self.coefficients2.b4 * stage_1 + self.coefficients2.a4 * output + self.inner.state_3;
        self.inner.state_3 = self.coefficients2.b5 * stage_1 + self.coefficients2.a5 * output;
        Sample::from(output)
    }
}
impl BiQuadFilterLowPass24dbChannel {
    fn update_coefficients(
        &mut self,
        sample_rate: SampleRate,
        cutoff: FrequencyHz,
        passband_ripple: ParameterType,
    ) {
        let k = (PI * cutoff.value() / sample_rate.value() as f64).tan();
        let sg = passband_ripple.sinh();
        let cg = passband_ripple.cosh() * passband_ripple.cosh();

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
        self.inner.set_coefficients(CoefficientSet {
            a0,
            a1,
            a2,
            b0,
            b1,
            b2,
        });

        let a3 = 1.0 / (c3 + k + c2);
        let a4 = 2.0 * (c2 - k) * a3;
        let a5 = (c3 - k - c2) * a3;
        let b3 = a3 * k;
        let b4 = 2.0 * b3;
        let b5 = b3;
        self.coefficients2 = CoefficientSet2 { a4, a5, b3, b4, b5 };
    }
}

#[derive(Debug, Control, IsEffect, Params, Uid, Serialize, Deserialize)]
pub struct BiQuadFilterLowPass12db {
    #[control]
    #[params]
    cutoff: FrequencyHz,
    #[control]
    #[params]
    q: ParameterType,

    uid: Uid,
    #[serde(skip)]
    sample_rate: SampleRate,
    #[serde(skip)]
    channels: [BiQuadFilterLowPass12dbChannel; 2],
}
impl Serializable for BiQuadFilterLowPass12db {}
impl Configurable for BiQuadFilterLowPass12db {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.update_coefficients();
    }
}
impl TransformsAudio for BiQuadFilterLowPass12db {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        match channel {
            0 | 1 => self.channels[channel].transform_channel(channel, input_sample),
            _ => panic!(),
        }
    }
}
impl BiQuadFilterLowPass12db {
    pub fn new_with(params: &BiQuadFilterLowPass12dbParams) -> Self {
        Self {
            cutoff: params.cutoff(),
            q: params.q(),
            uid: Default::default(),
            sample_rate: Default::default(),
            channels: [
                BiQuadFilterLowPass12dbChannel::default(),
                BiQuadFilterLowPass12dbChannel::default(),
            ],
        }
    }

    fn update_coefficients(&mut self) {
        self.channels[0].update_coefficients(self.sample_rate, self.cutoff, self.q);
        self.channels[1].update_coefficients(self.sample_rate, self.cutoff, self.q);
    }

    pub fn cutoff(&self) -> FrequencyHz {
        self.cutoff
    }
    pub fn set_cutoff(&mut self, cutoff: FrequencyHz) {
        if self.cutoff != cutoff {
            self.cutoff = cutoff;
            self.update_coefficients();
        }
    }
    pub fn q(&self) -> ParameterType {
        self.q
    }
    pub fn set_q(&mut self, q: ParameterType) {
        if self.q != q {
            self.q = q;
            self.update_coefficients();
        }
    }
}

#[derive(Debug, Default)]
struct BiQuadFilterLowPass12dbChannel {
    inner: BiQuadFilter,
}
impl TransformsAudio for BiQuadFilterLowPass12dbChannel {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        self.inner.transform_channel(channel, input_sample)
    }
}
impl BiQuadFilterLowPass12dbChannel {
    fn update_coefficients(
        &mut self,
        sample_rate: SampleRate,
        cutoff: FrequencyHz,
        q: ParameterType,
    ) {
        let (_w0, w0cos, _w0sin, alpha) =
            BiQuadFilter::rbj_intermediates_q(sample_rate, cutoff.value(), q);

        self.inner.coefficients = CoefficientSet {
            a0: 1.0 + alpha,
            a1: -2.0f64 * w0cos,
            a2: 1.0 - alpha,
            b0: (1.0 - w0cos) / 2.0f64,
            b1: (1.0 - w0cos),
            b2: (1.0 - w0cos) / 2.0f64,
        }
    }
}

#[derive(Debug, Control, IsEffect, Params, Uid, Serialize, Deserialize)]
pub struct BiQuadFilterHighPass {
    #[control]
    #[params]
    cutoff: FrequencyHz,
    #[control]
    #[params]
    q: ParameterType,

    uid: Uid,
    #[serde(skip)]
    sample_rate: SampleRate,
    #[serde(skip)]
    channels: [BiQuadFilterHighPassChannel; 2],
}
impl Serializable for BiQuadFilterHighPass {}
impl Configurable for BiQuadFilterHighPass {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.update_coefficients();
    }
}
impl TransformsAudio for BiQuadFilterHighPass {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        match channel {
            0 | 1 => self.channels[channel].transform_channel(channel, input_sample),
            _ => panic!(),
        }
    }
}
impl BiQuadFilterHighPass {
    pub fn new_with(params: &BiQuadFilterHighPassParams) -> Self {
        let mut r = Self {
            cutoff: params.cutoff(),
            q: params.q(),
            uid: Default::default(),
            sample_rate: Default::default(),
            channels: [
                BiQuadFilterHighPassChannel::default(),
                BiQuadFilterHighPassChannel::default(),
            ],
        };
        r.update_coefficients();
        r
    }

    fn update_coefficients(&mut self) {
        self.channels[0].update_coefficients(self.sample_rate, self.cutoff, self.q);
        self.channels[1].update_coefficients(self.sample_rate, self.cutoff, self.q);
    }

    pub fn cutoff(&self) -> FrequencyHz {
        self.cutoff
    }
    pub fn set_cutoff(&mut self, cutoff: FrequencyHz) {
        if self.cutoff != cutoff {
            self.cutoff = cutoff;
            self.update_coefficients();
        }
    }
    pub fn q(&self) -> ParameterType {
        self.q
    }
    pub fn set_q(&mut self, q: ParameterType) {
        if self.q != q {
            self.q = q;
            self.update_coefficients();
        }
    }
}

#[derive(Debug, Default)]
struct BiQuadFilterHighPassChannel {
    inner: BiQuadFilter,
}
impl TransformsAudio for BiQuadFilterHighPassChannel {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        self.inner.transform_channel(channel, input_sample)
    }
}
impl BiQuadFilterHighPassChannel {
    fn update_coefficients(
        &mut self,
        sample_rate: SampleRate,
        cutoff: FrequencyHz,
        q: ParameterType,
    ) {
        let (_w0, w0cos, _w0sin, alpha) =
            BiQuadFilter::rbj_intermediates_q(sample_rate, cutoff.value(), q);

        self.inner.coefficients = CoefficientSet {
            a0: 1.0 + alpha,
            a1: -2.0f64 * w0cos,
            a2: 1.0 - alpha,
            b0: (1.0 + w0cos) / 2.0f64,
            b1: -(1.0 + w0cos),
            b2: (1.0 + w0cos) / 2.0f64,
        }
    }
}

#[derive(Debug, Control, IsEffect, Params, Uid, Serialize, Deserialize)]
pub struct BiQuadFilterAllPass {
    #[control]
    #[params]
    cutoff: FrequencyHz,
    #[control]
    #[params]
    q: ParameterType,

    uid: Uid,
    #[serde(skip)]
    sample_rate: SampleRate,
    #[serde(skip)]
    channels: [BiQuadFilterAllPassChannel; 2],
}
impl Serializable for BiQuadFilterAllPass {}
impl Configurable for BiQuadFilterAllPass {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.update_coefficients();
    }
}
impl TransformsAudio for BiQuadFilterAllPass {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        match channel {
            0 | 1 => self.channels[channel].transform_channel(channel, input_sample),
            _ => panic!(),
        }
    }
}
impl BiQuadFilterAllPass {
    pub fn new_with(params: &BiQuadFilterAllPassParams) -> Self {
        Self {
            cutoff: params.cutoff(),
            q: params.q(),
            uid: Default::default(),
            sample_rate: Default::default(),
            channels: [
                BiQuadFilterAllPassChannel::default(),
                BiQuadFilterAllPassChannel::default(),
            ],
        }
    }

    fn update_coefficients(&mut self) {
        self.channels[0].update_coefficients(self.sample_rate, self.cutoff, self.q);
        self.channels[1].update_coefficients(self.sample_rate, self.cutoff, self.q);
    }

    pub fn cutoff(&self) -> FrequencyHz {
        self.cutoff
    }
    pub fn set_cutoff(&mut self, cutoff: FrequencyHz) {
        if self.cutoff != cutoff {
            self.cutoff = cutoff;
            self.update_coefficients();
        }
    }
    pub fn q(&self) -> ParameterType {
        self.q
    }
    pub fn set_q(&mut self, q: ParameterType) {
        if self.q != q {
            self.q = q;
            self.update_coefficients();
        }
    }
}

#[derive(Debug, Default)]
struct BiQuadFilterAllPassChannel {
    inner: BiQuadFilter,
}
impl TransformsAudio for BiQuadFilterAllPassChannel {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        self.inner.transform_channel(channel, input_sample)
    }
}
impl BiQuadFilterAllPassChannel {
    fn update_coefficients(
        &mut self,
        sample_rate: SampleRate,
        cutoff: FrequencyHz,
        q: ParameterType,
    ) {
        let (_w0, w0cos, _w0sin, alpha) =
            BiQuadFilter::rbj_intermediates_q(sample_rate, cutoff.value(), q);
        self.inner.coefficients = CoefficientSet {
            a0: 1.0 + alpha,
            a1: -2.0f64 * w0cos,
            a2: 1.0 - alpha,
            b0: 1.0 - alpha,
            b1: -2.0f64 * w0cos,
            b2: 1.0 + alpha,
        }
    }
}

#[derive(Debug, Control, IsEffect, Params, Uid, Serialize, Deserialize)]
pub struct BiQuadFilterBandPass {
    #[control]
    #[params]
    cutoff: FrequencyHz,
    #[control]
    #[params]
    bandwidth: ParameterType, // TODO: maybe this should be FrequencyHz

    uid: Uid,
    #[serde(skip)]
    sample_rate: SampleRate,
    #[serde(skip)]
    channels: [BiQuadFilterBandPassChannel; 2],
}
impl Serializable for BiQuadFilterBandPass {}
impl Configurable for BiQuadFilterBandPass {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.update_coefficients();
    }
}
impl TransformsAudio for BiQuadFilterBandPass {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        match channel {
            0 | 1 => self.channels[channel].transform_channel(channel, input_sample),
            _ => panic!(),
        }
    }
}
impl BiQuadFilterBandPass {
    pub fn new_with(params: &BiQuadFilterBandPassParams) -> Self {
        Self {
            cutoff: params.cutoff(),
            bandwidth: params.bandwidth(),
            uid: Default::default(),
            sample_rate: Default::default(),
            channels: [
                BiQuadFilterBandPassChannel::default(),
                BiQuadFilterBandPassChannel::default(),
            ],
        }
    }

    fn update_coefficients(&mut self) {
        self.channels[0].update_coefficients(self.sample_rate, self.cutoff, self.bandwidth);
        self.channels[1].update_coefficients(self.sample_rate, self.cutoff, self.bandwidth);
    }

    pub fn cutoff(&self) -> FrequencyHz {
        self.cutoff
    }
    pub fn set_cutoff(&mut self, cutoff: FrequencyHz) {
        if self.cutoff != cutoff {
            self.cutoff = cutoff;
            self.update_coefficients();
        }
    }
    pub fn bandwidth(&self) -> ParameterType {
        self.bandwidth
    }
    pub fn set_bandwidth(&mut self, bandwidth: ParameterType) {
        if self.bandwidth != bandwidth {
            self.bandwidth = bandwidth;
            self.update_coefficients();
        }
    }
}

#[derive(Debug, Default)]
struct BiQuadFilterBandPassChannel {
    inner: BiQuadFilter,
}
impl TransformsAudio for BiQuadFilterBandPassChannel {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        self.inner.transform_channel(channel, input_sample)
    }
}
impl BiQuadFilterBandPassChannel {
    fn update_coefficients(
        &mut self,
        sample_rate: SampleRate,
        cutoff: FrequencyHz,
        bandwidth: ParameterType,
    ) {
        let (_w0, w0cos, _w0sin, alpha) =
            BiQuadFilter::rbj_intermediates_bandwidth(sample_rate, cutoff.value(), bandwidth);
        self.inner.coefficients = CoefficientSet {
            a0: 1.0 + alpha,
            a1: -2.0f64 * w0cos,
            a2: 1.0 - alpha,
            b0: alpha,
            b1: 0.0,
            b2: -alpha,
        };
    }
}

#[derive(Debug, Control, IsEffect, Params, Uid, Serialize, Deserialize)]
pub struct BiQuadFilterBandStop {
    #[control]
    #[params]
    cutoff: FrequencyHz,
    #[control]
    #[params]
    bandwidth: ParameterType, // TODO: maybe this should be FrequencyHz

    uid: Uid,
    #[serde(skip)]
    sample_rate: SampleRate,

    #[serde(skip)]
    channels: [BiQuadFilterBandStopChannel; 2],
}
impl Serializable for BiQuadFilterBandStop {}
impl Configurable for BiQuadFilterBandStop {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.update_coefficients();
    }
}
impl TransformsAudio for BiQuadFilterBandStop {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        match channel {
            0 | 1 => self.channels[channel].transform_channel(channel, input_sample),
            _ => panic!(),
        }
    }
}
impl BiQuadFilterBandStop {
    pub fn new_with(params: &BiQuadFilterBandStopParams) -> Self {
        Self {
            cutoff: params.cutoff(),
            bandwidth: params.bandwidth(),
            uid: Default::default(),
            sample_rate: Default::default(),
            channels: [
                BiQuadFilterBandStopChannel::default(),
                BiQuadFilterBandStopChannel::default(),
            ],
        }
    }

    fn update_coefficients(&mut self) {
        self.channels[0].update_coefficients(self.sample_rate, self.cutoff, self.bandwidth);
        self.channels[1].update_coefficients(self.sample_rate, self.cutoff, self.bandwidth);
    }

    pub fn cutoff(&self) -> FrequencyHz {
        self.cutoff
    }
    pub fn set_cutoff(&mut self, cutoff: FrequencyHz) {
        if self.cutoff != cutoff {
            self.cutoff = cutoff;
            self.update_coefficients();
        }
    }
    pub fn bandwidth(&self) -> ParameterType {
        self.bandwidth
    }
    pub fn set_bandwidth(&mut self, bandwidth: ParameterType) {
        if self.bandwidth != bandwidth {
            self.bandwidth = bandwidth;
            self.update_coefficients();
        }
    }
}

#[derive(Debug, Default)]
struct BiQuadFilterBandStopChannel {
    inner: BiQuadFilter,
}
impl TransformsAudio for BiQuadFilterBandStopChannel {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        self.inner.transform_channel(channel, input_sample)
    }
}
impl BiQuadFilterBandStopChannel {
    fn update_coefficients(
        &mut self,
        sample_rate: SampleRate,
        cutoff: FrequencyHz,
        bandwidth: ParameterType,
    ) {
        let (_w0, w0cos, _w0sin, alpha) =
            BiQuadFilter::rbj_intermediates_bandwidth(sample_rate, cutoff.value(), bandwidth);

        self.inner.coefficients = CoefficientSet {
            a0: 1.0 + alpha,
            a1: -2.0f64 * w0cos,
            a2: 1.0 - alpha,
            b0: 1.0,
            b1: -2.0f64 * w0cos,
            b2: 1.0,
        }
    }
}

#[derive(Debug, Control, IsEffect, Params, Uid, Serialize, Deserialize)]
pub struct BiQuadFilterPeakingEq {
    #[control]
    #[params]
    cutoff: FrequencyHz,

    // I didn't know what to call this. RBJ says "...except for peakingEQ in
    // which A*Q is the classic EE Q." I think Q is close enough to get the gist.
    #[control]
    #[params]
    q: ParameterType,

    uid: Uid,
    #[serde(skip)]
    sample_rate: SampleRate,
    #[serde(skip)]
    channels: [BiQuadFilterPeakingEqChannel; 2],
}
impl Serializable for BiQuadFilterPeakingEq {}
impl Configurable for BiQuadFilterPeakingEq {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.update_coefficients();
    }
}
impl TransformsAudio for BiQuadFilterPeakingEq {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        match channel {
            0 | 1 => self.channels[channel].transform_channel(channel, input_sample),
            _ => panic!(),
        }
    }
}
impl BiQuadFilterPeakingEq {
    pub fn new_with(params: &BiQuadFilterPeakingEqParams) -> Self {
        let mut r = Self {
            cutoff: params.cutoff(),
            q: params.q(),
            uid: Default::default(),
            sample_rate: Default::default(),
            channels: [
                BiQuadFilterPeakingEqChannel::default(),
                BiQuadFilterPeakingEqChannel::default(),
            ],
        };
        r.update_coefficients();
        r
    }

    fn update_coefficients(&mut self) {
        self.channels[0].update_coefficients(self.sample_rate, self.cutoff, self.q);
        self.channels[1].update_coefficients(self.sample_rate, self.cutoff, self.q);
    }

    pub fn cutoff(&self) -> FrequencyHz {
        self.cutoff
    }
    pub fn set_cutoff(&mut self, cutoff: FrequencyHz) {
        if self.cutoff != cutoff {
            self.cutoff = cutoff;
            self.update_coefficients();
        }
    }
    pub fn q(&self) -> ParameterType {
        self.q
    }
    pub fn set_q(&mut self, q: ParameterType) {
        if self.q != q {
            self.q = q;
            self.update_coefficients();
        }
    }
}

#[derive(Debug, Default)]
struct BiQuadFilterPeakingEqChannel {
    inner: BiQuadFilter,
}
impl TransformsAudio for BiQuadFilterPeakingEqChannel {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        self.inner.transform_channel(channel, input_sample)
    }
}
impl BiQuadFilterPeakingEqChannel {
    fn update_coefficients(
        &mut self,
        sample_rate: SampleRate,
        cutoff: FrequencyHz,
        q: ParameterType,
    ) {
        let (_w0, w0cos, _w0sin, alpha) = BiQuadFilter::rbj_intermediates_q(
            sample_rate,
            cutoff.value(),
            std::f64::consts::FRAC_1_SQRT_2,
        );
        let a = 10f64.powf(q / 10.0f64).sqrt();

        self.inner.coefficients = CoefficientSet {
            a0: 1.0 + alpha / a,
            a1: -2.0f64 * w0cos,
            a2: 1.0 - alpha / a,
            b0: 1.0 + alpha * a,
            b1: -2.0f64 * w0cos,
            b2: 1.0 - alpha * a,
        }
    }
}

#[derive(Debug, Control, IsEffect, Params, Uid, Serialize, Deserialize)]
pub struct BiQuadFilterLowShelf {
    #[control]
    #[params]
    cutoff: FrequencyHz,
    #[control]
    #[params]
    db_gain: ParameterType,

    uid: Uid,
    #[serde(skip)]
    sample_rate: SampleRate,
    #[serde(skip)]
    channels: [BiQuadFilterLowShelfChannel; 2],
}
impl Serializable for BiQuadFilterLowShelf {}
impl Configurable for BiQuadFilterLowShelf {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.update_coefficients();
    }
}
impl TransformsAudio for BiQuadFilterLowShelf {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        match channel {
            0 | 1 => self.channels[channel].transform_channel(channel, input_sample),
            _ => panic!(),
        }
    }
}
impl BiQuadFilterLowShelf {
    pub fn new_with(params: &BiQuadFilterLowShelfParams) -> Self {
        Self {
            cutoff: params.cutoff(),
            db_gain: params.db_gain(),
            uid: Default::default(),
            sample_rate: Default::default(),
            channels: [
                BiQuadFilterLowShelfChannel::default(),
                BiQuadFilterLowShelfChannel::default(),
            ],
        }
    }

    fn update_coefficients(&mut self) {
        self.channels[0].update_coefficients(self.sample_rate, self.cutoff, self.db_gain);
        self.channels[1].update_coefficients(self.sample_rate, self.cutoff, self.db_gain);
    }

    pub fn cutoff(&self) -> FrequencyHz {
        self.cutoff
    }
    pub fn set_cutoff(&mut self, cutoff: FrequencyHz) {
        if self.cutoff != cutoff {
            self.cutoff = cutoff;
            self.update_coefficients();
        }
    }
    pub fn db_gain(&self) -> ParameterType {
        self.db_gain
    }
    pub fn set_db_gain(&mut self, db_gain: ParameterType) {
        if self.db_gain != db_gain {
            self.db_gain = db_gain;
            self.update_coefficients();
        }
    }
}

#[derive(Debug, Default)]
struct BiQuadFilterLowShelfChannel {
    inner: BiQuadFilter,
}
impl TransformsAudio for BiQuadFilterLowShelfChannel {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        self.inner.transform_channel(channel, input_sample)
    }
}
impl BiQuadFilterLowShelfChannel {
    fn update_coefficients(
        &mut self,
        sample_rate: SampleRate,
        cutoff: FrequencyHz,
        db_gain: ParameterType,
    ) {
        let a = 10f64.powf(db_gain / 10.0f64).sqrt();
        let (_w0, w0cos, _w0sin, alpha) =
            BiQuadFilter::rbj_intermediates_shelving(sample_rate, cutoff.value(), a, 1.0);

        self.inner.coefficients = CoefficientSet {
            a0: (a + 1.0) + (a - 1.0) * w0cos + 2.0 * a.sqrt() * alpha,
            a1: -2.0 * ((a - 1.0) + (a + 1.0) * w0cos),
            a2: (a + 1.0) + (a - 1.0) * w0cos - 2.0 * a.sqrt() * alpha,
            b0: a * ((a + 1.0) - (a - 1.0) * w0cos + 2.0 * a.sqrt() * alpha),
            b1: 2.0 * a * ((a - 1.0) - (a + 1.0) * w0cos),
            b2: a * ((a + 1.0) - (a - 1.0) * w0cos - 2.0 * a.sqrt() * alpha),
        };
    }
}

#[derive(Debug, Control, IsEffect, Params, Uid, Serialize, Deserialize)]
pub struct BiQuadFilterHighShelf {
    #[control]
    #[params]
    cutoff: FrequencyHz,
    #[control]
    #[params]
    db_gain: ParameterType,

    uid: Uid,
    #[serde(skip)]
    sample_rate: SampleRate,
    #[serde(skip)]
    channels: [BiQuadFilterHighShelfChannel; 2],
}
impl Serializable for BiQuadFilterHighShelf {}
impl Configurable for BiQuadFilterHighShelf {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.update_coefficients();
    }
}
impl TransformsAudio for BiQuadFilterHighShelf {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        match channel {
            0 | 1 => self.channels[channel].transform_channel(channel, input_sample),
            _ => panic!(),
        }
    }
}
impl BiQuadFilterHighShelf {
    pub fn new_with(params: &BiQuadFilterHighShelfParams) -> Self {
        Self {
            cutoff: params.cutoff(),
            db_gain: params.db_gain(),
            uid: Default::default(),
            sample_rate: Default::default(),
            channels: [
                BiQuadFilterHighShelfChannel::default(),
                BiQuadFilterHighShelfChannel::default(),
            ],
        }
    }

    fn update_coefficients(&mut self) {
        self.channels[0].update_coefficients(self.sample_rate, self.cutoff, self.db_gain);
        self.channels[1].update_coefficients(self.sample_rate, self.cutoff, self.db_gain);
    }

    pub fn cutoff(&self) -> FrequencyHz {
        self.cutoff
    }
    pub fn set_cutoff(&mut self, cutoff: FrequencyHz) {
        if self.cutoff != cutoff {
            self.cutoff = cutoff;
            self.update_coefficients();
        }
    }
    pub fn db_gain(&self) -> ParameterType {
        self.db_gain
    }
    pub fn set_db_gain(&mut self, db_gain: ParameterType) {
        if self.db_gain != db_gain {
            self.db_gain = db_gain;
            self.update_coefficients();
        }
    }
}

#[derive(Debug, Default)]
struct BiQuadFilterHighShelfChannel {
    inner: BiQuadFilter,
}
impl TransformsAudio for BiQuadFilterHighShelfChannel {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        self.inner.transform_channel(channel, input_sample)
    }
}
impl BiQuadFilterHighShelfChannel {
    fn update_coefficients(
        &mut self,
        sample_rate: SampleRate,
        cutoff: FrequencyHz,
        db_gain: ParameterType,
    ) {
        let a = 10f64.powf(db_gain / 10.0f64).sqrt();
        let (_w0, w0cos, _w0sin, alpha) =
            BiQuadFilter::rbj_intermediates_shelving(sample_rate, cutoff.value(), a, 1.0);

        self.inner.coefficients = CoefficientSet {
            a0: (a + 1.0) - (a - 1.0) * w0cos + 2.0 * a.sqrt() * alpha,
            a1: 2.0 * ((a - 1.0) - (a + 1.0) * w0cos),
            a2: (a + 1.0) - (a - 1.0) * w0cos - 2.0 * a.sqrt() * alpha,
            b0: a * ((a + 1.0) + (a - 1.0) * w0cos + 2.0 * a.sqrt() * alpha),
            b1: -2.0 * a * ((a - 1.0) + (a + 1.0) * w0cos),
            b2: a * ((a + 1.0) + (a - 1.0) * w0cos - 2.0 * a.sqrt() * alpha),
        }
    }
}

/// This filter does nothing, expensively. It exists for debugging. I might
/// delete it later.
#[derive(Debug, Control, IsEffect, Params, Uid, Serialize, Deserialize)]
pub struct BiQuadFilterNone {
    uid: Uid,
    #[serde(skip)]
    sample_rate: SampleRate,
    #[serde(skip)]
    channels: [BiQuadFilter; 2],
}
impl Serializable for BiQuadFilterNone {}
impl Configurable for BiQuadFilterNone {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
    }
}
impl TransformsAudio for BiQuadFilterNone {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        match channel {
            0 | 1 => self.channels[channel].transform_channel(channel, input_sample),
            _ => panic!(),
        }
    }
}
impl BiQuadFilterNone {
    pub fn new_with(_: BiQuadFilterNoneParams) -> Self {
        Self {
            uid: Default::default(),
            sample_rate: Default::default(),
            channels: [BiQuadFilter::default(), BiQuadFilter::default()],
        }
    }
}

#[derive(Clone, Debug)]
struct CoefficientSet {
    a0: f64,
    a1: f64,
    a2: f64,
    b0: f64,
    b1: f64,
    b2: f64,
}
impl Default for CoefficientSet {
    // This is an identity set.
    fn default() -> Self {
        Self {
            a0: 1.0,
            a1: 0.0,
            a2: 0.0,
            b0: 0.0,
            b1: 0.0,
            b2: 0.0,
        }
    }
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
#[derive(Clone, Debug, Default)]
pub struct BiQuadFilter {
    coefficients: CoefficientSet,

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
impl TransformsAudio for BiQuadFilter {
    // Everyone but LowPassFilter24db uses this implementation
    fn transform_channel(&mut self, _channel: usize, input_sample: Sample) -> Sample {
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
        sample_rate: SampleRate,
        cutoff: ParameterType,
        q: ParameterType,
    ) -> (f64, f64, f64, f64) {
        let w0 = 2.0f64 * PI * cutoff / sample_rate.value() as f64;
        let w0cos = w0.cos();
        let w0sin = w0.sin();
        let alpha = w0sin / (2.0f64 * q);
        (w0, w0cos, w0sin, alpha)
    }

    fn rbj_intermediates_bandwidth(
        sample_rate: SampleRate,
        cutoff: ParameterType,
        bandwidth: ParameterType,
    ) -> (f64, f64, f64, f64) {
        let w0 = 2.0f64 * PI * cutoff / sample_rate.value() as f64;
        let w0cos = w0.cos();
        let w0sin = w0.sin();
        let alpha = w0sin * (2.0f64.ln() / 2.0 * bandwidth as f64 * w0 / w0.sin()).sinh();
        (w0, w0cos, w0sin, alpha)
    }

    fn rbj_intermediates_shelving(
        sample_rate: SampleRate,
        cutoff: ParameterType,
        db_gain: ParameterType,
        s: f64,
    ) -> (f64, f64, f64, f64) {
        let w0 = 2.0f64 * PI * cutoff as f64 / sample_rate.value() as f64;
        let w0cos = w0.cos();
        let w0sin = w0.sin();
        let alpha = w0sin / 2.0 * ((db_gain + 1.0 / db_gain) * (1.0 / s - 1.0) + 2.0).sqrt();
        (w0, w0cos, w0sin, alpha)
    }

    fn set_coefficients(&mut self, coefficient_set: CoefficientSet) {
        self.coefficients = coefficient_set;
    }
}

impl Displays for BiQuadFilterAllPass {
    fn ui(&mut self, ui: &mut Ui) -> eframe::egui::Response {
        ui.label(self.name())
    }
}

impl Displays for BiQuadFilterLowPass12db {
    fn ui(&mut self, ui: &mut Ui) -> eframe::egui::Response {
        ui.label(self.name())
    }
}

impl Displays for BiQuadFilterHighPass {
    fn ui(&mut self, ui: &mut Ui) -> eframe::egui::Response {
        ui.label(self.name())
    }
}

impl Displays for BiQuadFilterHighShelf {
    fn ui(&mut self, ui: &mut Ui) -> eframe::egui::Response {
        ui.label(self.name())
    }
}

impl Displays for BiQuadFilterPeakingEq {
    fn ui(&mut self, ui: &mut Ui) -> eframe::egui::Response {
        ui.label(self.name())
    }
}

impl Displays for BiQuadFilterBandPass {
    fn ui(&mut self, ui: &mut Ui) -> eframe::egui::Response {
        ui.label(self.name())
    }
}

impl Displays for BiQuadFilterBandStop {
    fn ui(&mut self, ui: &mut Ui) -> eframe::egui::Response {
        ui.label(self.name())
    }
}

impl Displays for BiQuadFilterLowShelf {
    fn ui(&mut self, ui: &mut Ui) -> eframe::egui::Response {
        ui.label(self.name())
    }
}

impl Displays for BiQuadFilterNone {
    fn ui(&mut self, ui: &mut Ui) -> eframe::egui::Response {
        ui.label(self.name())
    }
}

impl Displays for BiQuadFilterLowPass24db {
    fn ui(&mut self, ui: &mut Ui) -> eframe::egui::Response {
        let mut cutoff = self.cutoff().value();
        let mut pbr = self.passband_ripple();
        let cutoff_response = ui.add(Slider::new(&mut cutoff, FrequencyHz::range()).text("Cutoff"));
        if cutoff_response.changed() {
            self.set_cutoff(cutoff.into());
        };
        let passband_response = ui.add(Slider::new(&mut pbr, 0.0..=10.0).text("Passband"));
        if passband_response.changed() {
            self.set_passband_ripple(pbr);
        };
        cutoff_response | passband_response
    }
}

#[cfg(test)]
mod tests {
    // TODO: get FFT working, and then write tests.
}
