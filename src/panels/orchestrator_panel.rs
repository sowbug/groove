// Copyright (c) 2023 Mike Tsao. All rights reserved.

use anyhow::{anyhow, Result};
use crossbeam_channel::{Receiver, Sender};
use eframe::egui::Ui;
use ensnare_core::{
    core::ChannelPair,
    entities::EntityFactory,
    midi::{MidiChannel, MidiMessage},
    orchestration::{Orchestrator, OrchestratorAction, OrchestratorBuilder},
    prelude::*,
    selection_set::SelectionSet,
    track::TrackUid,
    traits::prelude::*,
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
};

/// Commands that [Orchestrator] accepts.
#[derive(Clone, Debug)]
pub enum OrchestratorInput {
    /// An external MIDI message arrived.
    Midi(MidiChannel, MidiMessage),
    /// Open the project file at the given path, load it, and replace the
    /// current [Orchestrator] instance with it.
    ProjectOpen(PathBuf),
    /// Create a new blank project.
    ProjectNew,
    /// Start playing the current project.
    ProjectPlay,
    /// Save the current project to the specified file.
    ProjectSave(PathBuf),
    /// Stop playing the current project.
    ProjectStop,
    /// Delete all selected tracks.
    TrackDeleteSelected,
    /// Duplicate all selected tracks, placing the new one(s) below them.
    TrackDuplicateSelected,
    /// Create a new audio track.
    TrackNewAudio,
    /// Create a new aux track.
    TrackNewAux,
    /// Create a new MIDI track.
    TrackNewMidi,
    /// Delete the selected arranged patterns.
    TrackPatternRemoveSelected,
    /// Add a new entity to the selected track.
    TrackAddEntity(EntityKey),
    /// Sets the tempo.
    Tempo(Tempo),

    /// Quit the thread.
    Quit,
}

/// Events that [Orchestrator] generates.
#[derive(Debug)]
pub enum OrchestratorEvent {
    /// This is the current [Tempo].
    Tempo(Tempo),

    /// A new, empty project was created.
    New,

    /// A project has been successfully opened from the specified path with the
    /// specified title (if any).
    Loaded(PathBuf, Option<String>),
    /// A project failed to load.
    LoadError(PathBuf, anyhow::Error),

    /// The current project was successfully saved to the specified path.
    Saved(PathBuf),
    /// An attempt to save the current project failed.
    SaveError(PathBuf, anyhow::Error),

    /// Acknowledge request to quit.
    Quit,
}

/// An egui panel that renders a [Orchestrator].
#[derive(Debug)]
pub struct OrchestratorPanel {
    orchestrator: Arc<Mutex<Orchestrator>>,
    track_selection_set: Arc<Mutex<SelectionSet<TrackUid>>>,
    input_channel_pair: ChannelPair<OrchestratorInput>,
    event_channel_pair: ChannelPair<OrchestratorEvent>,
}
impl Default for OrchestratorPanel {
    fn default() -> Self {
        let mut orchestrator = OrchestratorBuilder::default().build().unwrap();
        let _ = orchestrator.create_starter_tracks();
        let mut r = Self {
            orchestrator: Arc::new(Mutex::new(orchestrator)),
            track_selection_set: Default::default(),
            input_channel_pair: Default::default(),
            event_channel_pair: Default::default(),
        };
        r.start_thread();
        r
    }
}
impl OrchestratorPanel {
    fn start_thread(&mut self) {
        let receiver = self.input_channel_pair.receiver.clone();
        let sender = self.event_channel_pair.sender.clone();
        self.introduce();
        let orchestrator = Arc::clone(&self.orchestrator);
        let track_selection_set = Arc::clone(&self.track_selection_set);
        std::thread::spawn(move || loop {
            let recv = receiver.recv();
            if let Ok(mut o) = orchestrator.lock() {
                match recv {
                    Ok(input) => match input {
                        OrchestratorInput::Midi(channel, message) => {
                            Self::handle_input_midi(&mut o, channel, message);
                        }
                        OrchestratorInput::ProjectPlay => o.play(),
                        OrchestratorInput::ProjectStop => o.stop(),
                        OrchestratorInput::ProjectNew => {
                            let mut mo = Orchestrator::default();
                            o.prepare_successor(&mut mo);
                            let _ = mo.create_starter_tracks(); // TODO: DRY this
                            *o = mo;
                            let _ = sender.send(OrchestratorEvent::New);
                        }
                        OrchestratorInput::ProjectOpen(path) => {
                            match Self::handle_input_load(&path) {
                                Ok(mut mo) => {
                                    o.prepare_successor(&mut mo);
                                    *o = mo;
                                    let _ = sender
                                        .send(OrchestratorEvent::Loaded(path, o.title().cloned()));
                                }
                                Err(err) => {
                                    let _ = sender.send(OrchestratorEvent::LoadError(path, err));
                                }
                            }
                            {}
                        }
                        OrchestratorInput::ProjectSave(path) => {
                            match Self::handle_input_save(&o, &path) {
                                Ok(_) => {
                                    let _ = sender.send(OrchestratorEvent::Saved(path));
                                }
                                Err(err) => {
                                    let _ = sender.send(OrchestratorEvent::SaveError(path, err));
                                }
                            }
                        }
                        OrchestratorInput::Quit => {
                            let _ = sender.send(OrchestratorEvent::Quit);
                            break;
                        }
                        OrchestratorInput::TrackNewMidi => {
                            let _ = o.new_midi_track();
                        }
                        OrchestratorInput::TrackNewAudio => {
                            let _ = o.new_audio_track();
                        }
                        OrchestratorInput::TrackNewAux => {
                            let _ = o.new_aux_track();
                        }
                        OrchestratorInput::TrackDeleteSelected => {
                            if let Ok(track_selection_set) = track_selection_set.lock() {
                                let uids = Vec::from_iter(track_selection_set.iter().copied());
                                o.delete_tracks(&uids);
                            }
                        }
                        OrchestratorInput::TrackDuplicateSelected => {
                            todo!("duplicate selected tracks");
                        }
                        OrchestratorInput::TrackPatternRemoveSelected => {
                            unimplemented!()
                            //                            o.remove_selected_patterns();
                        }
                        OrchestratorInput::TrackAddEntity(key) => {
                            // TODO: this is weird because it acts on all selected tracks. Figure out how to better restrict in the GUI.
                            if let Ok(track_selection_set) = track_selection_set.lock() {
                                track_selection_set.iter().for_each(|track_uid| {
                                    if let Some(track) = o.get_track_mut(track_uid) {
                                        if let Some(e) = EntityFactory::global().new_entity(&key) {
                                            let _ = track.append_entity(e);
                                        }
                                    }
                                });
                            }
                        }
                        OrchestratorInput::Tempo(tempo) => {
                            o.update_tempo(tempo);
                            let _ = sender.send(OrchestratorEvent::Tempo(tempo));
                        }
                    },
                    Err(err) => {
                        eprintln!("unexpected failure of OrchestratorInput channel: {:?}", err);
                        break;
                    }
                }
            }
        });
    }

