// Copyright (c) 2023 Mike Tsao. All rights reserved.

use groove_core::{
    traits::{IsEffect, TransformsAudio},
    Normal, ParameterType, Sample, SignalType,
};
use groove_proc_macros::{Control, Synchronization, Uid};
use std::str::FromStr;
use strum::EnumCount;
use strum_macros::{
    Display, EnumCount as EnumCountMacro, EnumIter, EnumString, FromRepr, IntoStaticStr,
};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

pub(super) trait Delays {
    fn peek_output(&self, apply_decay: bool) -> Sample;
    fn peek_indexed_output(&self, index: isize) -> Sample;
    fn pop_output(&mut self, input: Sample) -> Sample;
}

#[derive(Debug, Default)]
pub(crate) struct DelayLine {
    sample_rate: usize,
    delay_seconds: f32,
    decay_factor: SignalType,

    buffer_size: usize,
    buffer_pointer: usize,
    buffer: Vec<Sample>,
}
impl DelayLine {
    /// decay_factor: 1.0 = no decay
    pub(super) fn new_with(
        sample_rate: usize,
        delay_seconds: ParameterType,
        decay_factor: SignalType,
    ) -> Self {
        let mut r = Self {
            sample_rate,
            delay_seconds: delay_seconds as f32,
            decay_factor,

            buffer_size: Default::default(),
            buffer_pointer: 0,
            buffer: Default::default(),
        };
        r.resize_buffer();
        r
    }

    pub(super) fn delay_seconds(&self) -> f32 {
        self.delay_seconds
    }

    pub(super) fn set_delay_seconds(&mut self, delay_seconds: f32) {
        if delay_seconds != self.delay_seconds {
            self.delay_seconds = delay_seconds;
            self.resize_buffer();
        }
    }

    fn resize_buffer(&mut self) {
        self.buffer_size = (self.sample_rate as f32 * self.delay_seconds) as usize;
        self.buffer = Vec::with_capacity(self.buffer_size);
        self.buffer.resize(self.buffer_size, Sample::SILENCE);
        self.buffer_pointer = 0;
    }

    pub(super) fn decay_factor(&self) -> SignalType {
        self.decay_factor
    }
}
impl Delays for DelayLine {
    fn peek_output(&self, apply_decay: bool) -> Sample {
        if self.buffer_size == 0 {
            Sample::SILENCE
        } else if apply_decay {
            self.buffer[self.buffer_pointer] * self.decay_factor()
        } else {
            self.buffer[self.buffer_pointer]
        }
    }

    fn peek_indexed_output(&self, index: isize) -> Sample {
        if self.buffer_size == 0 {
            Sample::SILENCE
        } else {
            let mut index = -index;
            while index < 0 {
                index += self.buffer_size as isize;
            }
            self.buffer[self.buffer_pointer]
        }
    }

    fn pop_output(&mut self, input: Sample) -> Sample {
        if self.buffer_size == 0 {
            input
        } else {
            let out = self.peek_output(true);
            self.buffer[self.buffer_pointer] = input;
            self.buffer_pointer += 1;
            if self.buffer_pointer >= self.buffer_size {
                self.buffer_pointer = 0;
            }
            out
        }
    }
}

#[derive(Debug, Default)]
pub struct RecirculatingDelayLine {
    delay: DelayLine,
}
impl RecirculatingDelayLine {
    pub(super) fn new_with(
        sample_rate: usize,
        delay_seconds: ParameterType,
        decay_seconds: ParameterType,
        final_amplitude: Normal,
        peak_amplitude: Normal,
    ) -> Self {
        Self {
            delay: DelayLine::new_with(
                sample_rate,
                delay_seconds,
                (peak_amplitude.value() * final_amplitude.value())
                    .powf(delay_seconds / decay_seconds) as SignalType,
            ),
        }
    }

    pub(super) fn decay_factor(&self) -> SignalType {
        self.delay.decay_factor()
    }
}
impl Delays for RecirculatingDelayLine {
    fn peek_output(&self, apply_decay: bool) -> Sample {
        self.delay.peek_output(apply_decay)
    }

    fn peek_indexed_output(&self, index: isize) -> Sample {
        self.delay.peek_indexed_output(index)
    }

    fn pop_output(&mut self, input: Sample) -> Sample {
        let output = self.peek_output(true);
        self.delay.pop_output(input + output);
        output
    }
}

#[derive(Debug, Default)]
pub(super) struct AllPassDelayLine {
    delay: RecirculatingDelayLine,
}
impl AllPassDelayLine {
    pub(super) fn new_with(
        sample_rate: usize,
        delay_seconds: ParameterType,
        decay_seconds: ParameterType,
        final_amplitude: Normal,
        peak_amplitude: Normal,
    ) -> Self {
        Self {
            delay: RecirculatingDelayLine::new_with(
                sample_rate,
                delay_seconds,
                decay_seconds,
                final_amplitude,
                peak_amplitude,
            ),
        }
    }
}

impl Delays for AllPassDelayLine {
    fn peek_output(&self, _apply_decay: bool) -> Sample {
        panic!("AllPassDelay doesn't allow peeking")
    }

    fn peek_indexed_output(&self, _: isize) -> Sample {
        panic!("AllPassDelay doesn't allow peeking")
    }

    fn pop_output(&mut self, input: Sample) -> Sample {
        let decay_factor = self.delay.decay_factor();
        let vm = self.delay.peek_output(false);
        let vn = input - (vm * decay_factor);
        self.delay.pop_output(vn);
        vm + vn * decay_factor
    }
}

#[derive(Clone, Copy, Debug, Default, Synchronization)]
#[cfg_attr(
    feature = "serialization",
    derive(Serialize, Deserialize),
    serde(rename = "delay", rename_all = "kebab-case")
)]
pub struct DelayParams {
    #[sync]
    pub seconds: ParameterType,
}

