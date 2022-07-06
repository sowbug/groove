use crate::backend::devices::DeviceTrait;

struct Lfo {}

impl DeviceTrait for Lfo {
    fn sinks_midi(&self) -> bool {
        true
    }

    fn sources_automation(&self) -> bool {
        true
    }
}

impl Lfo {
    pub fn new() -> Lfo {
        Lfo {}
    }
}
