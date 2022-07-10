use crate::backend::devices::DeviceTrait;
use crate::primitives::clock::Clock;
use crossbeam::deque::Worker;
use std::cell::RefCell;
use std::rc::Rc;

use super::devices::Mixer;

pub struct Orchestrator {
    pub clock: Clock,

    pub master_mixer: Rc<RefCell<Mixer>>, // TODO(miket): should be private
    devices: Vec<Rc<RefCell<dyn DeviceTrait>>>,
}

impl Orchestrator {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            clock: Clock::new(sample_rate, 4, 4, 128.0),
            master_mixer: Rc::new(RefCell::new(Mixer::new())),
            devices: Vec::new(),
        }
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
}
