use super::EffectTrait;

#[derive(Default)]
pub struct BitCrusher {
    bits_to_crush: u8,
}

impl BitCrusher {
    pub fn new(bits_to_crush: u8) -> Self {
        Self {
            bits_to_crush,
            ..Default::default()
        }
    }

    pub fn set_bits_to_crush(&mut self, n: u8) {
        self.bits_to_crush = n;
    }
}

impl EffectTrait for BitCrusher {
    fn process(&mut self, input: f32, _time_seconds: f32) -> f32 {
        let input_i16 = (input * (i16::MAX as f32)) as i16;
        let squished = input_i16 >> self.bits_to_crush;
        let expanded = squished << self.bits_to_crush;
        let to_f32 = expanded as f32 / (i16::MAX as f32);
        to_f32 
    }
}

#[cfg(test)]
mod tests {

    use std::{cell::RefCell, f32::consts::PI, rc::Rc};

    use crate::{
        common::{MidiMessage, MidiNote, WaveformType},
        primitives::{oscillators::MiniOscillator, tests::write_effect_to_file, ControllerTrait},
    };

    use super::*;

    struct TestController {
        target: Rc<RefCell<BitCrusher>>,
        start: u8,
        end: u8,
        duration: f32,

        time_start: f32,
    }

    impl TestController {
        pub fn new(target: Rc<RefCell<BitCrusher>>, start: u8, end: u8, duration: f32) -> Self {
            Self {
                target,
                start,
                end,
                duration,
                time_start: -1.0f32,
            }
        }
    }

    impl<'a> ControllerTrait for TestController {
        fn process(&mut self, time_seconds: f32) {
            if self.time_start < 0.0 {
                self.time_start = time_seconds;
            }
            if self.end != self.start {
                self.target.borrow_mut().set_bits_to_crush(
                    (self.start as f32
                        + ((time_seconds - self.time_start) / self.duration)
                            * (self.end as f32 - self.start as f32)) as u8,
                );
            }
        }
    }

    #[test]
    fn test_bitcrusher_basic() {
        let mut fx = BitCrusher::new(8);
        assert_eq!(fx.process(PI - 3.0, 0.0), 0.14062929);
    }

    #[test]
    fn write_bitcrusher_sample() {
        let mut osc = MiniOscillator::new(WaveformType::Sine);
        osc.set_frequency(MidiMessage::note_type_to_frequency(MidiNote::C4));
        let fx = Rc::new(RefCell::new(BitCrusher::new(8)));
        let mut controller = TestController::new(fx.clone(), 0, 16, 2.0);
        write_effect_to_file(
            &mut osc,
            fx,
            &mut Some(&mut controller),
            "effect_bitcrusher",
        );
    }
}
