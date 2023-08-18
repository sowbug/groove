// Copyright (c) 2023 Mike Tsao. All rights reserved.

pub use drag_drop::{DragDropManager, DragDropSource};
pub use entities::register_factory_entities;
pub use entity_factory::{EntityFactory, Key};
pub use orchestrator::{Orchestrator, OrchestratorAction, OrchestratorBuilder};
pub use piano_roll::{Note, Pattern, PatternBuilder, PatternUid};
pub use selection_set::SelectionSet;
pub use sequencer::{ArrangedPattern, ArrangedPatternBuilder, Sequencer, SequencerParams};
pub use track::{Track, TrackAction, TrackTitle, TrackUid};
pub use transport::Transport;

#[cfg(test)]
pub use entities::register_test_factory_entities;

use crossbeam_channel::{Receiver, Sender};
use groove_core::IsUid;
use serde::{Deserialize, Serialize};

mod bus_station;
mod control_atlas;
mod control_router;
mod drag_drop;
mod entities;
mod entity_factory;
mod humidifier;
mod midi_router;
mod orchestrator;
mod piano_roll;
mod selection_set;
mod sequencer;
mod track;
mod transport;
/// egui widgets
pub mod widgets;

/// Generates unique [Uid]s. This factory is not threadsafe.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct UidFactory<U: IsUid + Clone> {
    previous_uid: U,
}
impl<U: IsUid + Clone> UidFactory<U> {
    /// Generates the next unique [Uid].
    pub fn next(&mut self) -> U {
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
