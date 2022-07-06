use std::{cell::RefCell, rc::Rc};

use crate::backend::devices::{DeviceTrait, AutomatableTrait};

struct Lfo {
    frequency: f32,
    current_value: f32,
    targets: Vec<Rc<RefCell<dyn AutomatableTrait>>>,
}

impl DeviceTrait for Lfo {
    fn sinks_midi(&self) -> bool {
        true
    }

    fn sources_automation(&self) -> bool {
        true
    }

    fn tick(&mut self, clock: &crate::backend::clock::Clock) -> bool {
        let phase_normalized = self.frequency * (clock.seconds as f32);
        self.current_value = 2.0 * (phase_normalized - (0.5 + phase_normalized).floor());
        for target in self.targets.iter_mut() {
            target.borrow_mut().handle_automation(self.current_value);
        }
        false
    }
}

impl Lfo {
    pub fn new(frequency: f32) -> Lfo {
        Lfo {
            frequency,
            current_value: 0.,
            targets: Vec::new(),
        }
    }
}


// TODO: is this just extra stuff hung off Oscillator?