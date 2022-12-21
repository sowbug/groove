use crate::{
    clock::Clock,
    common::{MonoSample, MONO_SAMPLE_SILENCE},
    messages::EntityMessage,
    traits::{HasUid, IsEffect, Response, TransformsAudio, Updateable},
};
use strum_macros::{Display, EnumString, FromRepr};

pub(super) trait Delays {
    fn peek_output(&self, apply_decay: bool) -> MonoSample;
    fn peek_indexed_output(&self, index: isize) -> MonoSample;
    fn pop_output(&mut self, input: MonoSample) -> MonoSample;
}

#[derive(Debug, Default)]
pub(crate) struct DelayLine {
    sample_rate: usize,
    delay_seconds: f32,
    decay_factor: f32,

    buffer_size: usize,
    buffer_pointer: usize,
    buffer: Vec<MonoSample>,
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
    fn peek_output(&self, apply_decay: bool) -> MonoSample {
        if self.buffer_size == 0 {
            MONO_SAMPLE_SILENCE
        } else if apply_decay {
            self.decay_factor() * self.buffer[self.buffer_pointer]
        } else {
            self.buffer[self.buffer_pointer]
        }
    }

    fn peek_indexed_output(&self, index: isize) -> MonoSample {
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

    fn pop_output(&mut self, input: MonoSample) -> MonoSample {
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
    fn peek_output(&self, apply_decay: bool) -> MonoSample {
        self.delay.peek_output(apply_decay)
    }

    fn peek_indexed_output(&self, index: isize) -> MonoSample {
        self.delay.peek_indexed_output(index)
    }

    fn pop_output(&mut self, input: MonoSample) -> MonoSample {
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
    fn peek_output(&self, _apply_decay: bool) -> MonoSample {
        panic!("AllPassDelay doesn't allow peeking")
    }

    fn peek_indexed_output(&self, _: isize) -> MonoSample {
        panic!("AllPassDelay doesn't allow peeking")
    }

    fn pop_output(&mut self, input: MonoSample) -> MonoSample {
        let decay_factor = self.delay.decay_factor();
        let vm = self.delay.peek_output(false);
        let vn = input - (vm * decay_factor);
        self.delay.pop_output(vn);
        vm + vn * decay_factor
    }
}

#[derive(Display, Debug, EnumString, FromRepr)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum DelayControlParams {
    #[strum(serialize = "delay", serialize = "delay-seconds")]
    DelaySeconds,
}

#[derive(Debug, Default)]
pub struct Delay {
    uid: usize,
    delay: DelayLine,
}
impl IsEffect for Delay {}
impl TransformsAudio for Delay {
    fn transform_audio(&mut self, _clock: &Clock, input_sample: MonoSample) -> MonoSample {
        self.delay.pop_output(input_sample)
    }
}
impl Updateable for Delay {
    type Message = EntityMessage;

    #[allow(unused_variables)]
    fn update(&mut self, clock: &Clock, message: Self::Message) -> Response<Self::Message> {
        match message {
            Self::Message::UpdateF32(param_id, value) => {
                self.set_indexed_param_f32(param_id, value);
            }
            _ => todo!(),
        }
        Response::none()
    }

    fn set_indexed_param_f32(&mut self, index: usize, value: f32) {
        if let Some(param) = DelayControlParams::from_repr(index) {
            match param {
                DelayControlParams::DelaySeconds => self.delay.set_delay_seconds(value),
            }
        } else {
            todo!()
        }
    }
}
impl HasUid for Delay {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}

impl Delay {
    #[allow(dead_code)]
    fn new() -> Self {
        Self::default()
    }

    pub(crate) fn new_with(sample_rate: usize, delay_seconds: f32) -> Self {
        Self {
            uid: Default::default(),
            delay: DelayLine::new_with(sample_rate, delay_seconds, 1.0),
        }
    }

    pub fn delay_seconds(&self) -> f32 {
        self.delay.delay_seconds()
    }

    // pub fn set_delay_seconds(&mut self, delay_seconds: f32) {
    //     self.delay.set_delay_seconds(delay_seconds);
    // }
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;
    use more_asserts::{assert_gt, assert_lt};
    use rand::random;

    use super::*;
    use crate::clock::Clock;

    #[test]
    fn test_delay_basic() {
        let mut clock = Clock::default();
        let mut fx = Delay::new_with(clock.sample_rate(), 1.0);

        // Add a unique first sample.
        assert_eq!(fx.transform_audio(&clock, 0.5), 0.0);
        clock.tick();

        // Push a whole bunch more.
        for i in 0..clock.sample_rate() - 1 {
            assert_eq!(
                fx.transform_audio(&clock, 1.0),
                0.0,
                "unexpected value at sample {}",
                i
            );
            clock.tick();
        }

        // We should get back our first sentinel sample.
        assert_eq!(fx.transform_audio(&clock, 0.0), 0.5);
        clock.tick();

        // And the next should be one of the bunch.
        assert_eq!(fx.transform_audio(&clock, 0.0), 1.0);
        clock.tick();
    }

    #[test]
    fn test_delay_zero() {
        let mut clock = Clock::default();
        let mut fx = Delay::new_with(clock.sample_rate(), 0.0);

        // We should keep getting back what we put in.
        for i in 0..clock.sample_rate() {
            let mut sample: f32 = random();
            sample = sample.fract() * 2.0 - 1.0;
            assert_eq!(
                fx.transform_audio(&clock, sample),
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
