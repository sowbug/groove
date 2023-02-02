use crate::{
    clock::Clock,
    common::{F32ControlValue, OldMonoSample, Sample, MONO_SAMPLE_SILENCE},
    messages::EntityMessage,
    traits::{Controllable, HasUid, IsEffect, TransformsAudio, Updateable},
};
use groove_macros::{Control, Uid};
use std::{marker::PhantomData, str::FromStr};
use strum_macros::{Display, EnumString, FromRepr};

pub(super) trait Delays {
    fn peek_output(&self, apply_decay: bool) -> OldMonoSample;
    fn peek_indexed_output(&self, index: isize) -> OldMonoSample;
    fn pop_output(&mut self, input: OldMonoSample) -> OldMonoSample;
}

#[derive(Debug, Default)]
pub(crate) struct DelayLine {
    sample_rate: usize,
    delay_seconds: f32,
    decay_factor: f32,

    buffer_size: usize,
    buffer_pointer: usize,
    buffer: Vec<OldMonoSample>,
}
impl DelayLine {
    /// decay_factor: 1.0 = no decay
    pub(super) fn new_with(sample_rate: usize, delay_seconds: f32, decay_factor: f32) -> Self {
        let mut r = Self {
            sample_rate,
            delay_seconds,
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
        self.buffer.resize(self.buffer_size, MONO_SAMPLE_SILENCE);
        self.buffer_pointer = 0;
    }

    pub(super) fn decay_factor(&self) -> f32 {
        self.decay_factor
    }
}
impl Delays for DelayLine {
    fn peek_output(&self, apply_decay: bool) -> OldMonoSample {
        if self.buffer_size == 0 {
            MONO_SAMPLE_SILENCE
        } else if apply_decay {
            self.decay_factor() * self.buffer[self.buffer_pointer]
        } else {
            self.buffer[self.buffer_pointer]
        }
    }

    fn peek_indexed_output(&self, index: isize) -> OldMonoSample {
        if self.buffer_size == 0 {
            MONO_SAMPLE_SILENCE
        } else {
            let mut index = -index;
            while index < 0 {
                index += self.buffer_size as isize;
            }
            self.buffer[self.buffer_pointer]
        }
    }

    fn pop_output(&mut self, input: OldMonoSample) -> OldMonoSample {
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
        delay_seconds: f32,
        decay_seconds: f32,
        final_amplitude: f32,
        peak_amplitude: f32,
    ) -> Self {
        Self {
            delay: DelayLine::new_with(
                sample_rate,
                delay_seconds,
                (peak_amplitude * final_amplitude).powf(delay_seconds / decay_seconds),
            ),
        }
    }

    pub(super) fn decay_factor(&self) -> f32 {
        self.delay.decay_factor()
    }
}
impl Delays for RecirculatingDelayLine {
    fn peek_output(&self, apply_decay: bool) -> OldMonoSample {
        self.delay.peek_output(apply_decay)
    }

    fn peek_indexed_output(&self, index: isize) -> OldMonoSample {
        self.delay.peek_indexed_output(index)
    }

    fn pop_output(&mut self, input: OldMonoSample) -> OldMonoSample {
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
        delay_seconds: f32,
        decay_seconds: f32,
        final_amplitude: f32,
        peak_amplitude: f32,
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
    fn peek_output(&self, _apply_decay: bool) -> OldMonoSample {
        panic!("AllPassDelay doesn't allow peeking")
    }

    fn peek_indexed_output(&self, _: isize) -> OldMonoSample {
        panic!("AllPassDelay doesn't allow peeking")
    }

    fn pop_output(&mut self, input: OldMonoSample) -> OldMonoSample {
        let decay_factor = self.delay.decay_factor();
        let vm = self.delay.peek_output(false);
        let vn = input - (vm * decay_factor);
        self.delay.pop_output(vn);
        vm + vn * decay_factor
    }
}

#[derive(Control, Debug, Default, Uid)]
pub struct Delay {
    uid: usize,

    #[controllable]
    seconds: PhantomData<f32>,

    delay: DelayLine,
}
impl IsEffect for Delay {}
impl TransformsAudio for Delay {
    fn transform_channel(
        &mut self,
        _clock: &Clock,
        _channel: usize,
        input_sample: crate::common::Sample,
    ) -> crate::common::Sample {
        let input_sample = input_sample.0 as OldMonoSample;
        Sample::from(self.delay.pop_output(input_sample))
    }
}
impl Updateable for Delay {
    type Message = EntityMessage;
}

impl Delay {
    #[allow(dead_code)]
    fn new() -> Self {
        Self::default()
    }

