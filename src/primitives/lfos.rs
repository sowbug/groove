use crate::backend::devices::DeviceTrait;

//https://stackoverflow.com/questions/27831944/how-do-i-store-a-closure-in-a-struct-in-rust
pub struct Lfo {
    frequency: f32,
    current_value: f32,
    target: Option<Box<dyn FnMut(f32) -> ()>>,
}

impl DeviceTrait for Lfo {
    fn sinks_midi(&self) -> bool {
        true
    }
    fn sources_midi(&self) -> bool {
        true
    }

    fn tick(&mut self, clock: &crate::backend::clock::Clock) -> bool {
        let phase_normalized = self.frequency * (clock.seconds as f32);
        self.current_value = 2.0 * (phase_normalized - (0.5 + phase_normalized).floor());
        match &mut self.target {
            Some(tfn) => (tfn)(self.current_value),
            None => {}
        }
        true
    }

    fn get_audio_sample(&self) -> f32 {
        self.current_value
    }
}

impl Lfo {
    pub fn new(frequency: f32) -> Lfo {
        Lfo {
            frequency,
            current_value: 0.,
            target: Option::None,
        }
    }
    pub fn connect_automation_sink(&mut self, target: impl FnMut(f32) -> () + 'static) {
        self.target = Option::Some(Box::new(target));
    }
}

// TODO: is this just extra stuff hung off Oscillator?

#[cfg(test)]
mod tests {

    use std::{cell::RefCell, rc::Rc};

    use more_asserts::assert_gt;

    use crate::{
        backend::clock::Clock,
        primitives::{self, oscillators::Oscillator},
    };

    use super::*;

    impl Lfo {
        fn new_test_1hz() -> Lfo {
            Lfo::new(1.)
        }
    }

    #[test]
    fn test_lfo_shape() {
        let mut clock = Clock::new_test();
        let mut lfo_1hz = Lfo::new_test_1hz();

        assert_eq!(lfo_1hz.frequency, 1.);

        lfo_1hz.tick(&clock);
        assert_eq!(lfo_1hz.get_audio_sample(), 0.);

        // test that sawtooth's first half is positive
        loop {
            clock.tick();
            lfo_1hz.tick(&clock);
            dbg!(clock.seconds);
            dbg!(lfo_1hz.get_audio_sample());
            if clock.seconds >= 0.5 {
                break;
            }
            assert!(lfo_1hz.get_audio_sample() > 0.);
        }
        assert_eq!(clock.samples, Clock::TEST_SAMPLE_RATE / 2);
        assert_eq!(lfo_1hz.get_audio_sample(), -1.);

        // test that sawtooth's second half is negative
        loop {
            clock.tick();
            lfo_1hz.tick(&clock);
            dbg!(clock.seconds);
            dbg!(lfo_1hz.get_audio_sample());
            if clock.seconds >= 1. {
                break;
            }
            assert!(lfo_1hz.get_audio_sample() < 0.);
        }
        assert_eq!(clock.samples, Clock::TEST_SAMPLE_RATE);
        assert_eq!(lfo_1hz.get_audio_sample(), 0.);
    }

    #[test]
    fn test_automation() {
        let mut clock = Clock::new_test();

        let oscillator = Rc::new(RefCell::new(Oscillator::new(
            primitives::oscillators::Waveform::Sine,
        )));
        oscillator.borrow_mut().set_frequency(440.);

        let mut lfo = Lfo::new_test_1hz();
        let o2 = oscillator.clone();
        let thefn = move |value: f32| -> () {
            let frequency = o2.borrow().get_frequency();
            let mut o = o2.borrow_mut();
            o.set_frequency(frequency + frequency * value * 0.05);
        };
        lfo.connect_automation_sink(thefn);

        oscillator.borrow_mut().tick(&clock);
        lfo.tick(&clock);
        assert_eq!(oscillator.borrow_mut().get_frequency(), 440.);

        clock.tick();
        oscillator.borrow_mut().tick(&clock);
        lfo.tick(&clock);
        assert_gt!(oscillator.borrow_mut().get_frequency(), 440.);
    }
}
