pub use orchestrator::Orchestrator;

mod orchestrator;

use core::fmt::Debug;
use crossbeam::deque::Worker;
use groove_core::{
    midi::HandlesMidi,
    traits::{IsController, IsEffect, Resets, TicksWithMessages, TransformsAudio},
    BipolarNormal, Sample, StereoSample,
};
use groove_entities::EntityMessage;
use groove_macros::{Control, Uid};
use std::str::FromStr;
use strum_macros::{Display, EnumString, FromRepr};

/// A Performance holds the output of an Orchestrator run.
#[derive(Debug)]
pub struct Performance {
    pub sample_rate: usize,
    pub worker: Worker<StereoSample>,
}

impl Performance {
    pub fn new_with(sample_rate: usize) -> Self {
        Self {
            sample_rate,
            worker: Worker::<StereoSample>::new_fifo(),
        }
    }
}
