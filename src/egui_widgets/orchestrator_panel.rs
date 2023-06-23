use crate::mini::{ChannelPair, DragDropManager, EntityFactory, MiniOrchestrator, TrackIndex};
use anyhow::{anyhow, Result};
use crossbeam_channel::{Receiver, Sender};
use eframe::egui::Ui;
use groove_core::{
    midi::{MidiChannel, MidiMessage},
    time::Tempo,
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

    // TODO: these are waiting for the big refactor (which might never happen)
    /// Select the given track.
    #[allow(dead_code)]
    TrackSelect(TrackIndex, bool), // (index, add to selection set)
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
pub struct OrchestratorPanel {
    #[allow(dead_code)]
    factory: Arc<EntityFactory>,
    #[allow(dead_code)]
    drag_drop_manager: Arc<Mutex<DragDropManager>>,
    orchestrator: Arc<Mutex<MiniOrchestrator>>,
    input_channel_pair: ChannelPair<MiniOrchestratorInput>,
    event_channel_pair: ChannelPair<MiniOrchestratorEvent>,
}
impl OrchestratorPanel {
    /// Creates a new panel.
    pub fn new_with(
        factory: Arc<EntityFactory>,
        drag_drop_manager: Arc<Mutex<DragDropManager>>,
    ) -> Self {
        let mut r = Self {
            factory: Arc::clone(&factory),
            drag_drop_manager,
            orchestrator: Arc::new(Mutex::new(MiniOrchestrator::new_with(factory))),
            input_channel_pair: Default::default(),
            event_channel_pair: Default::default(),
        };
        r.start_thread();
        r
    }

    fn start_thread(&mut self) {
        let receiver = self.input_channel_pair.receiver.clone();
        let sender = self.event_channel_pair.sender.clone();
        self.introduce();
        let orchestrator = Arc::clone(&self.orchestrator);
        let entity_factory = Arc::clone(&self.factory);
        std::thread::spawn(move || loop {
            let recv = receiver.recv();
            if let Ok(mut o) = orchestrator.lock() {
                match recv {
                    Ok(input) => match input {
                        MiniOrchestratorInput::Midi(channel, message) => {
                            Self::handle_input_midi(&mut o, channel, message);
                        }
                        MiniOrchestratorInput::ProjectPlay => eprintln!("Play"),
                        MiniOrchestratorInput::ProjectStop => eprintln!("Stop"),
                        MiniOrchestratorInput::ProjectNew => {
                            let mut mo = MiniOrchestrator::new_with(Arc::clone(&entity_factory));
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
                            o.new_midi_track();
                        }
                        MiniOrchestratorInput::TrackNewAudio => {
                            o.new_audio_track();
                        }
                        MiniOrchestratorInput::TrackDeleteSelected => {
                            o.delete_selected_tracks();
                        }
                        MiniOrchestratorInput::TrackDuplicateSelected => {
                            todo!("duplicate selected tracks");
                        }
                        MiniOrchestratorInput::TrackNewSend => {
                            o.new_send_track();
                        }
                        MiniOrchestratorInput::TrackPatternRemoveSelected => {
                            o.remove_selected_patterns();
                        }
                        MiniOrchestratorInput::TrackSelect(index, add_to_selection_set) => {
                            o.select_track(index, add_to_selection_set);
                        }
                        MiniOrchestratorInput::TrackSelectReset => todo!(),
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
        o.handle_midi(channel, message);
    }

    fn handle_input_load(path: &PathBuf) -> Result<MiniOrchestrator> {
        match std::fs::read_to_string(path) {
            Ok(project_string) => match serde_json::from_str(&project_string) {
                Ok(mo) => {
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
        if let Ok(o) = self.orchestrator.lock() {
            o.is_any_track_selected()
        } else {
            false
        }
    }

    /// Renders the panel.
    pub fn show(&mut self, ui: &mut Ui, is_control_only_down: bool) {
        if let Ok(mut o) = self.orchestrator.lock() {
            o.update_track_selection_tracking();
            o.show_with(ui, is_control_only_down);
        }
    }
}