#[derive(Control, Debug, Default, Uid)]
pub struct Delay {
    uid: usize,

    params: DelayParams,

    delay: DelayLine,
}
impl IsEffect for Delay {}
impl TransformsAudio for Delay {
    fn transform_channel(&mut self, _channel: usize, input_sample: Sample) -> Sample {
        self.delay.pop_output(input_sample)
    }
}
impl Delay {
    #[allow(dead_code)]
    fn new() -> Self {
        Self::default()
    }

    pub fn new_with(sample_rate: usize, params: DelayParams) -> Self {
        Self {
            params,
            delay: DelayLine::new_with(sample_rate, params.seconds(), 1.0),
            ..Default::default()
        }
    }

    pub fn seconds(&self) -> f32 {
        self.delay.delay_seconds()
    }

    pub fn set_seconds(&mut self, delay_seconds: f32) {
        self.delay.set_delay_seconds(delay_seconds);
    }

    pub fn set_control_seconds(&mut self, value: groove_core::control::F32ControlValue) {
        self.set_seconds(value.0);
    }

    pub fn params(&self) -> DelayParams {
        self.params
    }

    pub fn update(&mut self, message: DelayParamsMessage) {
        self.params.update(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::DEFAULT_SAMPLE_RATE;
    use float_cmp::approx_eq;
    use groove_core::SampleType;
    use more_asserts::{assert_gt, assert_lt};
    use rand::random;

    #[test]
    fn test_delay_basic() {
        let mut fx = Delay::new_with(DEFAULT_SAMPLE_RATE, DelayParams { seconds: 1.0 });

        // Add a unique first sample.
        assert_eq!(fx.transform_channel(0, Sample::from(0.5)), Sample::SILENCE);

        // Push a whole bunch more.
        for i in 0..DEFAULT_SAMPLE_RATE - 1 {
            assert_eq!(
                fx.transform_channel(0, Sample::MAX),
                Sample::SILENCE,
                "unexpected value at sample {}",
                i
            );
        }

        // We should get back our first sentinel sample.
        assert_eq!(fx.transform_channel(0, Sample::SILENCE), Sample::from(0.5));

        // And the next should be one of the bunch.
        assert_eq!(fx.transform_channel(0, Sample::SILENCE), Sample::MAX);
    }

    #[test]
    fn test_delay_zero() {
        let mut fx = Delay::new_with(DEFAULT_SAMPLE_RATE, DelayParams { seconds: 0.0 });

        // We should keep getting back what we put in.
        for i in 0..DEFAULT_SAMPLE_RATE {
            let random_bipolar_normal = random::<f32>().fract() * 2.0 - 1.0;
            let sample = Sample::from(random_bipolar_normal);
            assert_eq!(
                fx.transform_channel(0, sample),
                sample,
                "unexpected value at sample {}",
                i
            );
        }
    }

    #[test]
    fn test_delay_line() {
        // It's very simple: it should return an input sample, attenuated, after
        // the specified delay.
        let mut delay = DelayLine::new_with(3, 1.0, 0.3);
        assert_eq!(delay.pop_output(Sample::from(0.5)), Sample::SILENCE);
        assert_eq!(delay.pop_output(Sample::from(0.4)), Sample::SILENCE);
        assert_eq!(delay.pop_output(Sample::from(0.3)), Sample::SILENCE);
        assert_eq!(delay.pop_output(Sample::from(0.2)), Sample::from(0.5 * 0.3));
    }

    #[test]
    fn test_recirculating_delay_line() {
        // Recirculating means that the input value is added to the value at the
        // back of the buffer, rather than replacing that value. So if we put in
        // a single value, we should expect to get it back, usually quieter,
        // each time it cycles through the buffer.
        let mut delay =
            RecirculatingDelayLine::new_with(3, 1.0, 1.5, Normal::from(0.001), Normal::from(1.0));
        assert_eq!(delay.pop_output(Sample::from(0.5)), Sample::SILENCE);
        assert_eq!(delay.pop_output(Sample::from(0.0)), Sample::SILENCE);
        assert_eq!(delay.pop_output(Sample::from(0.0)), Sample::SILENCE);
        assert!(approx_eq!(
            SampleType,
            delay.pop_output(Sample::from(0.0)).0,
            Sample::from(0.5 * 0.01).0,
            epsilon = 0.001
        ));
        assert_eq!(delay.pop_output(Sample::from(0.0)), Sample::SILENCE);
        assert_eq!(delay.pop_output(Sample::from(0.0)), Sample::SILENCE);
        assert!(approx_eq!(
            SampleType,
            delay.pop_output(Sample::from(0.0)).0,
            Sample::from(0.5 * 0.01 * 0.01).0,
            epsilon = 0.001
        ));
        assert_eq!(delay.pop_output(Sample::from(0.0)), Sample::SILENCE);
        assert_eq!(delay.pop_output(Sample::from(0.0)), Sample::SILENCE);
    }

    #[test]
    fn test_allpass_delay_line() {
        // TODO: I'm not sure what this delay line is supposed to do.
        let mut delay =
            AllPassDelayLine::new_with(3, 1.0, 1.5, Normal::from(0.001), Normal::from(1.0));
        assert_lt!(delay.pop_output(Sample::from(0.5)), Sample::from(0.5));
        assert_eq!(delay.pop_output(Sample::from(0.0)), Sample::SILENCE);
        assert_eq!(delay.pop_output(Sample::from(0.0)), Sample::SILENCE);
        assert_gt!(delay.pop_output(Sample::from(0.0)), Sample::SILENCE); // Note! > not =
    }
}
