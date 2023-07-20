use crate::mini::{
    ChannelPair, EntityFactory, Key, MiniOrchestrator, SelectionSet, TrackAction, TrackTitle,
    TrackUid,
};
use anyhow::{anyhow, Result};
use crossbeam_channel::{Receiver, Sender};
use eframe::egui::Ui;
use groove_core::{
    midi::{MidiChannel, MidiMessage},
    time::Tempo,
    traits::{Configurable, HandlesMidi, Performs, Serializable},
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
};

/// Commands that [MiniOrchestrator] accepts.
#[derive(Clone, Debug)]
pub enum MiniOrchestratorInput {
    /// An external MIDI message arrived.
    Midi(MidiChannel, MidiMessage),
    /// Open the project file at the given path, load it, and replace the
    /// current [MiniOrchestrator] instance with it.
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
    /// Create a new MIDI track.
    TrackNewMidi,
    /// Create a new send track.
    TrackNewSend,
    /// Delete the selected arranged patterns.
    TrackPatternRemoveSelected,
    /// Add a new entity to the selected track.
    TrackAddThing(Key),
    /// Sets a new title for the track.
    TrackSetTitle(TrackUid, TrackTitle),
    /// Sets the tempo.
    Tempo(Tempo),

    // TODO: these are waiting for the big refactor (which might never happen)
    /// Select the given track.
    #[allow(dead_code)]
    TrackSelect(TrackUid, bool), // (track UID, add to selection set)
    /// Reset the selection set.
    #[allow(dead_code)]
    TrackSelectReset,

    /// Quit the thread.
    Quit,
}

/// Events that [MiniOrchestrator] generates.
#[derive(Debug)]
pub enum MiniOrchestratorEvent {
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

/// An egui panel that renders a [MiniOrchestrator].
#[derive(Debug, Default)]
pub struct OrchestratorPanel {
    orchestrator: Arc<Mutex<MiniOrchestrator>>,
    factory: Arc<EntityFactory>,
    track_selection_set: Arc<Mutex<SelectionSet<TrackUid>>>,
    input_channel_pair: ChannelPair<MiniOrchestratorInput>,
    event_channel_pair: ChannelPair<MiniOrchestratorEvent>,
}
impl OrchestratorPanel {
    /// Creates a new panel.
    pub fn new_with(factory: Arc<EntityFactory>) -> Self {
        let mut o = MiniOrchestrator::default();
        let _ = o.create_starter_tracks();
        let mut r = Self {
            orchestrator: Arc::new(Mutex::new(o)),
            factory,
            ..Default::default()
        };
        r.start_thread();
        r
    }