    // Send any important initial messages after creation.
    fn introduce(&self) {
        if let Ok(o) = self.orchestrator.lock() {
            self.broadcast_tempo(o.tempo());
        }
    }

    fn broadcast_tempo(&self, tempo: Tempo) {
        self.broadcast(OrchestratorEvent::Tempo(tempo));
    }

    fn broadcast(&self, event: OrchestratorEvent) {
        let _ = self.event_channel_pair.sender.send(event);
    }

    /// The sending side of the [OrchestratorInput] channel.
    pub fn sender(&self) -> &Sender<OrchestratorInput> {
        &self.input_channel_pair.sender
    }

    /// The receiving side of the [OrchestratorEvent] channel.
    pub fn receiver(&self) -> &Receiver<OrchestratorEvent> {
        &self.event_channel_pair.receiver
    }

    /// The [Orchestrator] contained in this panel.
    pub fn orchestrator(&self) -> &Arc<Mutex<Orchestrator>> {
        &self.orchestrator
    }

    fn handle_input_midi(
        o: &mut MutexGuard<Orchestrator>,
        channel: MidiChannel,
        message: MidiMessage,
    ) {
        o.handle_midi_message(channel, message, &mut |_, _| {});
    }

    fn handle_input_load(path: &PathBuf) -> Result<Orchestrator> {
        match std::fs::read_to_string(path) {
            Ok(project_string) => match serde_json::from_str::<Orchestrator>(&project_string) {
                Ok(mut mo) => {
                    mo.after_deser();
                    anyhow::Ok(mo)
                }
                Err(err) => Err(anyhow!("Error while parsing: {}", err)),
            },
            Err(err) => Err(anyhow!("Error while reading: {}", err)),
        }
    }

    fn handle_input_save(o: &MutexGuard<Orchestrator>, path: &PathBuf) -> Result<()> {
        let o: &Orchestrator = o;
        match serde_json::to_string_pretty(o)
            .map_err(|_| anyhow::format_err!("Unable to serialize prefs JSON"))
        {
            Ok(json) => match std::fs::write(path, json) {
                Ok(_) => Ok(()),
                Err(err) => Err(anyhow!("While writing project: {}", err)),
            },
            Err(err) => Err(anyhow!("While serializing project: {}", err)),
        }
    }

    /// Sends the given [OrchestratorInput] to the [Orchestrator].
    pub fn send_to_service(&self, input: OrchestratorInput) {
        match self.sender().send(input) {
            Ok(_) => {}
            Err(err) => eprintln!("sending OrchestratorInput failed with {:?}", err),
        }
    }

    /// Requests that the [Orchestrator] prepare to exit.
    pub fn exit(&self) {
        eprintln!("OrchestratorInput::Quit");
        self.send_to_service(OrchestratorInput::Quit);
    }

    /// Whether one or more tracks are currently selected.
    pub fn is_any_track_selected(&self) -> bool {
        if let Ok(tss) = self.track_selection_set.lock() {
            !tss.is_empty()
        } else {
            false
        }
    }

    /// Renders the panel.
    pub fn show(&mut self, ui: &mut Ui, is_control_only_down: bool) {
        let mut o = self.orchestrator.lock().unwrap();
        o.set_track_selection_set(self.track_selection_set.lock().unwrap().clone());
        o.ui(ui);
        if let Some(action) = o.action() {
            match action {
                OrchestratorAction::ClickTrack(track_uid) => {
                    self.track_selection_set
                        .lock()
                        .unwrap()
                        .click(&track_uid, is_control_only_down);
                }
                OrchestratorAction::DoubleClickTrack(track_uid) => {
                    o.toggle_track_ui_state(&track_uid);
                }
                OrchestratorAction::NewDeviceForTrack(track_uid, key) => {
                    if let Some(track) = o.get_track_mut(&track_uid) {
                        if let Some(entity) = EntityFactory::global().new_entity(&key) {
                            let _ = track.append_entity(entity);
                        }
                    }
                }
            }
        }
    }

    /// Lets the [EntityFactory] know of the highest [Uid] that the current
    /// Orchestrator has seen, so that it won't generate duplicates.
    pub fn update_entity_factory_uid(&self) {
        let uid = self
            .orchestrator
            .lock()
            .unwrap()
            .calculate_max_entity_uid()
            .0;
        EntityFactory::global().set_next_uid(uid + 1);
    }
}
