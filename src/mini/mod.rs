// Copyright (c) 2023 Mike Tsao. All rights reserved.

pub use drag_drop::{DragDropManager, DragDropSource};
pub use entities::{register_mini_factory_entities, NewIsController, NewIsEffect, NewIsInstrument};
pub use entity_factory::{EntityFactory, Key};
pub use orchestrator::MiniOrchestrator;
pub use sequencer::{MiniSequencer, MiniSequencerParams};
pub use track::TrackIndex; // TODO: this is weird to have to export without Track

#[cfg(test)]
pub use entities::register_test_factory_entities;

use crossbeam_channel::{Receiver, Sender};
use groove_core::Uid;
use serde::{Deserialize, Serialize};

mod drag_drop;
mod entities;
mod entity_factory;
mod orchestrator;
mod sequencer;
mod track;

/// Generates unique [Uid]s. This factory is not threadsafe.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct UidFactory {
    previous_uid: Uid,
}
impl UidFactory {
    /// Generates the next unique [Uid].
    pub fn next(&mut self) -> Uid {
        self.previous_uid.increment().clone()
    }
}

/// A convenience struct to bundle both halves of a [crossbeam_channel]
/// together.
///
/// This is actually for more than just convenience: because Serde needs to be
/// able to assign defaults to individual fields on a struct by calling
/// stateless functions, we have to create both sender and receiver at once in a
/// single field.
#[derive(Debug)]
pub struct ChannelPair<T> {
    #[allow(missing_docs)]
    pub sender: Sender<T>,
    #[allow(missing_docs)]
    pub receiver: Receiver<T>,
}
impl<T> Default for ChannelPair<T> {
    fn default() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        Self { sender, receiver }
    }
}
