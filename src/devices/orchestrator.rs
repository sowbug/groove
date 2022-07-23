use crate::devices::traits::DeviceTrait;
use crate::primitives::clock::{Clock, ClockSettings};
use crossbeam::deque::Worker;
use std::cell::RefCell;
use std::rc::Rc;

use super::mixer::Mixer;

#[derive(Default, Clone)]
pub struct Orchestrator {
    pub clock: Clock,

    master_mixer: Rc<RefCell<Mixer>>,
    devices: Vec<Rc<RefCell<dyn DeviceTrait>>>,
}

impl Orchestrator {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            clock: Clock::new(ClockSettings::new(sample_rate, 128.0, (4, 4))),
            master_mixer: Rc::new(RefCell::new(Mixer::new())),
            devices: Vec::new(),
        }
    }

    pub fn new_44100() -> Self {
        Self::new(44100)
    }

    pub fn add_device(&mut self, device: Rc<RefCell<dyn DeviceTrait>>) {
        self.devices.push(device);
    }

    fn tick(&mut self) -> (f32, bool) {
        let mut done = true;
        for d in self.devices.clone() {
            if d.borrow().sources_midi() {
                done = d.borrow_mut().tick(&self.clock) && done;
            }
        }
        for d in self.devices.clone() {
            if d.borrow().sources_audio() {
                done = d.borrow_mut().tick(&self.clock) && done;
            }
        }
        self.clock.tick();
        (self.master_mixer.borrow().get_audio_sample(), done)
    }

    pub fn perform_to_queue(&mut self, worker: &Worker<f32>) -> anyhow::Result<()> {
        loop {
            let (sample, done) = self.tick();
            worker.push(sample);
            if done {
                break;
            }
        }
        Ok(())
    }

    pub(crate) fn add_master_mixer_source(&self, device: Rc<RefCell<dyn DeviceTrait>>) {
        self.master_mixer.borrow_mut().add_audio_source(device);
    }
}
