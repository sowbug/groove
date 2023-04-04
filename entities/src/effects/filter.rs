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

#[derive(Debug, Nano, Uid)]
pub struct BiQuadFilterLowPass24db {
    #[nano]
    cutoff: FrequencyHz,
    #[nano]
    passband_ripple: ParameterType,

    uid: usize,
    sample_rate: usize,
    channels: [BiQuadFilterLowPass24dbChannel; 2],
}
impl IsEffect for BiQuadFilterLowPass24db {}
impl TransformsAudio for BiQuadFilterLowPass24db {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        match channel {
            0 | 1 => self.channels[channel].transform_channel(channel, input_sample),
            _ => panic!(),
        }
    }
}
impl BiQuadFilterLowPass24db {
    pub fn new_with(sample_rate: usize, params: BiQuadFilterLowPass24dbNano) -> Self {
        let mut r = Self {
            cutoff: params.cutoff(),
            passband_ripple: params.passband_ripple(),
            uid: Default::default(),
            sample_rate,
            channels: [
                BiQuadFilterLowPass24dbChannel::new_with(
                    sample_rate,
                    params.cutoff(),
                    params.passband_ripple(),
                ),
                BiQuadFilterLowPass24dbChannel::new_with(
                    sample_rate,
                    params.cutoff(),
                    params.passband_ripple(),
                ),
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

    pub fn update(&mut self, message: BiQuadFilterLowPass24dbMessage) {
        match message {
            BiQuadFilterLowPass24dbMessage::BiQuadFilterLowPass24db(e) => {
                *self = Self::new_with(self.sample_rate, e)
            }
            _ => self.derived_update(message),
        }
    }
}

#[derive(Debug)]
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
    pub fn new_with(
        sample_rate: usize,
        cutoff: FrequencyHz,
        passband_ripple: ParameterType,
    ) -> Self {
        let mut r = Self {
            inner: BiQuadFilter::new_with(sample_rate, cutoff, passband_ripple),
            coefficients2: Default::default(),
        };
        r.update_coefficients(sample_rate, cutoff, passband_ripple);
        r
    }

    fn update_coefficients(
        &mut self,
        sample_rate: usize,
        cutoff: FrequencyHz,
        passband_ripple: ParameterType,
    ) {
        let k = (PI * cutoff.value() / sample_rate as f64).tan();
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

#[derive(Debug, Nano, Uid)]
pub struct BiQuadFilterLowPass12db {
    #[nano]
    cutoff: FrequencyHz,
    #[nano]
    q: ParameterType,

    uid: usize,
    sample_rate: usize,
    channels: [BiQuadFilterLowPass12dbChannel; 2],
}
impl IsEffect for BiQuadFilterLowPass12db {}
impl TransformsAudio for BiQuadFilterLowPass12db {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        match channel {
            0 | 1 => self.channels[channel].transform_channel(channel, input_sample),
            _ => panic!(),
        }
    }
}
impl BiQuadFilterLowPass12db {
    pub fn new_with(sample_rate: usize, params: BiQuadFilterLowPass12dbNano) -> Self {
        let mut r = Self {
            cutoff: params.cutoff(),
            q: params.q(),
            uid: Default::default(),
            sample_rate,
            channels: [
                BiQuadFilterLowPass12dbChannel::new_with(sample_rate, params.cutoff(), params.q()),
                BiQuadFilterLowPass12dbChannel::new_with(sample_rate, params.cutoff(), params.q()),
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
    pub fn update(&mut self, message: BiQuadFilterLowPass12dbMessage) {
        match message {
            BiQuadFilterLowPass12dbMessage::BiQuadFilterLowPass12db(e) => {
                *self = Self::new_with(self.sample_rate, e)
            }
            _ => self.derived_update(message),
        }
    }
}

#[derive(Debug)]
struct BiQuadFilterLowPass12dbChannel {
    inner: BiQuadFilter,
}
impl TransformsAudio for BiQuadFilterLowPass12dbChannel {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        self.inner.transform_channel(channel, input_sample)
    }
}
impl BiQuadFilterLowPass12dbChannel {
    pub fn new_with(sample_rate: usize, cutoff: FrequencyHz, q: ParameterType) -> Self {
        let mut r = Self {
            inner: BiQuadFilter::new_with(sample_rate, cutoff, q),
        };
        r.update_coefficients(sample_rate, cutoff, q);
        r
    }

    fn update_coefficients(&mut self, sample_rate: usize, cutoff: FrequencyHz, q: ParameterType) {
        let (w0, w0cos, w0sin, alpha) =
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

#[derive(Debug, Nano, Uid)]
pub struct BiQuadFilterHighPass {
    #[nano]
    cutoff: FrequencyHz,
    #[nano]
    q: ParameterType,

    uid: usize,
    sample_rate: usize,
    channels: [BiQuadFilterHighPassChannel; 2],
}
impl IsEffect for BiQuadFilterHighPass {}
impl TransformsAudio for BiQuadFilterHighPass {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        match channel {
            0 | 1 => self.channels[channel].transform_channel(channel, input_sample),
            _ => panic!(),
        }
    }
}
impl BiQuadFilterHighPass {
    pub fn new_with(sample_rate: usize, params: BiQuadFilterHighPassNano) -> Self {
        let mut r = Self {
            cutoff: params.cutoff(),
            q: params.q(),
            uid: Default::default(),
            sample_rate,
            channels: [
                BiQuadFilterHighPassChannel::new_with(sample_rate, params.cutoff(), params.q()),
                BiQuadFilterHighPassChannel::new_with(sample_rate, params.cutoff(), params.q()),
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
    pub fn update(&mut self, message: BiQuadFilterHighPassMessage) {
        match message {
            BiQuadFilterHighPassMessage::BiQuadFilterHighPass(e) => {
                *self = Self::new_with(self.sample_rate, e)
            }
            _ => self.derived_update(message),
        }
    }
}

#[derive(Debug)]
struct BiQuadFilterHighPassChannel {
    inner: BiQuadFilter,
}
impl TransformsAudio for BiQuadFilterHighPassChannel {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        self.inner.transform_channel(channel, input_sample)
    }
}
impl BiQuadFilterHighPassChannel {
    pub fn new_with(sample_rate: usize, cutoff: FrequencyHz, q: ParameterType) -> Self {
        let mut r = Self {
            inner: BiQuadFilter::new_with(sample_rate, cutoff, q),
        };
        r.update_coefficients(sample_rate, cutoff, q);
        r
    }

    fn update_coefficients(&mut self, sample_rate: usize, cutoff: FrequencyHz, q: ParameterType) {
        let (w0, w0cos, w0sin, alpha) =
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

#[derive(Debug, Nano, Uid)]
pub struct BiQuadFilterAllPass {
    #[nano]
    cutoff: FrequencyHz,
    #[nano]
    q: ParameterType,

    uid: usize,
    sample_rate: usize,
    channels: [BiQuadFilterAllPassChannel; 2],
}
impl IsEffect for BiQuadFilterAllPass {}
impl TransformsAudio for BiQuadFilterAllPass {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        match channel {
            0 | 1 => self.channels[channel].transform_channel(channel, input_sample),
            _ => panic!(),
        }
    }
}
impl BiQuadFilterAllPass {
    pub fn new_with(sample_rate: usize, params: BiQuadFilterAllPassNano) -> Self {
        let mut r = Self {
            cutoff: params.cutoff(),
            q: params.q(),
            uid: Default::default(),
            sample_rate,
            channels: [
                BiQuadFilterAllPassChannel::new_with(sample_rate, params.cutoff(), params.q()),
                BiQuadFilterAllPassChannel::new_with(sample_rate, params.cutoff(), params.q()),
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
    pub fn update(&mut self, message: BiQuadFilterAllPassMessage) {
        match message {
            BiQuadFilterAllPassMessage::BiQuadFilterAllPass(e) => {
                *self = Self::new_with(self.sample_rate, e)
            }
            _ => self.derived_update(message),
        }
    }
}

#[derive(Debug)]
struct BiQuadFilterAllPassChannel {
    inner: BiQuadFilter,
}
impl TransformsAudio for BiQuadFilterAllPassChannel {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        self.inner.transform_channel(channel, input_sample)
    }
}
impl BiQuadFilterAllPassChannel {
    pub fn new_with(sample_rate: usize, cutoff: FrequencyHz, q: ParameterType) -> Self {
        let mut r = Self {
            inner: BiQuadFilter::new_with(sample_rate, cutoff, q),
        };
        r.update_coefficients(sample_rate, cutoff, q);
        r
    }

    fn update_coefficients(&mut self, sample_rate: usize, cutoff: FrequencyHz, q: ParameterType) {
        let (w0, w0cos, w0sin, alpha) =
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

#[derive(Debug, Nano, Uid)]
pub struct BiQuadFilterBandPass {
    #[nano]
    cutoff: FrequencyHz,
    #[nano]
    bandwidth: ParameterType, // TODO: maybe this should be FrequencyHz

    uid: usize,
    sample_rate: usize,
    channels: [BiQuadFilterBandPassChannel; 2],
}
impl IsEffect for BiQuadFilterBandPass {}
impl TransformsAudio for BiQuadFilterBandPass {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        match channel {
            0 | 1 => self.channels[channel].transform_channel(channel, input_sample),
            _ => panic!(),
        }
    }
}
impl BiQuadFilterBandPass {
    pub fn new_with(sample_rate: usize, params: BiQuadFilterBandPassNano) -> Self {
        let mut r = Self {
            cutoff: params.cutoff(),
            bandwidth: params.bandwidth(),
            uid: Default::default(),
            sample_rate,
            channels: [
                BiQuadFilterBandPassChannel::new_with(
                    sample_rate,
                    params.cutoff(),
                    params.bandwidth(),
                ),
                BiQuadFilterBandPassChannel::new_with(
                    sample_rate,
                    params.cutoff(),
                    params.bandwidth(),
                ),
            ],
        };
        r.update_coefficients();
        r
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
    pub fn update(&mut self, message: BiQuadFilterBandPassMessage) {
        match message {
            BiQuadFilterBandPassMessage::BiQuadFilterBandPass(e) => {
                *self = Self::new_with(self.sample_rate, e)
            }
            _ => self.derived_update(message),
        }
    }
}

#[derive(Debug)]
struct BiQuadFilterBandPassChannel {
    inner: BiQuadFilter,
}
impl TransformsAudio for BiQuadFilterBandPassChannel {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        self.inner.transform_channel(channel, input_sample)
    }
}
impl BiQuadFilterBandPassChannel {
    pub fn new_with(sample_rate: usize, cutoff: FrequencyHz, q: ParameterType) -> Self {
        let mut r = Self {
            inner: BiQuadFilter::new_with(sample_rate, cutoff, q),
        };
        r.update_coefficients(sample_rate, cutoff, q);
        r
    }

    fn update_coefficients(
        &mut self,
        sample_rate: usize,
        cutoff: FrequencyHz,
        bandwidth: ParameterType,
    ) {
        let (w0, w0cos, w0sin, alpha) =
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

#[derive(Debug, Nano, Uid)]
pub struct BiQuadFilterBandStop {
    #[nano]
    cutoff: FrequencyHz,
    #[nano]
    bandwidth: ParameterType, // TODO: maybe this should be FrequencyHz

    uid: usize,
    sample_rate: usize,
    channels: [BiQuadFilterBandStopChannel; 2],
}
impl IsEffect for BiQuadFilterBandStop {}
impl TransformsAudio for BiQuadFilterBandStop {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        match channel {
            0 | 1 => self.channels[channel].transform_channel(channel, input_sample),
            _ => panic!(),
        }
    }
}
impl BiQuadFilterBandStop {
    pub fn new_with(sample_rate: usize, params: BiQuadFilterBandStopNano) -> Self {
        let mut r = Self {
            cutoff: params.cutoff(),
            bandwidth: params.bandwidth(),
            uid: Default::default(),
            sample_rate,
            channels: [
                BiQuadFilterBandStopChannel::new_with(
                    sample_rate,
                    params.cutoff(),
                    params.bandwidth(),
                ),
                BiQuadFilterBandStopChannel::new_with(
                    sample_rate,
                    params.cutoff(),
                    params.bandwidth(),
                ),
            ],
        };
        r.update_coefficients();
        r
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
    pub fn update(&mut self, message: BiQuadFilterBandStopMessage) {
        match message {
            BiQuadFilterBandStopMessage::BiQuadFilterBandStop(e) => {
                *self = Self::new_with(self.sample_rate, e)
            }
            _ => self.derived_update(message),
        }
    }
}

#[derive(Debug)]
struct BiQuadFilterBandStopChannel {
    inner: BiQuadFilter,
}
impl TransformsAudio for BiQuadFilterBandStopChannel {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        self.inner.transform_channel(channel, input_sample)
    }
}
impl BiQuadFilterBandStopChannel {
    pub fn new_with(sample_rate: usize, cutoff: FrequencyHz, q: ParameterType) -> Self {
        let mut r = Self {
            inner: BiQuadFilter::new_with(sample_rate, cutoff, q),
        };
        r.update_coefficients(sample_rate, cutoff, q);
        r
    }

    fn update_coefficients(
        &mut self,
        sample_rate: usize,
        cutoff: FrequencyHz,
        bandwidth: ParameterType,
    ) {
        let (w0, w0cos, w0sin, alpha) =
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

#[derive(Debug, Nano, Uid)]
pub struct BiQuadFilterPeakingEq {
    #[nano]
    cutoff: FrequencyHz,

    // I didn't know what to call this. RBJ says "...except for peakingEQ in
    // which A*Q is the classic EE Q." Rather than try to shoehorn it into
    // something not-quite-accurate, I'm calling it a mysterious and hopefully
    // not already overloaded "alpha" that hopefully alludes to its relationship
    // with A.
    #[nano]
    alpha: ParameterType,

    uid: usize,
    sample_rate: usize,
    channels: [BiQuadFilterPeakingEqChannel; 2],
}
impl IsEffect for BiQuadFilterPeakingEq {}
impl TransformsAudio for BiQuadFilterPeakingEq {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        match channel {
            0 | 1 => self.channels[channel].transform_channel(channel, input_sample),
            _ => panic!(),
        }
    }
}
impl BiQuadFilterPeakingEq {
    pub fn new_with(sample_rate: usize, params: BiQuadFilterPeakingEqNano) -> Self {
        let mut r = Self {
            cutoff: params.cutoff(),
            alpha: params.alpha(),
            uid: Default::default(),
            sample_rate,
            channels: [
                BiQuadFilterPeakingEqChannel::new_with(
                    sample_rate,
                    params.cutoff(),
                    params.alpha(),
                ),
                BiQuadFilterPeakingEqChannel::new_with(
                    sample_rate,
                    params.cutoff(),
                    params.alpha(),
                ),
            ],
        };
        r.update_coefficients();
        r
    }

    fn update_coefficients(&mut self) {
        self.channels[0].update_coefficients(self.sample_rate, self.cutoff, self.alpha);
        self.channels[1].update_coefficients(self.sample_rate, self.cutoff, self.alpha);
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
    pub fn alpha(&self) -> ParameterType {
        self.alpha
    }
    pub fn set_alpha(&mut self, bandwidth: ParameterType) {
        if self.alpha != bandwidth {
            self.alpha = bandwidth;
            self.update_coefficients();
        }
    }
    pub fn update(&mut self, message: BiQuadFilterPeakingEqMessage) {
        match message {
            BiQuadFilterPeakingEqMessage::BiQuadFilterPeakingEq(e) => {
                *self = Self::new_with(self.sample_rate, e)
            }
            _ => self.derived_update(message),
        }
    }
}

#[derive(Debug)]
struct BiQuadFilterPeakingEqChannel {
    inner: BiQuadFilter,
}
impl TransformsAudio for BiQuadFilterPeakingEqChannel {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        self.inner.transform_channel(channel, input_sample)
    }
}
impl BiQuadFilterPeakingEqChannel {
    pub fn new_with(sample_rate: usize, cutoff: FrequencyHz, alpha: ParameterType) -> Self {
        let mut r = Self {
            inner: BiQuadFilter::new_with(sample_rate, cutoff, alpha),
        };
        r.update_coefficients(sample_rate, cutoff, alpha);
        r
    }

    fn update_coefficients(
        &mut self,
        sample_rate: usize,
        cutoff: FrequencyHz,
        alpha: ParameterType,
    ) {
        let (w0, w0cos, w0sin, alpha) = BiQuadFilter::rbj_intermediates_q(
            sample_rate,
            cutoff.value(),
            std::f64::consts::FRAC_1_SQRT_2,
        );
        let a = 10f64.powf(alpha / 10.0f64).sqrt();

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

#[derive(Debug, Nano, Uid)]
pub struct BiQuadFilterLowShelf {
    #[nano]
    cutoff: FrequencyHz,
    #[nano]
    db_gain: ParameterType,

    uid: usize,
    sample_rate: usize,
    channels: [BiQuadFilterLowShelfChannel; 2],
}
impl IsEffect for BiQuadFilterLowShelf {}
impl TransformsAudio for BiQuadFilterLowShelf {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        match channel {
            0 | 1 => self.channels[channel].transform_channel(channel, input_sample),
            _ => panic!(),
        }
    }
}
impl BiQuadFilterLowShelf {
    pub fn new_with(sample_rate: usize, params: BiQuadFilterLowShelfNano) -> Self {
        let mut r = Self {
            cutoff: params.cutoff(),
            db_gain: params.db_gain(),
            uid: Default::default(),
            sample_rate,
            channels: [
                BiQuadFilterLowShelfChannel::new_with(
                    sample_rate,
                    params.cutoff(),
                    params.db_gain(),
                ),
                BiQuadFilterLowShelfChannel::new_with(
                    sample_rate,
                    params.cutoff(),
                    params.db_gain(),
                ),
            ],
        };
        r.update_coefficients();
        r
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
    pub fn update(&mut self, message: BiQuadFilterLowShelfMessage) {
        match message {
            BiQuadFilterLowShelfMessage::BiQuadFilterLowShelf(e) => {
                *self = Self::new_with(self.sample_rate, e)
            }
            _ => self.derived_update(message),
        }
    }
}

#[derive(Debug)]
struct BiQuadFilterLowShelfChannel {
    inner: BiQuadFilter,
}
impl TransformsAudio for BiQuadFilterLowShelfChannel {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        self.inner.transform_channel(channel, input_sample)
    }
}
impl BiQuadFilterLowShelfChannel {
    pub fn new_with(sample_rate: usize, cutoff: FrequencyHz, q: ParameterType) -> Self {
        let mut r = Self {
            inner: BiQuadFilter::new_with(sample_rate, cutoff, q),
        };
        r.update_coefficients(sample_rate, cutoff, q);
        r
    }

    fn update_coefficients(
        &mut self,
        sample_rate: usize,
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

#[derive(Debug, Nano, Uid)]
pub struct BiQuadFilterHighShelf {
    #[nano]
    cutoff: FrequencyHz,
    #[nano]
    db_gain: ParameterType,

    uid: usize,
    sample_rate: usize,
    channels: [BiQuadFilterHighShelfChannel; 2],
}
impl IsEffect for BiQuadFilterHighShelf {}
impl TransformsAudio for BiQuadFilterHighShelf {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        match channel {
            0 | 1 => self.channels[channel].transform_channel(channel, input_sample),
            _ => panic!(),
        }
    }
}
impl BiQuadFilterHighShelf {
    pub fn new_with(sample_rate: usize, params: BiQuadFilterHighShelfNano) -> Self {
        let mut r = Self {
            cutoff: params.cutoff(),
            db_gain: params.db_gain(),
            uid: Default::default(),
            sample_rate,
            channels: [
                BiQuadFilterHighShelfChannel::new_with(
                    sample_rate,
                    params.cutoff(),
                    params.db_gain(),
                ),
                BiQuadFilterHighShelfChannel::new_with(
                    sample_rate,
                    params.cutoff(),
                    params.db_gain(),
                ),
            ],
        };
        r.update_coefficients();
        r
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
    pub fn update(&mut self, message: BiQuadFilterHighShelfMessage) {
        match message {
            BiQuadFilterHighShelfMessage::BiQuadFilterHighShelf(e) => {
                *self = Self::new_with(self.sample_rate, e)
            }
            _ => self.derived_update(message),
        }
    }
}

#[derive(Debug)]
struct BiQuadFilterHighShelfChannel {
    inner: BiQuadFilter,
}
impl TransformsAudio for BiQuadFilterHighShelfChannel {
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample {
        self.inner.transform_channel(channel, input_sample)
    }
}
impl BiQuadFilterHighShelfChannel {
    pub fn new_with(sample_rate: usize, cutoff: FrequencyHz, q: ParameterType) -> Self {
        let mut r = Self {
            inner: BiQuadFilter::new_with(sample_rate, cutoff, q),
        };
        r.update_coefficients(sample_rate, cutoff, q);
        r
    }

    fn update_coefficients(
        &mut self,
        sample_rate: usize,
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
#[derive(Clone, Debug)]
pub struct BiQuadFilter {
    sample_rate: usize,
    cutoff: FrequencyHz,
    param2: ParameterType,

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

    pub fn new_with(sample_rate: usize, cutoff: FrequencyHz, param2: ParameterType) -> Self {
        Self {
            sample_rate,
            cutoff,
            param2,
            coefficients: CoefficientSet::default(),
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

    fn rbj_intermediates_bandwidth(
        sample_rate: usize,
        cutoff: ParameterType,
        bandwidth: ParameterType,
    ) -> (f64, f64, f64, f64) {
        let w0 = 2.0f64 * PI * cutoff / sample_rate as f64;
        let w0cos = w0.cos();
        let w0sin = w0.sin();
        let alpha = w0sin * (2.0f64.ln() / 2.0 * bandwidth as f64 * w0 / w0.sin()).sinh();
        (w0, w0cos, w0sin, alpha)
    }

    fn rbj_intermediates_shelving(
        sample_rate: usize,
        cutoff: ParameterType,
        db_gain: ParameterType,
        s: f64,
    ) -> (f64, f64, f64, f64) {
        let w0 = 2.0f64 * PI * cutoff as f64 / sample_rate as f64;
        let w0cos = w0.cos();
        let w0sin = w0.sin();
        let alpha = w0sin / 2.0 * ((db_gain + 1.0 / db_gain) * (1.0 / s - 1.0) + 2.0).sqrt();
        (w0, w0cos, w0sin, alpha)
    }

    fn set_coefficients(&mut self, coefficient_set: CoefficientSet) {
        self.coefficients = coefficient_set;
    }
}

#[cfg(test)]
mod tests {
    // TODO: get FFT working, and then write tests.
}