    pub(crate) fn new_with(sample_rate: usize, delay_seconds: f32) -> Self {
        Self {
            delay: DelayLine::new_with(sample_rate, delay_seconds, 1.0),
            ..Default::default()
        }
    }

    pub fn seconds(&self) -> f32 {
        self.delay.delay_seconds()
    }

    pub fn set_seconds(&mut self, delay_seconds: f32) {
        self.delay.set_delay_seconds(delay_seconds);
    }

    pub(crate) fn set_control_seconds(&mut self, value: F32ControlValue) {
        self.set_seconds(value.0);
    }
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;
    use more_asserts::{assert_gt, assert_lt};
    use rand::random;

    use super::*;
    use crate::{clock::Clock, common::Sample};

    #[test]
    fn test_delay_basic() {
        let mut clock = Clock::default();
        let mut fx = Delay::new_with(clock.sample_rate(), 1.0);

        // Add a unique first sample.
        assert_eq!(
            fx.transform_channel(&clock, 0, Sample::from(0.5)),
            Sample::SILENCE
        );
        clock.tick();

        // Push a whole bunch more.
        for i in 0..clock.sample_rate() - 1 {
            assert_eq!(
                fx.transform_channel(&clock, 0, Sample::MAX),
                Sample::SILENCE,
                "unexpected value at sample {}",
                i
            );
            clock.tick();
        }

        // We should get back our first sentinel sample.
        assert_eq!(
            fx.transform_channel(&clock, 0, Sample::SILENCE),
            Sample::from(0.5)
        );
        clock.tick();

        // And the next should be one of the bunch.
        assert_eq!(
            fx.transform_channel(&clock, 0, Sample::SILENCE),
            Sample::MAX
        );
        clock.tick();
    }

    #[test]
    fn test_delay_zero() {
        let mut clock = Clock::default();
        let mut fx = Delay::new_with(clock.sample_rate(), 0.0);

        // We should keep getting back what we put in.
        for i in 0..clock.sample_rate() {
            let random_bipolar_normal = random::<f32>().fract() * 2.0 - 1.0;
            let sample = Sample::from(random_bipolar_normal);
            assert_eq!(
                fx.transform_channel(&clock, 0, sample),
                sample,
                "unexpected value at sample {}",
                i
            );
            clock.tick();
        }
    }

    #[test]
    fn test_delay_line() {
        // It's very simple: it should return an input sample, attenuated, after
        // the specified delay.
        let mut delay = DelayLine::new_with(3, 1.0, 0.3);
        assert_eq!(delay.pop_output(0.5), 0.0);
        assert_eq!(delay.pop_output(0.4), 0.0);
        assert_eq!(delay.pop_output(0.3), 0.0);
        assert_eq!(delay.pop_output(0.2), 0.5 * 0.3);
    }

    #[test]
    fn test_recirculating_delay_line() {
        // Recirculating means that the input value is added to the value at the
        // back of the buffer, rather than replacing that value. So if we put in
        // a single value, we should expect to get it back, usually quieter,
        // each time it cycles through the buffer.
        let mut delay = RecirculatingDelayLine::new_with(3, 1.0, 1.5, 0.001, 1.0);
        assert_eq!(delay.pop_output(0.5), 0.0);
        assert_eq!(delay.pop_output(0.0), 0.0);
        assert_eq!(delay.pop_output(0.0), 0.0);
        assert_approx_eq!(delay.pop_output(0.0), 0.5 * 0.01, 0.001);
        assert_eq!(delay.pop_output(0.0), 0.0);
        assert_eq!(delay.pop_output(0.0), 0.0);
        assert_approx_eq!(delay.pop_output(0.0), 0.5 * 0.01 * 0.01, 0.001);
        assert_eq!(delay.pop_output(0.0), 0.0);
        assert_eq!(delay.pop_output(0.0), 0.0);
    }

    #[test]
    fn test_allpass_delay_line() {
        // TODO: I'm not sure what this delay line is supposed to do.
        let mut delay = AllPassDelayLine::new_with(3, 1.0, 1.5, 0.001, 1.0);
        assert_lt!(delay.pop_output(0.5), 0.5);
        assert_eq!(delay.pop_output(0.0), 0.0);
        assert_eq!(delay.pop_output(0.0), 0.0);
        assert_gt!(delay.pop_output(0.0), 0.0); // Note! > not =
    }
}
