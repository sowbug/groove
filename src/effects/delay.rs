use std::collections::VecDeque;

use crate::{
    clock::Clock,
    common::{MonoSample, MONO_SAMPLE_SILENCE},
    messages::EntityMessage,
    traits::{HasUid, IsEffect, Response, TransformsAudio, Updateable},
};
use strum_macros::{Display, EnumString, FromRepr};

#[derive(Display, Debug, EnumString, FromRepr)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum DelayControlParams {
    #[strum(serialize = "delay", serialize = "delay-seconds")]
    DelaySeconds,
}

#[derive(Debug)]
pub struct Delay {
    uid: usize,
    sample_rate: usize,
    delay_seconds: f32,
    buffer_size: usize,
    buffer: VecDeque<MonoSample>,
}
impl IsEffect for Delay {}
impl TransformsAudio for Delay {
    fn transform_audio(&mut self, _clock: &Clock, input_sample: MonoSample) -> MonoSample {
        if self.buffer_size == 0 {
            input_sample
        } else {
            self.buffer.push_back(input_sample);
            if self.buffer.len() == self.buffer_size {
                self.buffer.pop_front().unwrap_or(MONO_SAMPLE_SILENCE)
            } else {
                MONO_SAMPLE_SILENCE
            }
        }
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
                DelayControlParams::DelaySeconds => self.set_delay_seconds(value),
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
impl Default for Delay {
    fn default() -> Self {
        let mut r = Self::default_without_alloc();
        r.set_delay_seconds(0.5);
        r
    }
}

impl Delay {
    #[allow(dead_code)]
    fn new() -> Self {
        Self::default()
    }

    // This exists because we want both DRY and efficiency. We set the defaults
    // in one place (here), but new_with() doesn't cause one gigantic
    // ring-buffer allocation that's immediately thrown away and replaced with a
    // new one.
    fn default_without_alloc() -> Self {
        Self {
            uid: Default::default(),
            sample_rate: Clock::default().sample_rate(),
            delay_seconds: 0.0,
            buffer_size: 0,
            buffer: Default::default(),
        }
    }

    pub(crate) fn new_with(sample_rate: usize, delay_seconds: f32) -> Self {
        let mut r = Self::default_without_alloc();
        r.sample_rate = sample_rate;
        r.set_delay_seconds(delay_seconds);
        r
    }

    pub fn delay_seconds(&self) -> f32 {
        self.delay_seconds
    }

    pub fn set_delay_seconds(&mut self, delay_seconds: f32) {
        if delay_seconds != self.delay_seconds {
            self.delay_seconds = delay_seconds;
            self.resize_buffer();
        }
    }

    fn resize_buffer(&mut self) {
        self.buffer_size = (self.sample_rate as f32 * self.delay_seconds) as usize;
        self.buffer = VecDeque::with_capacity(self.buffer_size);
    }
}

#[cfg(test)]
mod tests {
    use rand::random;

    use super::*;
    use crate::clock::Clock;

    #[test]
    fn test_delay_basic() {
        let mut clock = Clock::default();
        let mut fx = Delay::new_with(clock.sample_rate(), 1.0);

        // Add a unique first sample.
        assert_eq!(fx.transform_audio(&clock, 0.5), 0.0,);
        clock.tick();

        // Push a whole bunch more.
        for i in 0..clock.sample_rate() - 2 {
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
}