    fn start_thread(&mut self) {
        let receiver = self.input_channel_pair.receiver.clone();
        let sender = self.event_channel_pair.sender.clone();
        self.introduce();
        let orchestrator = Arc::clone(&self.orchestrator);
        let factory = Arc::clone(&self.factory);
        let track_selection_set = Arc::clone(&self.track_selection_set);
        std::thread::spawn(move || loop {
            let recv = receiver.recv();
            if let Ok(mut o) = orchestrator.lock() {
                match recv {
                    Ok(input) => match input {
                        MiniOrchestratorInput::Midi(channel, message) => {
                            Self::handle_input_midi(&mut o, channel, message);
                        }
                        MiniOrchestratorInput::ProjectPlay => o.play(),
                        MiniOrchestratorInput::ProjectStop => o.stop(),
                        MiniOrchestratorInput::ProjectNew => {
                            let mut mo = MiniOrchestrator::default();
                            o.prepare_successor(&mut mo);
                            *o = mo;
                            let _ = sender.send(MiniOrchestratorEvent::New);
                        }
                        MiniOrchestratorInput::ProjectOpen(path) => {
                            match Self::handle_input_load(&path) {
                                Ok(mut mo) => {
                                    o.prepare_successor(&mut mo);
                                    *o = mo;
                                    let _ = sender.send(MiniOrchestratorEvent::Loaded(
                                        path,
                                        o.title().cloned(),
                                    ));
                                }
                                Err(err) => {
                                    let _ =
                                        sender.send(MiniOrchestratorEvent::LoadError(path, err));
                                }
                            }
                            {}
                        }
                        MiniOrchestratorInput::ProjectSave(path) => {
                            match Self::handle_input_save(&o, &path) {
                                Ok(_) => {
                                    let _ = sender.send(MiniOrchestratorEvent::Saved(path));
                                }
                                Err(err) => {
                                    let _ =
                                        sender.send(MiniOrchestratorEvent::SaveError(path, err));
                                }
                            }
                        }
                        MiniOrchestratorInput::Quit => {
                            let _ = sender.send(MiniOrchestratorEvent::Quit);
                            break;
                        }
                        MiniOrchestratorInput::TrackNewMidi => {
                            let _ = o.new_midi_track();
                        }
                        MiniOrchestratorInput::TrackNewAudio => {
                            let _ = o.new_audio_track();
                        }
                        MiniOrchestratorInput::TrackNewSend => {
                            let _ = o.new_send_track();
                        }
                        MiniOrchestratorInput::TrackDeleteSelected => {
                            if let Ok(track_selection_set) = track_selection_set.lock() {
                                track_selection_set
                                    .iter()
                                    .for_each(|uid| o.delete_track(uid));
                            }
                        }
                        MiniOrchestratorInput::TrackDuplicateSelected => {
                            todo!("duplicate selected tracks");
                        }
                        MiniOrchestratorInput::TrackPatternRemoveSelected => {
                            unimplemented!()
                            //                            o.remove_selected_patterns();
                        }
                        MiniOrchestratorInput::TrackSelectReset => todo!(),
                        MiniOrchestratorInput::TrackAddThing(key) => {
                            // TODO: this is weird because it acts on all selected tracks. Figure out how to better restrict in the GUI.
                            if let Ok(track_selection_set) = track_selection_set.lock() {
                                track_selection_set.iter().for_each(|track_uid| {
                                    if let Some(e) = factory.new_thing(&key) {
                                        let _ = o.add_thing(e, track_uid);
                                    }
                                });
                            }
                        }
                        MiniOrchestratorInput::Tempo(tempo) => {
                            o.update_tempo(tempo);
                            let _ = sender.send(MiniOrchestratorEvent::Tempo(tempo));
                        }
                        MiniOrchestratorInput::TrackSetTitle(track_uid, title) => {
                            o.set_track_title(track_uid, title);
                        }
                        MiniOrchestratorInput::TrackSelect(track_uid, add_to_selection_set) => {
                            if let Ok(mut track_selection_set) = track_selection_set.lock() {
                                track_selection_set.click(track_uid, add_to_selection_set);
                            }
                        }
                    },
                    Err(err) => {
                        eprintln!(
                            "unexpected failure of MiniOrchestratorInput channel: {:?}",
                            err
                        );
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
        self.broadcast(MiniOrchestratorEvent::Tempo(tempo));
    }

    fn broadcast(&self, event: MiniOrchestratorEvent) {
        let _ = self.event_channel_pair.sender.send(event);
    }

    /// The sending side of the [MiniOrchestratorInput] channel.
    pub fn sender(&self) -> &Sender<MiniOrchestratorInput> {
        &self.input_channel_pair.sender
    }

    /// The receiving side of the [MiniOrchestratorEvent] channel.
    pub fn receiver(&self) -> &Receiver<MiniOrchestratorEvent> {
        &self.event_channel_pair.receiver
    }

    /// The [MiniOrchestrator] contained in this panel.
    pub fn orchestrator(&self) -> &Arc<Mutex<MiniOrchestrator>> {
        &self.orchestrator
    }

    fn handle_input_midi(
        o: &mut MutexGuard<MiniOrchestrator>,
        channel: MidiChannel,
        message: MidiMessage,
    ) {
        o.handle_midi_message(channel, message, &mut |_, _| {});
    }

    fn handle_input_load(path: &PathBuf) -> Result<MiniOrchestrator> {
        match std::fs::read_to_string(path) {
            Ok(project_string) => match serde_json::from_str::<MiniOrchestrator>(&project_string) {
                Ok(mut mo) => {
                    mo.after_deser();
                    return anyhow::Ok(mo);
                }
                Err(err) => {
                    return Err(anyhow!("Error while parsing: {}", err));
                }
            },
            Err(err) => {
                return Err(anyhow!("Error while reading: {}", err));
            }
        }
    }

    fn handle_input_save(o: &MutexGuard<MiniOrchestrator>, path: &PathBuf) -> Result<()> {
        let o: &MiniOrchestrator = &o;
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

    /// Sends the given [MiniOrchestratorInput] to the [MiniOrchestrator].
    pub fn send_to_service(&self, input: MiniOrchestratorInput) {
        match self.sender().send(input) {
            Ok(_) => {}
            Err(err) => eprintln!("sending MiniOrchestratorInput failed with {:?}", err),
        }
    }

    /// Requests that the [MiniOrchestrator] prepare to exit.
    pub fn exit(&self) {
        eprintln!("MiniOrchestratorInput::Quit");
        self.send_to_service(MiniOrchestratorInput::Quit);
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
        if let Ok(mut o) = self.orchestrator.lock() {
            let tss = if let Ok(tss) = self.track_selection_set.lock() {
                tss.clone()
            } else {
                panic!()
            };
            if let Some(action) = o.show_with(ui, &tss) {
                match action {
                    TrackAction::Click(track_uid) => self.send_to_service(
                        MiniOrchestratorInput::TrackSelect(track_uid, is_control_only_down),
                    ),
                    TrackAction::SetTitle(track_uid, title) => {
                        self.send_to_service(MiniOrchestratorInput::TrackSetTitle(track_uid, title))
                    }
                }
            }
        }
    }

    /// Lets the [EntityFactory] know of the highest [Uid] that the current
    /// Orchestrator has seen, so that it won't generate duplicates.
    pub fn update_entity_factory_uid(&self) {
        self.factory
            .set_next_uid(self.orchestrator.lock().unwrap().max_entity_uid().0 + 1);
    }
}
